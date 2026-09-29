use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use color_eyre::Result;
use color_eyre::eyre::anyhow;
use serde::Deserialize;
use serde::Serialize;
use tabled::builder::Builder;
use tabled::settings::Style;
use xml::EmitterConfig;
use xml::writer::XmlEvent;

#[derive(Serialize, Deserialize)]
struct Config {
    path: String,
    project_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    http_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shutdown_port: Option<u16>,
}

impl Config {
    pub fn parse<P>(path: P) -> Result<Config>
    where
        P: AsRef<Path>,
    {
        Ok(toml::from_str::<Config>(&fs::read_to_string(path)?)?)
    }

    fn http_port(&self) -> u16 {
        self.http_port.unwrap_or(8080)
    }

    fn shutdown_port(&self) -> u16 {
        self.shutdown_port.unwrap_or(8005)
    }
}

pub struct Controller {
    catalina_home: PathBuf,
}

impl Controller {
    pub fn create() -> Result<Self> {
        Ok(Self {
            catalina_home: Controller::get_catalina_home()?,
        })
    }

    pub fn run(&self, config: String, jpda: bool) -> Result<()> {
        let cfg = ConfigFolder::create()?.load_config(config.clone())?;
        let catalina_base = CatalinaBase::init(&config, &self.catalina_home, &cfg)?;
        let mut command = Command::new(self.get_catalina_sh()?);
        command.env("CATALINA_BASE", catalina_base.as_path());
        if jpda {
            command.arg("jpda");
        }
        command.arg("run");
        handle_signals(command.spawn()?)?;
        Ok(())
    }

    pub fn debug(&self, config: String) -> Result<()> {
        let cfg = ConfigFolder::create()?.load_config(config.clone())?;
        let catalina_base = CatalinaBase::init(&config, &self.catalina_home, &cfg)?;
        let child = Command::new(self.get_catalina_sh()?)
            .env("CATALINA_BASE", catalina_base.as_path())
            .arg("debug")
            .spawn()?;
        handle_signals(child)?;
        Ok(())
    }

    pub fn deploy(&self, config: String) -> Result<()> {
        let cfg = ConfigFolder::create()?.load_config(config.clone())?;
        let catalina_base = CatalinaBase::init(&config, &self.catalina_home, &cfg)?;
        let deploy_folder = DeployFolder(catalina_base.deploy_folder());

        let trimmed_path = cfg.path.trim_matches('/');
        let filename = trimmed_path.replace("/", "#");
        let path = "/".to_string() + trimmed_path;
        let mut artifact_path = PathBuf::from_str(&cfg.project_path)?;
        artifact_path.push("target");
        artifact_path.push("*.war");
        let doc_base_buf = glob::glob(
            artifact_path
                .to_str()
                .expect("Path contains invalid unicode"),
        )?
        .next()
        .ok_or(anyhow!(format!(
            "Failed to match the path: \"{}\"",
            artifact_path
                .to_str()
                .expect("Path contains invalid unicode")
        )))??;
        let mut doc_base = PathBuf::new();
        doc_base.push(
            doc_base_buf
                .parent()
                .expect("Couldn't get parent of doc base.")
                .canonicalize()?,
        );
        doc_base.push(
            doc_base_buf
                .file_stem()
                .expect("Couldn't get file stem of doc base."),
        );
        let deploy_file = deploy_folder.create_deploy_file(filename)?;
        let mut writer = EmitterConfig::new()
            .write_document_declaration(false)
            .pad_self_closing(true)
            .create_writer(deploy_file);
        writer.write(
            XmlEvent::start_element("Context")
                .attr("path", &path)
                .attr(
                    "docBase",
                    doc_base.to_str().expect("Path contains invalid unicode"),
                ),
        )?;
        writer.write(XmlEvent::end_element())?;
        Ok(())
    }

    pub fn cleanup(&self, config: String) -> Result<()> {
        let cfg = ConfigFolder::create()?.load_config(config.clone())?;
        let catalina_base = CatalinaBase::init(&config, &self.catalina_home, &cfg)?;
        let deploy_folder = DeployFolder(catalina_base.deploy_folder());

        let filename = cfg.path.trim_matches('/').replace("/", "#") + ".xml";
        let removed_files = deploy_folder
            .read_dir()?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                if *entry.file_name() != *filename {
                    std::fs::remove_file(entry.path()).ok()?;
                    Some(entry.path().file_stem()?.to_str()?.to_owned())
                } else {
                    None
                }
            })
            .collect::<HashSet<String>>();

        let work_folder = catalina_base.work_folder();
        if work_folder.exists() {
            work_folder
                .read_dir()?
                .filter_map(|entry| entry.ok())
                .filter(|entry| {
                    removed_files.contains(
                        entry
                            .path()
                            .file_stem()
                            .expect("Couldn't extract file stem")
                            .to_str()
                            .expect("Path contains invalid unicode"),
                    )
                })
                .for_each(|entry| _ = std::fs::remove_dir_all(entry.path()));
        }
        Ok(())
    }

    pub fn add_config(
        &self,
        name: String,
        path: String,
        project_path: String,
        http_port: Option<u16>,
        shutdown_port: Option<u16>,
    ) -> Result<()> {
        let config_folder = ConfigFolder::create()?;
        let config = Config {
            path,
            project_path,
            http_port,
            shutdown_port,
        };
        config_folder.add_config(name.clone(), &config)?;
        println!("Successfully added config file {name}.toml");
        Ok(())
    }

    pub fn remove_config(&self, name: String) -> Result<()> {
        let config_folder = ConfigFolder::create()?;
        config_folder.remove_config(name.clone())?;
        let catalina_base_path = CatalinaBase::path_for(&name)?;
        if catalina_base_path.exists() {
            fs::remove_dir_all(&catalina_base_path)?;
        }
        println!("Successfully removed config file {name}.toml");
        Ok(())
    }

    pub fn list_configs(&self) -> Result<()> {
        let config_folder = ConfigFolder::create()?;
        let mut builder = Builder::default();
        builder.push_record(vec!["Name", "Path", "Project Path", "HTTP Port", "Shutdown Port"]);
        config_folder
            .get_file_paths()
            .iter()
            .filter_map(|path| {
                Some((
                    path.file_name()?.to_str()?,
                    Config::parse(path)
                        .inspect_err(|err| println!("{err}"))
                        .ok()?,
                ))
            })
            .for_each(|entry| {
                let name = entry.0;
                let config = entry.1;
                let http_port = config.http_port().to_string();
                let shutdown_port = config.shutdown_port().to_string();
                builder.push_record(vec![
                    name,
                    &config.path,
                    &config.project_path,
                    &http_port,
                    &shutdown_port,
                ]);
            });
        println!("{}", builder.build().with(Style::rounded()));
        Ok(())
    }

    fn get_catalina_sh(&self) -> Result<String> {
        let catalina_home_sh = self.catalina_home.join("bin").join("catalina.sh");
        if catalina_home_sh.exists() {
            return Ok(catalina_home_sh
                .to_str()
                .expect("Path contains invalid unicode")
                .to_string());
        }
        let which = Command::new("which").arg("catalina.sh").output()?;
        Ok(String::from_utf8(which.stdout)
            .expect("Failed to convert catalina.sh path into valid utf8 string")
            .trim()
            .to_string())
    }

    fn get_catalina_home() -> Result<PathBuf> {
        if let Ok(catalina_home) = std::env::var("CATALINA_HOME") {
            return Ok(PathBuf::from_str(&catalina_home).expect("Path contains invalid unicode"));
        } else if let Ok(catalina_sh) = Command::new("which").arg("catalina.sh").output() {
            let catalina_sh = String::from_utf8(catalina_sh.stdout)
                .expect("Failed to convert catalina.sh path into valid utf8 string");
            let mut catalina_home =
                PathBuf::from_str(&catalina_sh).expect("Path contains invalid unicode");
            catalina_home.pop();
            catalina_home.pop();
            return Ok(catalina_home);
        }
        Err(anyhow!(
            "Couldn't find Tomcat installation or CATALINA_HOME pointing to one"
        ))
    }
}

fn handle_signals(mut child: Child) -> Result<()> {
    let pid = child.id();
    let stopping = Arc::new(AtomicBool::new(false));
    let stopping_clone = stopping.clone();
    // Handles SIGINT, SIGTERM and SIGHUP: the first one asks Tomcat to shut down gracefully,
    // a second one kills it
    ctrlc::set_handler(move || {
        let force = stopping_clone.swap(true, Ordering::SeqCst);
        terminate(pid, force);
    })?;
    let status = child.wait()?;
    if !status.success() && !stopping.load(Ordering::SeqCst) {
        return Err(anyhow!("Tomcat exited with {status}"));
    }
    Ok(())
}

#[cfg(unix)]
fn terminate(pid: u32, force: bool) {
    let signal = if force { libc::SIGKILL } else { libc::SIGTERM };
    // SAFETY: kill only sends a signal to the given pid and has no memory safety requirements
    unsafe {
        libc::kill(pid as libc::pid_t, signal);
    }
}

#[cfg(not(unix))]
fn terminate(_pid: u32, _force: bool) {}

struct CatalinaBase(PathBuf);

impl CatalinaBase {
    fn path_for(profile_name: &str) -> Result<PathBuf> {
        let mut path = PathBuf::from(std::env::var("HOME")?);
        path.push(".local");
        path.push("share");
        path.push("tomcatctl");
        path.push(profile_name);
        Ok(path)
    }

    fn init(profile_name: &str, catalina_home: &Path, config: &Config) -> Result<Self> {
        let path = Self::path_for(profile_name)?;
        let conf_dir = path.join("conf");

        // Conf files and compiled JSPs are specific to a Tomcat version, so refresh them when the
        // Tomcat installation changed. Bases without a marker predate it and are refreshed once.
        let home_marker = conf_dir.join(".catalina_home");
        let current_home = catalina_home
            .canonicalize()
            .unwrap_or_else(|_| catalina_home.to_path_buf());
        let home_changed = conf_dir.exists()
            && fs::read_to_string(&home_marker).ok().map(PathBuf::from) != Some(current_home.clone());
        if home_changed {
            println!(
                "Tomcat installation changed to {}, refreshing conf and work folders of {profile_name}",
                current_home.display()
            );
            let work_dir = path.join("work");
            if work_dir.exists() {
                fs::remove_dir_all(work_dir)?;
            }
        }

        fs::create_dir_all(conf_dir.join("Catalina").join("localhost"))?;
        fs::create_dir_all(path.join("logs"))?;
        fs::create_dir_all(path.join("temp"))?;
        fs::create_dir_all(path.join("work"))?;
        fs::create_dir_all(path.join("webapps"))?;

        // Copy conf files from CATALINA_HOME on first init or after a Tomcat change;
        // skip server.xml since we manage it
        let home_conf = catalina_home.join("conf");
        if home_conf.exists() {
            for entry in fs::read_dir(&home_conf)?.flatten() {
                if entry.file_name() != "server.xml" && entry.path().is_file() {
                    let dest = conf_dir.join(entry.file_name());
                    if home_changed || !dest.exists() {
                        fs::copy(entry.path(), dest)?;
                    }
                }
            }
        }
        fs::write(&home_marker, current_home.to_string_lossy().as_bytes())?;

        // Always (re)write server.xml so port changes in the config take effect
        fs::write(conf_dir.join("server.xml"), Self::server_xml(config))?;

        Ok(Self(path))
    }

    fn deploy_folder(&self) -> PathBuf {
        self.0.join("conf").join("Catalina").join("localhost")
    }

    fn work_folder(&self) -> PathBuf {
        self.0.join("work").join("Catalina").join("localhost")
    }

    fn as_path(&self) -> &Path {
        &self.0
    }

    fn server_xml(config: &Config) -> String {
        let http_port = config.http_port();
        let shutdown_port = config.shutdown_port();
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Server port="{shutdown_port}" shutdown="SHUTDOWN">
  <Listener className="org.apache.catalina.startup.VersionLoggerListener" />
  <Listener className="org.apache.catalina.core.AprLifecycleListener" SSLEngine="on" />
  <Listener className="org.apache.catalina.core.JreMemoryLeakPreventionListener" />
  <Listener className="org.apache.catalina.mbeans.GlobalResourcesLifecycleListener" />
  <Listener className="org.apache.catalina.core.ThreadLocalLeakPreventionListener" />
  <GlobalNamingResources>
    <Resource name="UserDatabase" auth="Container"
              type="org.apache.catalina.UserDatabase"
              description="User database that can be updated and saved"
              factory="org.apache.catalina.users.MemoryUserDatabaseFactory"
              pathname="conf/tomcat-users.xml" />
  </GlobalNamingResources>
  <Service name="Catalina">
    <Connector port="{http_port}" protocol="HTTP/1.1"
               connectionTimeout="20000"
               redirectPort="8443"
               maxParameterCount="1000" />
    <Engine name="Catalina" defaultHost="localhost">
      <Realm className="org.apache.catalina.realm.LockOutRealm">
        <Realm className="org.apache.catalina.realm.UserDatabaseRealm"
               resource="UserDatabase"/>
      </Realm>
      <Host name="localhost" appBase="webapps"
            unpackWARs="true" autoDeploy="false">
        <Valve className="org.apache.catalina.valves.AccessLogValve" directory="logs"
               prefix="localhost_access_log" suffix=".txt"
               pattern="%h %l %u %t &quot;%r&quot; %s %b" />
      </Host>
    </Engine>
  </Service>
</Server>"#
        )
    }
}

struct ConfigFolder(PathBuf);

impl Deref for ConfigFolder {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ConfigFolder {
    pub fn create() -> Result<Self> {
        let mut config_folder = PathBuf::from(std::env::var("HOME")?);
        config_folder.push(".config");
        config_folder.push("tomcatctl");
        if !config_folder.exists() {
            fs::create_dir_all(&config_folder)?
        }
        Ok(Self(config_folder))
    }

    pub fn add_config(&self, name: String, config: &Config) -> Result<()> {
        let mut path = self.0.clone();
        path.push(name + ".toml");
        let mut file = File::create_new(path)?;
        let content = toml::to_string_pretty(&config)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    pub fn remove_config(&self, name: String) -> Result<()> {
        let mut path = self.0.clone();
        path.push(name + ".toml");
        fs::remove_file(path)?;
        Ok(())
    }

    pub fn load_config(&self, config: String) -> Result<Config> {
        let mut path = self.0.clone();
        path.push(config + ".toml");
        Ok(toml::from_str::<Config>(&fs::read_to_string(path)?)?)
    }

    pub fn get_file_paths(&self) -> Vec<PathBuf> {
        if let Ok(dir) = fs::read_dir(&self.0) {
            dir.filter_map(|x| {
                if let Ok(entry) = x {
                    Some(entry.path())
                } else {
                    None
                }
            })
            .collect()
        } else {
            vec![]
        }
    }
}

struct DeployFolder(PathBuf);

impl Deref for DeployFolder {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DeployFolder {
    pub fn create_deploy_file(self, filename: String) -> Result<File> {
        let mut deploy_file = self.0.clone();
        deploy_file.push(filename + ".xml");
        Ok(File::create(deploy_file)?)
    }
}
