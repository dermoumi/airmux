extern crate rmux;
use rmux::config::Config;
use rmux::*;

use clap::{crate_description, crate_name, crate_version, load_yaml, App, ArgMatches};
use main_error::MainError;

use std::error::Error;

pub const APP_NAME: &'static str = crate_name!();
pub const APP_AUTHOR: &'static str = "rmux";
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
        ("debug", Some(sub_matches)) => command_debug(sub_matches),
        ("kill", Some(sub_matches)) => command_kill(sub_matches),
        ("edit", Some(sub_matches)) => command_edit(sub_matches),
        ("remove", Some(sub_matches)) => command_remove(sub_matches),
        ("list", Some(sub_matches)) => command_list(sub_matches),
        _ => panic!(),
    }
    .map_err(|x| x.into())
}

fn command_start(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches).check()?;

    let project_name = matches.value_of_os("project_name");
    let template = matches.value_of_os("template");
    let attach = matches.is_present("attach");
    let no_attach = matches.is_present("no_attach");
    let verbose = matches.is_present("verbose");
    let args = matches.values_of_lossy("args").unwrap_or(vec![]);

    let force_attach = if attach {
        Some(true)
    } else if no_attach {
        Some(false)
    } else {
        None
    };

    actions::start_project(
        &config,
        project_name.map(|s| s.to_os_string()),
        template.map(|s| s.to_os_string()),
        force_attach,
        false,
        verbose,
        args,
    )
}

fn command_debug(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches).check()?;

    let project_name = matches.value_of_os("project_name");
    let template = matches.value_of_os("template");
    let attach = matches.is_present("attach");
    let no_attach = matches.is_present("no_attach");
    let verbose = matches.is_present("verbose");
    let args = matches.values_of_lossy("args").unwrap_or(vec![]);

    let force_attach = if attach {
        Some(true)
    } else if no_attach {
        Some(false)
    } else {
        None
    };

    actions::start_project(
        &config,
        project_name.map(|s| s.to_os_string()),
        template.map(|s| s.to_os_string()),
        force_attach,
        true,
        verbose,
        args,
    )
}

fn command_kill(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches).check()?;

    let project_name = matches.value_of_os("project_name");
    let args = matches.values_of_lossy("args").unwrap_or(vec![]);

    actions::kill_project(&config, project_name.map(|s| s.to_os_string()), args)
}

fn command_edit(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches).check()?;

    let project_name = matches.value_of_os("project_name");
    let extension = matches.value_of_os("extension");
    let editor = matches.value_of_os("editor").unwrap();
    let no_check = matches.is_present("no_check");
    let args = matches.values_of_lossy("args").unwrap_or(vec![]);

    actions::edit_project(
        &config,
        project_name.map(|s| s.to_os_string()),
        extension.map(|s| s.to_os_string()),
        editor,
        no_check,
        args,
    )
}

fn command_remove(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches).check()?;

    let project_name = matches.value_of_os("project_name");
    let no_input = matches.is_present("no_input");

    actions::remove_project(&config, project_name.map(|s| s.to_os_string()), no_input)
}

fn command_list(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches).check()?;

    actions::list_projects(&config)
}
