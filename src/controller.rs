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
use std::sync::Arc;
use std::sync::Mutex;

use color_eyre::eyre::anyhow;
use color_eyre::Result;
use serde::Deserialize;
use serde::Serialize;
use tabled::builder::Builder;
use tabled::settings::Style;
use xml::writer::XmlEvent;
use xml::EmitterConfig;

#[derive(Serialize, Deserialize)]
struct Config {
    path: String,
    project_path: String,
}

impl Config {
    pub fn parse<P>(path: P) -> Result<Config>
    where
        P: AsRef<Path>,
    {
        Ok(toml::from_str::<Config>(&fs::read_to_string(path)?)?)
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

    pub fn run(&self, jpda: bool) -> Result<()> {
        let mut command = Command::new(self.get_catalina_sh()?);
        if jpda {
            command.arg("jpda");
        }
        command.arg("run");
        let child = command.spawn()?;
        handle_signals(child)?;
        Ok(())
    }

    pub fn debug(&self) -> Result<()> {
        let child = Command::new(self.get_catalina_sh()?).arg("debug").spawn()?;
        handle_signals(child)?;
        Ok(())
    }

    pub fn deploy(&self, config: String) -> Result<()> {
        let deploy_folder = DeployFolder::create(&self.catalina_home)?;
        if deploy_folder.exists() {
            let config_folder = ConfigFolder::create()?;
            let config = config_folder.load_config(config)?;
            let trimmed_path = config.path.trim_matches('/');
            let filename = trimmed_path.replace("/", "#");
            let path = "/".to_string() + trimmed_path;
            let mut artifact_path = PathBuf::from_str(&config.project_path)?;
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
            writer.write(XmlEvent::start_element("Context").attr("path", &path).attr(
                "docBase",
                doc_base.to_str().expect("Path contains invalid unicode"),
            ))?;
            writer.write(XmlEvent::end_element())?;
        } else {
            return Err(anyhow!(format!(
                "Can't find Catalina config folder at path: \"{}\"",
                deploy_folder
                    .to_str()
                    .expect("Path contains invalid unicode")
            )));
        }
        Ok(())
    }
    pub fn cleanup(&self, config: String) -> Result<()> {
        let deploy_folder = DeployFolder::create(&self.catalina_home)?;
        if deploy_folder.exists() {
            let config_folder = ConfigFolder::create()?;
            let config = config_folder.load_config(config)?;
            let filename = config.path.trim_matches('/').replace("/", "#") + ".xml";
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
            let mut work_folder = self.catalina_home.to_owned();
            work_folder.push("work");
            work_folder.push("Catalina");
            work_folder.push("localhost");
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
        }

        Ok(())
    }

    pub fn add_config(&self, name: String, path: String, project_path: String) -> Result<()> {
        let config_folder = ConfigFolder::create()?;
        let config = Config { path, project_path };
        config_folder.add_config(name.clone(), &config)?;
        println!("Successfully added config file {name}.toml");
        Ok(())
    }

    pub fn remove_config(&self, name: String) -> Result<()> {
        let config_folder = ConfigFolder::create()?;
        config_folder.remove_config(name.clone())?;
        println!("Successfully removed config file {name}.toml");
        Ok(())
    }

    pub fn list_configs(&self) -> Result<()> {
        let config_folder = ConfigFolder::create()?;
        let mut builder = Builder::default();
        builder.push_record(vec!["Name", "Path", "Project Path"]);
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
            .for_each(|config| {
                let name = config.0;
                let config = config.1;
                builder.push_record(vec![name, &config.path, &config.project_path])
            });
        println!("{}", builder.build().with(Style::rounded()));
        Ok(())
    }

    fn get_catalina_sh(&self) -> Result<String> {
        if let Ok(catalina_sh) = Command::new("which").arg("catalina.sh").output() {
            Ok(String::from_utf8(catalina_sh.stdout)
                .expect("Failed to convert catalina.sh path into valid utf8 string")
                .trim()
                .to_string())
        } else {
            let mut catalina_home = self.catalina_home.clone();
            catalina_home.push("bin");
            catalina_home.push("catalina.sh");
            Ok(catalina_home
                .to_str()
                .expect("Path contains invalid unicode")
                .to_string())
        }
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

fn handle_signals(child: Child) -> Result<()> {
    let child = Arc::new(Mutex::new(child));
    let child_clone = child.clone();
    ctrlc::set_handler(move || {
        let mut child = child_clone
            .lock()
            .expect("Failed to lock mutex while trying to shutdown child process");
        child.kill().expect("Failed to shutdown child process");
    })?;
    let mut child = child
        .lock()
        .expect("Failed to lock mutex while trying to shutdown child process");
    child.wait()?;
    Ok(())
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
    pub fn create(catalina_home: &Path) -> Result<Self> {
        let mut deploy_folder = catalina_home.to_owned();
        deploy_folder.push("conf");
        deploy_folder.push("Catalina");
        deploy_folder.push("localhost");
        if !deploy_folder.exists() {
            fs::create_dir_all(&deploy_folder)?
        }
        Ok(Self(deploy_folder))
    }
    pub fn create_deploy_file(self, filename: String) -> Result<File> {
        let mut deploy_file = self.0.clone();
        deploy_file.push(filename + ".xml");
        Ok(File::create(deploy_file)?)
    }
}
