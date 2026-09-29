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
        eprintln!("{}", err.red());
        std::process::exit(1);
    }
    Ok(())
}

fn run_controller(args: Cli, controller: Controller) -> Result<()> {
    match args.command {
        MainCommands::Run { jpda, config } => {
            controller.cleanup(config.clone())?;
            controller.deploy(config.clone())?;
            controller.run(config, jpda)?;
        }
        MainCommands::Debug { config } => {
            controller.cleanup(config.clone())?;
            controller.deploy(config.clone())?;
            controller.debug(config)?;
        }
        MainCommands::Deploy { config } => {
            controller.cleanup(config.clone())?;
            controller.deploy(config)?;
        }
        MainCommands::Config { command } => match command {
            ConfigCommands::Add {
                name,
                path,
                project_path,
                http_port,
                shutdown_port,
            } => {
                controller.add_config(name, path, project_path, http_port, shutdown_port)?;
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
