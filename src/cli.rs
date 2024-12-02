use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "tomcatctl")]
#[command(about = "A CLI for interacting with Apache Tomcat", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: MainCommands,
}

#[derive(Debug, Subcommand)]
pub enum MainCommands {
    #[command()]
    Run {
        #[arg(long)]
        jpda: bool,
        config: String,
    },
    #[command(arg_required_else_help = true)]
    Debug { config: String },
    #[command(arg_required_else_help = true)]
    Deploy { config: String },
    #[command(arg_required_else_help = true)]
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommands {
    #[command(arg_required_else_help = true)]
    Add {
        name: String,
        path: String,
        project_path: String,
    },
    #[command(arg_required_else_help = true)]
    Remove {
        name: String,
    },
    List,
}
