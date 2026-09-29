use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "tomcatctl", version)]
#[command(
    about = "A CLI for interacting with Apache Tomcat\nTo get started create a profile using the \"config add\" subcommands"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: MainCommands,
}

#[derive(Debug, Subcommand)]
pub enum MainCommands {
    #[command(
        arg_required_else_help = true,
        about = "Start Tomcat inside this terminal"
    )]
    Run {
        #[arg(long, help = "Start in debug mode and open the JPDA endpoint")]
        jpda: bool,
        config: String,
    },
    #[command(
        arg_required_else_help = true,
        about = "Start Tomcat in the built-in debugger"
    )]
    Debug { config: String },
    #[command(
        arg_required_else_help = true,
        about = "Deploy the specified config without starting Tomcat"
    )]
    Deploy { config: String },
    #[command(
        arg_required_else_help = true,
        about = "Manage your tomcatctl deployment configs"
    )]
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommands {
    #[command(arg_required_else_help = true, about = "Add a deployment config")]
    Add {
        name: String,
        path: String,
        project_path: String,
        #[arg(long, help = "HTTP connector port (default: 8080)")]
        http_port: Option<u16>,
        #[arg(long, help = "Shutdown port (default: 8005)")]
        shutdown_port: Option<u16>,
        #[arg(long, help = "JPDA debug port, used with --jpda (default: 8000)")]
        jpda_port: Option<u16>,
    },
    #[command(
        arg_required_else_help = true,
        about = "Change a property of a deployment config",
        alias = "edit"
    )]
    Set {
        name: String,
        property: ConfigProperty,
        value: String,
    },
    #[command(
        arg_required_else_help = true,
        about = "Remove a deployment config",
        alias = "rm"
    )]
    Remove { name: String },
    #[command(about = "List all valid deployment configs", alias = "ls")]
    List,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ConfigProperty {
    #[value(help = "Context path under which the project is deployed")]
    Path,
    #[value(alias = "project_path", help = "Location of the project with target/*.war")]
    ProjectPath,
    #[value(alias = "http_port", help = "HTTP connector port")]
    HttpPort,
    #[value(alias = "shutdown_port", help = "Shutdown port")]
    ShutdownPort,
    #[value(alias = "jpda_port", help = "JPDA debug port, used with --jpda")]
    JpdaPort,
}
