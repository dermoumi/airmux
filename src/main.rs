mod actions;
mod utils;

use clap::{load_yaml, App, ArgMatches};
use std::error::Error;
use std::process::exit;

fn main() {
    let arg_config = load_yaml!("config/app.yml");
    let matches = App::from_yaml(&arg_config)
        .name(utils::APP_NAME)
        .version(utils::APP_VERSION)
        .about(utils::APP_DESCRIPTION)
        .get_matches();

    if let Err(error) = match matches.subcommand() {
        ("start", Some(sub_matches)) => command_start(sub_matches),
        ("edit", Some(sub_matches)) => command_edit(sub_matches),
        ("remove", Some(sub_matches)) => command_remove(sub_matches),
        _ => command_start(&matches),
    } {
        println!("An error has occured: {}", error);
        exit(1);
    }
}

fn command_start(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let tmux_command = matches
        .value_of_os("tmux_command")
        .ok_or("tmux command cannot be empty")?;
    let project_name = matches.value_of_os("NAME").ok_or("NAME cannot be empty")?;
    let attach = !matches.is_present("no-attach");

    actions::start_project(tmux_command, project_name, attach)
}

fn command_edit(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let tmux_command = matches
        .value_of_os("tmux_command")
        .ok_or("tmux command cannot be empty")?;
    let project_name = matches.value_of_os("NAME").ok_or("NAME cannot be empty")?;
    let editor = matches
        .value_of_os("editor")
        .ok_or("editor cannot be empty")?;

    actions::edit_project(tmux_command, project_name, editor)
}

fn command_remove(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let tmux_command = matches
        .value_of_os("tmux_command")
        .ok_or("tmux command cannot be empty")?;
    let project_name = matches.value_of_os("NAME").ok_or("NAME cannot be empty")?;
    let no_input = matches.is_present("no-input");

    actions::remove_project(tmux_command, project_name, no_input)
}
