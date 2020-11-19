extern crate rmux;
use rmux::config::Config;
use rmux::*;

use clap::{crate_description, crate_name, crate_version, load_yaml, App, ArgMatches};
use main_error::MainError;

use std::error::Error;

pub const APP_NAME: &'static str = crate_name!();
pub const APP_AUTHOR: &'static str = "dermoumi";
pub const APP_VERSION: &'static str = crate_version!();
pub const APP_DESCRIPTION: &'static str = crate_description!();

fn main() -> Result<(), MainError> {
    let arg_config = load_yaml!("app.yml");
    let matches = App::from_yaml(&arg_config)
        .name(APP_NAME)
        .version(APP_VERSION)
        .about(APP_DESCRIPTION)
        .get_matches();

    match matches.subcommand() {
        ("start", Some(sub_matches)) => command_start(sub_matches),
        ("edit", Some(sub_matches)) => command_edit(sub_matches),
        ("remove", Some(sub_matches)) => command_remove(sub_matches),
        ("list", Some(sub_matches)) => command_list(sub_matches),
        _ => command_start(&matches),
    }
    .map_err(|x| x.into())
}

fn command_start(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches).check()?;

    let project_name = matches.value_of_os("NAME").ok_or("")?;
    let attach = !matches.is_present("no_attach");

    actions::start_project(&config, project_name, attach)
}

fn command_edit(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches).check()?;

    let project_name = matches.value_of_os("NAME").ok_or("")?;
    let editor = matches.value_of_os("editor").ok_or("")?;

    actions::edit_project(&config, project_name, editor)
}

fn command_remove(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches).check()?;

    let project_name = matches.value_of_os("NAME").ok_or("")?;
    let no_input = matches.is_present("no_input");

    actions::remove_project(&config, project_name, no_input)
}

fn command_list(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches).check()?;

    actions::list_projects(&config)
}
