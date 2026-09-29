# tomcatctl

`tomcatctl` is a tool for controlling Apache Tomcat from the command line and automating deployment of Java apps.
Instead of writing `catalina.sh` commands and XML by hand, you create named configs once and start them with a single command.
Every config gets its own Tomcat instance with its own ports, so you can run several apps side by side (e.g. a Magnolia CMS author and public instance).

`tomcatctl` currently supports macOS and Linux.

## Installation

```sh
cargo install --locked tomcatctl
```

Prebuilt binaries are available on the [releases page](https://github.com/arseeeeN/tomcatctl/releases).

`tomcatctl` needs an existing Tomcat installation. It uses `$CATALINA_HOME` if it is set, otherwise the Tomcat that contains `catalina.sh` on your `PATH`.

## How to use

First you need to create a config for the deployment you want to create.

```sh
# tomcatctl config add <config-name> <deployment-path> <project-path> [--http-port <port>] [--shutdown-port <port>] [--jpda-port <port>]
#   - config-name: The name of the config, has no impact on the deployment
#   - deployment-path: The context path under which the project will be deployed in Tomcat
#   - project-path: The location of your project, the folder that contains target/*.war. Supports glob paths
#     and is resolved relative to the directory you run tomcatctl in.
#   - --http-port: HTTP connector port (default: 8080)
#   - --shutdown-port: Tomcat shutdown port (default: 8005)
#   - --jpda-port: JPDA debug port, used with `run --jpda` (default: 8000)
tomcatctl config add magnolia /dev './*-webapp'
```

After that you can deploy your built Java project.

```sh
# tomcatctl run <config-name>
tomcatctl run magnolia
```

The project is deployed in place from the exploded directory next to the `*.war` file, nothing is copied.
Press Ctrl+C to shut Tomcat down gracefully, press it again to kill it.

### Commands

| Command | Description |
|---|---|
| `tomcatctl run <config> [--jpda]` | Deploy and start Tomcat in the foreground, optionally with the JPDA debug endpoint |
| `tomcatctl debug <config>` | Deploy and start Tomcat in the built-in debugger (`jdb`) |
| `tomcatctl deploy <config>` | Deploy without starting Tomcat |
| `tomcatctl config add <name> <path> <project-path>` | Add a config |
| `tomcatctl config set <name> <property> <value>` | Change a property of a config (alias: `edit`) |
| `tomcatctl config list` | List all configs |
| `tomcatctl config remove <name>` | Remove a config and its Tomcat instance |

### Running multiple instances

Give each config its own ports and start them in separate terminals:

```sh
tomcatctl config add author /author './*-webapp'
tomcatctl config add public /public './*-webapp' --http-port 8081 --shutdown-port 8006 --jpda-port 8001

tomcatctl run author --jpda
tomcatctl run public --jpda
```

The JPDA debug port only listens on `localhost`. To use a different address for a single run, set the `JPDA_ADDRESS` environment variable, which takes precedence over the config.
All other environment variables are passed on to `catalina.sh` as well, e.g. `CATALINA_OPTS` for JVM options.

### Changing a config

Use `config set` with one of the properties `path`, `project-path`, `http-port`, `shutdown-port` or `jpda-port`:

```sh
tomcatctl config set public jpda-port 8002
```

You can also edit the TOML file in `~/.config/tomcatctl/` directly. Changes take effect on the next `run`.

### Where things are stored

- Configs: `~/.config/tomcatctl/<name>.toml`
- Tomcat instance (`CATALINA_BASE`) per config: `~/.local/share/tomcatctl/<name>/`, including its logs

## Why does this exist?

I wanted to have a way of cleanly deploying Java projects to Tomcat without having to rely on tools like IntelliJ to do it for me.
I couldn't find a single tool that does exactly what I want, so I built one myself.
This can also potentially be used in pipelines and containers to more cleanly deploy tomcat projects.
