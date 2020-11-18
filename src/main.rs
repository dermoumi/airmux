mod actions;
mod config;
mod utils;

use clap::{crate_description, crate_name, crate_version, load_yaml, App, ArgMatches};
use config::Config;
use snafu::Snafu;
use std::error;

pub const APP_NAME: &'static str = crate_name!();
pub const APP_AUTHOR: &'static str = "dermoumi";
pub const APP_VERSION: &'static str = crate_version!();
pub const APP_DESCRIPTION: &'static str = crate_description!();

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("project name cannot be empty"))]
    ProjectNameEmpty {},
    #[snafu(display("editor cannot be empty"))]
    EditorEmpty {},
}

fn main() -> Result<(), Box<dyn error::Error>> {
    let arg_config = load_yaml!("yaml/app.yml");
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
}

fn command_start(matches: &ArgMatches) -> Result<(), Box<dyn error::Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches).check()?;

    let project_name = matches
        .value_of_os("NAME")
        .ok_or(Error::ProjectNameEmpty {})?;
    let attach = !matches.is_present("no_attach");

    actions::start_project(&config, project_name, attach)
}

fn command_edit(matches: &ArgMatches) -> Result<(), Box<dyn error::Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches).check()?;

    let project_name = matches
        .value_of_os("NAME")
        .ok_or(Error::ProjectNameEmpty {})?;
    let editor = matches.value_of_os("editor").ok_or(Error::EditorEmpty {})?;

    actions::edit_project(&config, project_name, editor)
}

fn command_remove(matches: &ArgMatches) -> Result<(), Box<dyn error::Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches).check()?;

    let project_name = matches
        .value_of_os("NAME")
        .ok_or(Error::ProjectNameEmpty {})?;
    let no_input = matches.is_present("no_input");

    actions::remove_project(&config, project_name, no_input)
}

fn command_list(matches: &ArgMatches) -> Result<(), Box<dyn error::Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches).check()?;

    actions::list_projects(&config)
}
