use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "tomcatctl")]
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
