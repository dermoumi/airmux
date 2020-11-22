extern crate rmux;
use rmux::config::Config;
use rmux::*;

use clap::{crate_description, crate_name, crate_version, load_yaml, App, ArgMatches};
use main_error::MainError;

use std::error::Error;
use std::fs;

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

    let project_name = matches.value_of_os("project_name").unwrap();
    let template = matches.value_of_os("template");
    let attach = matches.is_present("attach");
    let no_attach = matches.is_present("no_attach");
    let source = matches.is_present("source");
    let verbose = matches.is_present("verbose");

    let force_attach = if attach {
        Some(true)
    } else if no_attach {
        Some(false)
    } else {
        None
    };

    let template_content;
    let template = match template {
        Some(filename) => {
            template_content = fs::read_to_string(filename)?;
            Some(template_content.as_str())
        }
        _ => None,
    };

    actions::start_project(
        &config,
        project_name,
        template,
        force_attach,
        source,
        verbose,
    )
}

fn command_kill(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches).check()?;

    let project_name = matches.value_of_os("project_name").unwrap();

    actions::kill_project(&config, project_name)
}

fn command_edit(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches).check()?;

    let project_name = matches.value_of_os("project_name").unwrap();
    let editor = matches.value_of_os("editor").unwrap();
    let no_check = matches.is_present("no_check");

    actions::edit_project(&config, project_name, editor, no_check)
}

fn command_remove(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches).check()?;

    let project_name = matches.value_of_os("project_name").unwrap();
    let no_input = matches.is_present("no_input");

    actions::remove_project(&config, project_name, no_input)
}

fn command_list(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches).check()?;

    actions::list_projects(&config)
}
