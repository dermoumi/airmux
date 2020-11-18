mod actions;
mod config;
mod utils;

use clap::{crate_description, crate_name, crate_version, load_yaml, App, ArgMatches};
use config::Config;
use std::error;
use std::process::exit;

pub const APP_NAME: &'static str = crate_name!();
pub const APP_AUTHOR: &'static str = "dermoumi";
pub const APP_VERSION: &'static str = crate_version!();
pub const APP_DESCRIPTION: &'static str = crate_description!();

fn main() {
    let arg_config = load_yaml!("yaml/app.yml");
    let matches = App::from_yaml(&arg_config)
        .name(APP_NAME)
        .version(APP_VERSION)
        .about(APP_DESCRIPTION)
        .get_matches();

    if let Err(error) = match matches.subcommand() {
        ("start", Some(sub_matches)) => command_start(sub_matches),
        ("edit", Some(sub_matches)) => command_edit(sub_matches),
        ("remove", Some(sub_matches)) => command_remove(sub_matches),
        ("list", Some(sub_matches)) => command_list(sub_matches),
        _ => command_start(&matches),
    } {
        println!("An error has occured: {}", error);
        exit(1);
    }
}

fn command_start(matches: &ArgMatches) -> Result<(), Box<dyn error::Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches);
    config.check()?;

    let project_name = matches.value_of_os("NAME").ok_or("NAME cannot be empty")?;
    let attach = !matches.is_present("no_attach");

    actions::start_project(&config, project_name, attach)
}

fn command_edit(matches: &ArgMatches) -> Result<(), Box<dyn error::Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches);
    config.check()?;

    let project_name = matches.value_of_os("NAME").ok_or("NAME cannot be empty")?;
    let editor = matches
        .value_of_os("editor")
        .ok_or("editor cannot be empty")?;

    actions::edit_project(&config, project_name, editor)
}

fn command_remove(matches: &ArgMatches) -> Result<(), Box<dyn error::Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches);
    config.check()?;

    let project_name = matches.value_of_os("NAME").ok_or("NAME cannot be empty")?;
    let no_input = matches.is_present("no_input");

    actions::remove_project(&config, project_name, no_input)
}

fn command_list(matches: &ArgMatches) -> Result<(), Box<dyn error::Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches);
    config.check()?;

    actions::list_projects(&config)
}
