use clap::Parser;
use color_eyre::owo_colors::OwoColorize;
use color_eyre::Result;

mod cli;
use cli::*;

mod controller;
use controller::*;

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Cli::parse();
    let controller = Controller::create()?;
    if let Err(err) = run_controller(args, controller) {
        println!("{}", err.red());
    }
    Ok(())
}

fn run_controller(args: Cli, controller: Controller) -> Result<()> {
    match args.command {
        MainCommands::Run { jpda, config } => {
            controller.deploy(config)?;
            controller.run(jpda)?;
        }
        MainCommands::Debug { config } => {
            controller.deploy(config)?;
            controller.debug()?;
        }
        MainCommands::Deploy { config } => {
            controller.deploy(config)?;
        }
        MainCommands::Config { command } => match command {
            ConfigCommands::Add {
                name,
                path,
                project_path,
            } => {
                controller.add_config(name, path, project_path)?;
            }
            ConfigCommands::Remove { name } => {
                controller.remove_config(name)?;
            }
            ConfigCommands::List => {
                controller.list_configs()?;
            }
        },
    }
    Ok(())
}
