extern crate airmux;
use airmux::config::Config;
use airmux::*;

use clap::{
    crate_description, crate_name, crate_version, App, AppSettings, Arg, ArgMatches, SubCommand,
};
use main_error::MainError;

use std::error::Error;

pub const APP_NAME: &str = crate_name!();
pub const APP_AUTHOR: &str = "airmux";
pub const APP_VERSION: &str = crate_version!();
pub const APP_DESCRIPTION: &str = crate_description!();

fn main() -> Result<(), MainError> {
    let app = App::new("airmux")
        .name(APP_NAME)
        .version(APP_VERSION)
        .about(APP_DESCRIPTION)
        .settings(&[
            AppSettings::SubcommandRequired,
            AppSettings::VersionlessSubcommands,
            AppSettings::InferSubcommands,
        ])
        .arg(
            Arg::with_name("config_dir")
                .global(true)
                .help("configuration directory to use")
                .short("c")
                .long("config-dir")
                .value_name("DIR")
                .env("AIRMUX_CONFIG"),
        )
        .subcommands(vec![
            SubCommand::with_name("list")
                .about("List all configured projects")
                .alias("ls"),
            SubCommand::with_name("start")
                .about("Start a project as a tmux session")
                .args(&[
                    Arg::with_name("project_name")
                        .help("name of the project")
                        .value_name("PROJECT_NAME")
                        .index(1),
                    Arg::with_name("attach")
                        .help("force attach the session")
                        .short("a")
                        .long("attach")
                        .conflicts_with("no_attach"),
                    Arg::with_name("no_attach")
                        .help("don't automatically attach the session")
                        .short("d")
                        .long("no-attach"),
                    Arg::with_name("verbose")
                        .help("print a message if the session was created or updated")
                        .short("V")
                        .long("verbose"),
                    Arg::with_name("args")
                        .help("arguments to be passed as variables to the yaml file")
                        .value_name("ARGUMENT")
                        .multiple(true),
                    Arg::with_name("tmux_command")
                        .global(true)
                        .help("tmux command to use")
                        .short("t")
                        .long("command")
                        .value_name("COMMAND")
                        .env("AIRMUX_COMMAND"),
                ]),
            SubCommand::with_name("debug")
                .about("Print tmux source without actually running tmux")
                .args(&[
                    Arg::with_name("project_name")
                        .help("name of the project")
                        .value_name("PROJECT_NAME")
                        .index(1),
                    Arg::with_name("attach")
                        .help("force attach the session (ignored)")
                        .short("a")
                        .long("attach")
                        .conflicts_with("no_attach"),
                    Arg::with_name("no_attach")
                        .help("don't automatically attach the session (ignored)")
                        .short("d")
                        .long("no-attach"),
                    Arg::with_name("verbose")
                        .help("print a message if the session was created or updated")
                        .short("V")
                        .long("verbose"),
                    Arg::with_name("args")
                        .help("arguments to be passed as variables to the yaml file")
                        .value_name("ARGUMENT")
                        .multiple(true),
                    Arg::with_name("tmux_command")
                        .global(true)
                        .help("tmux command to use")
                        .short("t")
                        .long("command")
                        .value_name("COMMAND")
                        .env("AIRMUX_COMMAND"),
                ]),
            SubCommand::with_name("kill")
                .about("Kill tmux session that matches the project")
                .args(&[
                    Arg::with_name("project_name")
                        .help("name of the project")
                        .value_name("PROJECT_NAME")
                        .index(1),
                    Arg::with_name("args")
                        .help("arguments to be passed as variables to the yaml file")
                        .value_name("ARGUMENT")
                        .multiple(true),
                    Arg::with_name("tmux_command")
                        .global(true)
                        .help("tmux command to use")
                        .short("t")
                        .long("command")
                        .value_name("COMMAND")
                        .env("AIRMUX_COMMAND"),
                ]),
            SubCommand::with_name("edit")
                .about("Create or edit a project")
                .alias("new")
                .args(&[
                    Arg::with_name("project_name")
                        .help("name of the project")
                        .value_name("PROJECT_NAME")
                        .index(1),
                    Arg::with_name("extension")
                        .help("the extension to use for the project file (yml|yaml|json)")
                        .short("e")
                        .long("ext")
                        .value_name("FILE_EXT")
                        .possible_values(&["yml", "yaml", "json"])
                        .case_insensitive(true),
                    Arg::with_name("editor")
                        .help("the editor to use")
                        .short("E")
                        .long("editor")
                        .required(true)
                        .value_name("EDITOR")
                        .env("EDITOR"),
                    Arg::with_name("no_check")
                        .help("do not check the project file")
                        .short("C")
                        .long("no-check"),
                    Arg::with_name("args")
                        .help("arguments to be passed as variables to the yaml file when checking")
                        .value_name("ARGUMENT")
                        .multiple(true),
                    Arg::with_name("tmux_command")
                        .global(true)
                        .help("tmux command to use")
                        .short("t")
                        .long("command")
                        .value_name("COMMAND")
                        .env("AIRMUX_COMMAND"),
                ]),
            SubCommand::with_name("remove")
                .about("Remove a project (does not affect loaded tmux sessions)")
                .aliases(&["rm", "delete"])
                .args(&[
                    Arg::with_name("project_name")
                        .help("name of the project")
                        .value_name("PROJECT_NAME")
                        .index(1),
                    Arg::with_name("no_input")
                        .help("do not prompt for confirmation")
                        .short("y")
                        .long("no-input"),
                ]),
            SubCommand::with_name("freeze")
                .about("Save current tmux session as a project file (commands not included)")
                .args(&[
                    Arg::with_name("stdout")
                        .help("print the project file to stdout instead")
                        .short("s")
                        .long("stdout")
                        .conflicts_with_all(&[
                            "project_name",
                            "editor",
                            "no_input",
                            "no_check",
                            "args",
                        ]),
                    Arg::with_name("project_name")
                        .help("name of the project")
                        .value_name("PROJECT_NAME")
                        .index(1),
                    Arg::with_name("extension")
                        .help("the extension to use for the project file (yml|yaml|json)")
                        .short("e")
                        .long("ext")
                        .value_name("FILE_EXT")
                        .possible_values(&["yml", "yaml", "json"])
                        .case_insensitive(true),
                    Arg::with_name("no_input")
                        .help("do not prompt for confirmation")
                        .short("y")
                        .long("no-input"),
                    Arg::with_name("editor")
                        .help("the editor to use")
                        .short("E")
                        .long("editor")
                        .required(true)
                        .value_name("EDITOR")
                        .env("EDITOR"),
                    Arg::with_name("no_check")
                        .help("do not check the project file")
                        .short("C")
                        .long("no-check"),
                    Arg::with_name("args")
                        .help("arguments to be passed as variables to the yaml file when checking")
                        .value_name("ARGUMENT")
                        .multiple(true),
                    Arg::with_name("tmux_command")
                        .global(true)
                        .help("tmux command to use")
                        .short("t")
                        .long("command")
                        .value_name("COMMAND")
                        .env("AIRMUX_COMMAND"),
                ]),
        ]);

    let matches = app.get_matches();
    match matches.subcommand() {
        ("start", Some(sub_matches)) => command_start(sub_matches),
        ("debug", Some(sub_matches)) => command_debug(sub_matches),
        ("kill", Some(sub_matches)) => command_kill(sub_matches),
        ("edit", Some(sub_matches)) => command_edit(sub_matches),
        ("remove", Some(sub_matches)) => command_remove(sub_matches),
        ("list", Some(sub_matches)) => command_list(sub_matches),
        ("freeze", Some(sub_matches)) => command_freeze(sub_matches),
        _ => panic!(),
    }
    .map_err(|x| x.into())
}

fn command_start(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches).check()?;

    let project_name = matches.value_of_lossy("project_name");
    let attach = matches.is_present("attach");
    let no_attach = matches.is_present("no_attach");
    let verbose = matches.is_present("verbose");
    let args = matches.values_of_lossy("args").unwrap_or_default();
    let args: Vec<&str> = args.iter().map(AsRef::as_ref).collect();

    let force_attach = if attach {
        Some(true)
    } else if no_attach {
        Some(false)
    } else {
        None
    };

    actions::start_project(
        &config,
        project_name.as_deref(),
        force_attach,
        false,
        verbose,
        &args,
    )
}

fn command_debug(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches).check()?;

    let project_name = matches.value_of_lossy("project_name");
    let attach = matches.is_present("attach");
    let no_attach = matches.is_present("no_attach");
    let verbose = matches.is_present("verbose");
    let args = matches.values_of_lossy("args").unwrap_or_default();
    let args: Vec<&str> = args.iter().map(AsRef::as_ref).collect();

    let force_attach = if attach {
        Some(true)
    } else if no_attach {
        Some(false)
    } else {
        None
    };

    actions::start_project(
        &config,
        project_name.as_deref(),
        force_attach,
        true,
        verbose,
        &args,
    )
}

fn command_kill(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches).check()?;

    let project_name = matches.value_of_lossy("project_name");
    let args = matches.values_of_lossy("args").unwrap_or_default();
    let args: Vec<&str> = args.iter().map(AsRef::as_ref).collect();

    actions::kill_project(&config, project_name.as_deref(), &args)
}

fn command_edit(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches).check()?;

    let project_name = matches.value_of_lossy("project_name");
    let extension = matches.value_of_lossy("extension");
    let editor = matches.value_of_lossy("editor").unwrap();
    let no_check = matches.is_present("no_check");
    let args = matches.values_of_lossy("args").unwrap_or_default();
    let args: Vec<&str> = args.iter().map(AsRef::as_ref).collect();

    actions::edit_project(
        &config,
        project_name.as_deref(),
        extension.as_deref(),
        &editor,
        no_check,
        &args,
    )
}

fn command_remove(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches).check()?;

    let project_name = matches.value_of_lossy("project_name");
    let no_input = matches.is_present("no_input");

    actions::remove_project(&config, project_name.as_deref(), no_input)
}

fn command_list(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches).check()?;

    actions::list_projects(&config)
}

fn command_freeze(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let config = Config::from_args(APP_NAME, APP_AUTHOR, matches).check()?;

    let stdout = matches.is_present("stdout");
    let project_name = matches.value_of_lossy("project_name");
    let extension = matches.value_of_lossy("extension");
    let no_input = matches.is_present("no_input");
    let editor = matches.value_of_lossy("editor").unwrap();
    let no_check = matches.is_present("no_check");
    let args = matches.values_of_lossy("args").unwrap_or_default();
    let args: Vec<&str> = args.iter().map(AsRef::as_ref).collect();

    actions::freeze_project(
        &config,
        stdout,
        project_name.as_deref(),
        extension.as_deref(),
        &editor,
        no_input,
        no_check,
        &args,
    )
}
