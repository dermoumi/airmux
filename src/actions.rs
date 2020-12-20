use crate::{pane::Pane, utils, window::Window};

use crate::config::Config;
use crate::pane_split::PaneSplit;
use crate::project::Project;
use crate::startup_window::StartupWindow;

use dialoguer::Confirm;
use mkdirp::mkdirp;
use shell_words::{join, quote};
use shellexpand::env_with_context;
use snafu::{ensure, Snafu};

use std::collections::HashMap;
use std::env;
use std::error;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const FILE_EXTENSIONS: &'static [&'static str] = &["yml", "yaml", "json"];

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("editor cannot be empty"))]
    EditorEmpty {},
    #[snafu(display("project name cannot be empty"))]
    ProjectNameEmpty,
    #[snafu(display("project {:?} does not exist", project_name))]
    ProjectDoesNotExist { project_name: OsString },
    #[snafu(display("project file {:?} is a directory", path))]
    ProjectFileIsADirectory { path: PathBuf },
    #[snafu(display("cannot pipe to tmux command"))]
    CannotPipeToTmux,
    #[snafu(display("session name is not set. please file a bug report."))]
    SessionNameNotSet,
    #[snafu(display("tmux failed with exit code: {}", exit_code))]
    TmuxFailed { exit_code: i32 },
    #[snafu(display("unsupported file extension: {:?}", extension))]
    UnsupportedFileExtension { extension: OsString },
    #[snafu(display("you should be in an active tmux session to run this command"))]
    NoActiveTmuxSession,
}

pub fn start_project(
    config: &Config,
    project_name: Option<OsString>,
    force_attach: Option<bool>,
    show_source: bool,
    verbose: bool,
    args: Vec<String>,
) -> Result<(), Box<dyn error::Error>> {
    let (project_name, project_file) = project::get_filename(config, project_name)?;
    ensure!(project_file.is_file(), ProjectDoesNotExist { project_name });

    let project = project::load(config, &project_name, &project_file, force_attach, &args)?;
    project.check()?;

    let source = source::generate(&project, verbose)?;

    // Run tmux
    if show_source {
        println!("{}", source);
    } else {
        // Some tmux versions close the tmux server if there are no running sessions
        // This prevents us from running `tmux source`.
        // So we create a dummy tmux session that we'll discard at the end
        let dummy_session = source::TmuxDummySession::new(&project)?;

        // Source our tmux config file
        let (tmux_command, tmux_args) =
            project.tmux_command(vec![OsString::from("source"), OsString::from("-")])?;

        let mut command = Command::new(tmux_command);
        command.args(tmux_args).stdin(Stdio::piped());

        if let Some(path) = &project.working_dir {
            if path.is_dir() {
                command.current_dir(path);
            }
        }

        let mut child = command.spawn()?;
        child
            .stdin
            .as_mut()
            .ok_or(Error::CannotPipeToTmux)?
            .write_all(source.as_bytes())?;

        // Wait until tmux completely finished processing input
        let status = child.wait()?;

        // Make sure to remove the dummy session before attaching,
        // Otherwise it'll pollute the session list the entire time we're attached
        // Because rmux won't quit until `tmux attach-session` returns
        drop(dummy_session);

        // Check tmux exit code
        ensure!(
            status.success(),
            TmuxFailed {
                exit_code: status.code().unwrap_or(-1)
            }
        );

        // Attach
        if project.attach {
            let session_name = project.session_name.as_ref().unwrap();
            let (tmux_command, tmux_args) = match option_env!("TMUX") {
                Some(_) => project.tmux_command(vec![
                    OsString::from("switch-client"),
                    OsString::from("-t"),
                    OsString::from(session_name),
                ])?,
                None => project.tmux_command(vec![
                    OsString::from("attach-session"),
                    OsString::from("-t"),
                    OsString::from(session_name),
                ])?,
            };
            Command::new(tmux_command).args(tmux_args).spawn()?.wait()?;
        }
    }

    Ok(())
}

pub fn kill_project(
    config: &Config,
    project_name: Option<OsString>,
    args: Vec<String>,
) -> Result<(), Box<dyn error::Error>> {
    let (project_name, project_file) = project::get_filename(config, project_name)?;
    ensure!(project_file.is_file(), ProjectDoesNotExist { project_name });

    let project = project::load(config, &project_name, &project_file, None, &args)?;
    project.check()?;

    let session_name = project
        .session_name
        .to_owned()
        .ok_or(/* should never happen */ Error::SessionNameNotSet {})?;

    // Run tmux
    let (tmux_command, tmux_args) = project.tmux_command(vec![
        OsString::from("kill-session"),
        OsString::from("-t"),
        OsString::from(&session_name),
    ])?;

    let status = Command::new(tmux_command).args(tmux_args).spawn()?.wait()?;

    ensure!(
        status.success(),
        TmuxFailed {
            exit_code: status.code().unwrap_or(-1)
        }
    );

    Ok(())
}

pub fn edit_project<S>(
    config: &Config,
    project_name: Option<OsString>,
    extension: Option<OsString>,
    editor: S,
    no_check: bool,
    args: Vec<String>,
) -> Result<(), Box<dyn error::Error>>
where
    S: AsRef<OsStr>,
{
    let (project_name, project_file) = project::get_filename(config, project_name)?;
    let extension = match extension {
        Some(extension) => extension.to_string_lossy().to_string(),
        None => project_file
            .extension()
            .map_or(String::from("yml"), |e| e.to_string_lossy().to_string()),
    };

    edit::check_supported_extension(&extension)?;
    let project_file = project_file.with_extension(&extension);

    edit::open_in_editor(
        config,
        project_name,
        project_file,
        extension.as_str(),
        editor,
        None,
        no_check,
        args,
    )
}

pub fn remove_project(
    config: &Config,
    project_name: Option<OsString>,
    no_input: bool,
) -> Result<(), Box<dyn error::Error>> {
    let (project_name, project_file) = project::get_filename(config, project_name)?;
    ensure!(project_file.is_file(), ProjectDoesNotExist { project_name });

    if !no_input
        && !Confirm::new()
            .with_prompt(format!(
                "Are you sure you want to remove {:?}?",
                project_name
            ))
            .default(false)
            .show_default(true)
            .interact()?
    {
        println!("Aborted.");
        return Ok(());
    }

    fs::remove_file(&project_file)?;

    // If it's in the projects directory, remove parent directories that are empty
    let projects_dir = config.get_projects_dir("")?;
    if project_file.starts_with(&projects_dir) {
        for parent in project_file.ancestors() {
            if parent == projects_dir {
                break;
            }

            let _ = fs::remove_dir(parent);
        }
    }

    println!("Project {:?} removed successfully.", project_name);
    Ok(())
}

pub fn list_projects(config: &Config) -> Result<(), Box<dyn error::Error>> {
    let data_dir = config.get_projects_dir("")?;

    let projects = list::get_projects(data_dir)?;
    println!(
        "{}",
        projects
            .into_iter()
            .map(|entry| entry.to_string_lossy().into())
            .collect::<Vec<String>>()
            .join("\n")
    );

    Ok(())
}

pub fn freeze_project<S>(
    config: &Config,
    stdout: bool,
    project_name: Option<OsString>,
    extension: Option<OsString>,
    editor: S,
    no_input: bool,
    no_check: bool,
    args: Vec<String>,
) -> Result<(), Box<dyn error::Error>>
where
    S: AsRef<OsStr>,
{
    let as_json = match &extension {
        Some(ext) if ext.to_string_lossy().to_lowercase() == "json" => true,
        _ => false,
    };

    let project = freeze::get_project(config)?;
    let content = project.serialize_compact(as_json)?;

    if stdout {
        println!("{}", content);
        return Ok(());
    }

    let (project_name, project_file) = project::get_filename(config, project_name)?;
    let extension = match extension {
        Some(extension) => extension.to_string_lossy().to_string(),
        None => project_file
            .extension()
            .map_or(String::from("yml"), |e| e.to_string_lossy().to_string()),
    };

    edit::check_supported_extension(&extension)?;
    let project_file = project_file.with_extension(&extension);

    if project_file.exists()
        && !no_input
        && !Confirm::new()
            .with_prompt(format!(
                "Project {:?} already exists, are you sure you want to override it?",
                project_name
            ))
            .default(false)
            .show_default(true)
            .interact()?
    {
        println!("Aborted.");
        return Ok(());
    }

    edit::open_in_editor(
        config,
        project_name,
        project_file,
        extension.as_str(),
        editor,
        Some(content),
        no_check,
        args,
    )
}

mod project {
    use super::*;

    pub fn get_filename<S>(
        config: &Config,
        project_filename: Option<S>,
    ) -> Result<(OsString, PathBuf), Box<dyn error::Error>>
    where
        S: AsRef<OsStr>,
    {
        match project_filename {
            Some(project_name) => {
                let project_name = project_name.as_ref();
                ensure!(!project_name.is_empty(), ProjectNameEmpty {});

                let projects_dir = config.get_projects_dir("")?;
                let project_file = projects_dir.join(project_name);
                let project_file = test_for_file_extensions(project_file)?;

                Ok((OsString::from(project_name), project_file))
            }
            None => {
                // Try to find a local project file in current directory and all ancestors
                let mut project_dir = std::env::current_dir()?;
                loop {
                    let project_file = project_dir.join(PathBuf::from(".rmux"));

                    // Try for each supported file extension
                    for ext in FILE_EXTENSIONS {
                        let project_file = project_file.with_extension(ext);
                        if project_file.exists() && !project_file.is_dir() {
                            let project_name = project_dir
                                .file_name()
                                .map_or(OsString::new(), |p| p.to_os_string());
                            return Ok((project_name, project_file));
                        }
                    }

                    // Move on to parent if nothing is found
                    match project_dir.parent() {
                        Some(parent_dir) => project_dir = parent_dir.to_path_buf(),
                        None => break,
                    }
                }

                // Fall back to local project file
                let project_dir = std::env::current_dir()?;
                let project_file = project_dir.join(".rmux.yml");
                let project_name = match project_dir.file_name() {
                    Some(name) => {
                        // Remove dots and colons
                        let unicode_name = name.to_string_lossy();
                        OsString::from(unicode_name.replace(".", "").replace(":", ""))
                    }
                    None => OsString::new(),
                };

                Ok((project_name, project_file))
            }
        }
    }

    pub fn load<S, P>(
        config: &Config,
        project_name: S,
        project_file: P,
        force_attach: Option<bool>,
        args: &[String],
    ) -> Result<Project, Box<dyn error::Error>>
    where
        S: AsRef<OsStr>,
        P: AsRef<Path>,
    {
        let project_name = project_name.as_ref();

        let project_yaml = fs::read_to_string(project_file)?;
        let project_yaml = env_with_context(&project_yaml, |s| env_context(s, args))
            .map_err(|x| x.to_string())?
            .to_string();

        Ok(serde_yaml::from_str::<Project>(&project_yaml)?.prepare(
            &config,
            &project_name.to_string_lossy(),
            force_attach,
        ))
    }

    pub fn env_context(s: &str, args: &[String]) -> Result<Option<String>, Box<dyn error::Error>> {
        // Check if it's a number and that it's > 0 and <= args.len()
        if let Ok(arg_index) = s.parse::<usize>() {
            if arg_index > 0 && arg_index <= args.len() {
                return Ok(Some(args[arg_index - 1].to_owned()));
            }
        }

        // Fallback to env vars
        Ok(env::var(s).ok())
    }

    pub fn test_for_file_extensions<P>(path: P) -> Result<PathBuf, Box<dyn error::Error>>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();

        // If the path already contains an extension, use it directly
        if let Some(extension) = path.extension() {
            edit::check_supported_extension(extension)?;
            return Ok(path.to_path_buf());
        }

        // Loop over extensions and try to file an existing file to reuse
        for extension in FILE_EXTENSIONS {
            let filename = path.with_extension(extension);
            if filename.exists() && !filename.is_dir() {
                return Ok(filename);
            }
        }

        // If no file was found, fall back to the first extension in the list
        return Ok(path.with_extension(FILE_EXTENSIONS[0]));
    }
}

mod source {
    use super::*;

    pub fn generate(project: &Project, verbose: bool) -> Result<String, Box<dyn error::Error>> {
        let tmux_command = project.tmux(Vec::<&str>::new())?;
        let tmux_command = tmux_command.as_str();

        let session_name = project.session_name.to_owned().unwrap();
        let session_name = session_name.as_str();

        let source = vec![
            // Clean up potentially lingering tmux env vars
            join(vec!["set-environment", "-gu", "__RMUX_SESSION_CREATED"]),
            join(vec!["set-environment", "-gu", "__RMUX_SESSION_UPDATED"]),
            // Assume that the tmux session will be attached until proven otherwise
            join(vec![
                "set-environment",
                "-g",
                "__RMUX_SESSION_ATTACHED",
                "1",
            ]),
            // on_start commands
            if project.on_start.len() > 0 {
                join(vec![
                    "run-shell",
                    project
                        .on_start
                        .join(";")
                        .replace("__TMUX__", tmux_command)
                        .as_str(),
                ])
            } else {
                String::new()
            },
            // Check if a session with the same name already exists
            join(vec![
                "if-shell",
                format!(
                    "! {} | {}",
                    project.tmux(vec!["list-sessions", "-F", "##S"])?,
                    join(vec!["grep", "-Fx", session_name]),
                )
                .as_str(),
                vec![
                    // Create new session
                    join(vec!["new-session", "-s", session_name, "-d"]),
                    // Move the first window away temporarily
                    join(vec![
                        "move-window",
                        "-s",
                        format!("{}:^", session_name).as_str(),
                        "-t",
                        format!("{}:999999", session_name).as_str(),
                    ]),
                    // on_first_start commands
                    if project.on_first_start.len() > 0 {
                        join(vec![
                            "run-shell",
                            project
                                .on_first_start
                                .join(";")
                                .replace("__TMUX__", tmux_command)
                                .replace("__SESSION__", quote(session_name).to_string().as_str())
                                .as_str(),
                        ])
                    } else {
                        String::new()
                    },
                    // on_exit commands
                    if project.on_exit.len() > 0 {
                        join(vec![
                            "set-hook",
                            "-t",
                            session_name,
                            "-a",
                            "client-detached",
                            format!(
                                "run-shell {}",
                                quote(
                                    project
                                        .on_exit
                                        .join(";")
                                        .replace("__TMUX__", tmux_command)
                                        .as_str()
                                )
                            )
                            .as_str(),
                        ])
                    } else {
                        String::new()
                    },
                    // on_stop (+on_exit) commands
                    if project.on_exit.len() > 0 || project.on_stop.len() > 0 {
                        let mut command_list = project
                            .on_exit
                            .to_owned()
                            .into_iter()
                            .chain(project.on_stop.to_owned().into_iter())
                            .collect::<Vec<String>>();
                        command_list.push(project.tmux(vec![
                            "set-hook",
                            "-gu",
                            "session-closed[#{session_created}]",
                        ])?);

                        let if_command = format!(
                            "! {} | {}",
                            project.tmux(vec!["list-sessions", "-F", "####S"])?,
                            join(vec!["grep", "-Fx", session_name]),
                        );

                        let run_shell_command = join(vec![
                            "run-shell",
                            command_list
                                .join(";")
                                .replace("__TMUX__", tmux_command)
                                .as_str(),
                        ]);

                        let hook_command = join(vec![
                            "if-shell",
                            if_command.as_str(),
                            run_shell_command.as_str(),
                        ]);

                        let set_hook_command = project.tmux(vec![
                            "set-hook",
                            "-g",
                            "session-closed[#{session_created}]",
                            hook_command.as_str(),
                        ])?;

                        let run_shell_set_hook_command = join(vec![
                            "run-shell",
                            "-t",
                            session_name,
                            set_hook_command.as_str(),
                        ]);

                        vec!["set -g exit-empty off", run_shell_set_hook_command.as_str()].join(";")
                    } else {
                        String::new()
                    },
                    // Set whether the session was created or not
                    String::from("set-environment -g __RMUX_SESSION_CREATED 1"),
                    // Unset the session attached variable
                    String::from("set-environment -gu __RMUX_SESSION_ATTACHED"),
                ]
                .join(";")
                .as_str(),
            ]),
            // on_restart commands
            if project.on_restart.len() > 0 {
                join(vec![
                    "if-shell",
                    "-F",
                    "#{__RMUX_SESSION_ATTACHED}",
                    join(vec![
                        "run-shell",
                        project
                            .on_restart
                            .join(";")
                            .replace("__TMUX__", tmux_command)
                            .replace("__SESSION__", quote(session_name).to_string().as_str())
                            .as_str(),
                    ])
                    .as_str(),
                ])
            } else {
                String::new()
            },
            // window base index
            join(vec![
                "set",
                "-s",
                "-t",
                session_name,
                "base-index",
                project.window_base_index.to_string().as_str(),
            ]),
            // Setup windows
            project
                .windows
                .iter()
                .enumerate()
                .map(
                    |(window_index, window)| -> Result<String, Box<dyn error::Error>> {
                        let window_tmux_index = window_index + project.window_base_index;
                        let target_window = format!("{}:{}", session_name, window_tmux_index);
                        let target_window = target_window.as_str();

                        let if_command = format!(
                            "! {} | {}",
                            project.tmux(vec!["list-windows", "-t", session_name, "-F", "##I",])?,
                            join(vec!["grep", "-Fx", window_tmux_index.to_string().as_str(),])
                        );

                        let mut new_window_command = vec!["new-window", "-d", "-t", target_window];

                        let mut found_working_dir = false;
                        let mut working_dir = String::new();

                        if window.panes.len() > 0 {
                            if let Some(wd) = &window.panes[0].working_dir {
                                working_dir = wd.to_string_lossy().to_string();
                                found_working_dir = true;
                            }
                        }
                        if !found_working_dir {
                            if let Some(wd) = &window.working_dir {
                                working_dir = wd.to_string_lossy().to_string();
                                found_working_dir = true;
                            }
                        }
                        if !found_working_dir {
                            if let Some(wd) = &project.working_dir {
                                working_dir = wd.to_string_lossy().to_string();
                                found_working_dir = true;
                            }
                        }

                        if found_working_dir {
                            new_window_command
                                .splice(2..2, vec!["-c", working_dir.as_str()].into_iter());
                        }

                        let run_shell_command = vec![
                            // Create the window
                            join(new_window_command),
                            // Pane base index
                            join(vec![
                                "set",
                                "-s",
                                "-t",
                                target_window,
                                "pane-base-index",
                                project.pane_base_index.to_string().as_str(),
                            ]),
                            // Rename the window
                            if let Some(window_name) = &window.name {
                                join(vec!["rename-window", "-t", target_window, window_name])
                            } else {
                                String::new()
                            },
                            // Window on_create commands
                            if window.on_create.len() > 0 {
                                join(vec![
                                    "run-shell",
                                    window
                                        .on_create
                                        .join(";")
                                        .replace("__TMUX__", tmux_command)
                                        .replace(
                                            "__SESSION__",
                                            quote(session_name).to_string().as_str(),
                                        )
                                        .replace(
                                            "__WINDOW__",
                                            quote(target_window).to_string().as_str(),
                                        )
                                        .as_str(),
                                ])
                            } else {
                                String::new()
                            },
                            // Panes
                            window
                                .panes
                                .iter()
                                .enumerate()
                                .map(
                                    |(pane_index, pane)| -> Result<String, Box<dyn error::Error>> {
                                        let target_pane = format!(
                                            "#{{__RMUX_PANE_{}}}",
                                            pane_index + project.pane_base_index
                                        );
                                        let target_pane = target_pane.as_str();

                                        Ok(vec![
                                            // Create pane (first one is automatically created)
                                            if pane_index > 0 {
                                                // Split direction
                                                let mut split_command = vec![
                                                    "split-window",
                                                    match &pane.split {
                                                        Some(split)
                                                            if *split == PaneSplit::Vertical =>
                                                        {
                                                            "-v"
                                                        }
                                                        _ => "-h",
                                                    },
                                                ];

                                                // Working directory
                                                let mut found_working_dir = true;
                                                let mut working_dir = String::new();

                                                if let Some(wd) = &pane.working_dir {
                                                    working_dir = wd.to_string_lossy().to_string();
                                                } else if let Some(wd) = &window.working_dir {
                                                    working_dir = wd.to_string_lossy().to_string();
                                                } else if let Some(wd) = &project.working_dir {
                                                    working_dir = wd.to_string_lossy().to_string();
                                                } else {
                                                    found_working_dir = false;
                                                }

                                                if found_working_dir {
                                                    split_command.append(&mut vec![
                                                        "-c",
                                                        working_dir.as_str(),
                                                    ]);
                                                }

                                                // Split size
                                                if let Some(split_size) = &pane.split_size {
                                                    split_command
                                                        .append(&mut vec!["-l", split_size]);
                                                }

                                                // Target pane
                                                let split_from_target;
                                                split_command.append(&mut vec![
                                                    "-t",
                                                    match &pane.split_from {
                                                        None => target_window,
                                                        Some(split_from) => {
                                                            split_from_target = format!(
                                                                "#{{__RMUX_PANE_{}}}",
                                                                split_from,
                                                            );

                                                            split_from_target.as_str()
                                                        }
                                                    },
                                                ]);

                                                // Create pane
                                                join(vec![
                                                    "run-shell",
                                                    project.tmux(split_command)?.as_str(),
                                                ])
                                            } else {
                                                String::new()
                                            },
                                            // Set real tmux pane index as a __RMUX_PANE_idx environment
                                            // Allows us to reference tmux panes with their project order
                                            join(vec![
                                                "run-shell",
                                                "-t",
                                                target_window,
                                                project
                                                    .tmux(vec![
                                                        "set-environment",
                                                        "-t",
                                                        session_name,
                                                        "-g",
                                                        format!(
                                                            "__RMUX_PANE_{}",
                                                            pane_index + project.pane_base_index
                                                        )
                                                        .as_str(),
                                                        "#D",
                                                    ])?
                                                    .as_str(),
                                            ]),
                                            // project and window's on_pane_create
                                            // plus pane's on_create commands
                                            join(vec![
                                                "run-shell",
                                                project
                                                    .on_pane_create
                                                    .iter()
                                                    .cloned()
                                                    .chain(window.on_pane_create.iter().cloned())
                                                    .chain(pane.on_create.iter().cloned())
                                                    .collect::<Vec<String>>()
                                                    .join(";")
                                                    .replace("__TMUX__", tmux_command)
                                                    .replace(
                                                        "__SESSION__",
                                                        quote(session_name).to_string().as_str(),
                                                    )
                                                    .replace(
                                                        "__WINDOW__",
                                                        quote(target_window).to_string().as_str(),
                                                    )
                                                    .replace(
                                                        "__PANE__",
                                                        quote(target_pane).to_string().as_str(),
                                                    )
                                                    .as_str(),
                                            ]),
                                            // project and window's pane_commands
                                            // plus pane commands
                                            join(vec![
                                                "run-shell",
                                                project
                                                    .pane_commands
                                                    .iter()
                                                    .chain(window.pane_commands.iter())
                                                    .chain(pane.commands.iter())
                                                    .filter(|command| !command.is_empty())
                                                    .map(|command| {
                                                        project.tmux(vec![
                                                            "send-keys",
                                                            "-t",
                                                            target_pane,
                                                            command,
                                                            "C-m",
                                                        ])
                                                    })
                                                    .collect::<Result<
                                                        Vec<String>,
                                                        Box<dyn error::Error>,
                                                    >>(
                                                    )?
                                                    .join(";")
                                                    .as_str(),
                                            ]),
                                            // project and window's post_pane_create
                                            // plus pane's post_create commands
                                            join(vec![
                                                "run-shell",
                                                project
                                                    .post_pane_create
                                                    .iter()
                                                    .cloned()
                                                    .chain(window.post_pane_create.iter().cloned())
                                                    .chain(pane.post_create.iter().cloned())
                                                    .collect::<Vec<String>>()
                                                    .join(";")
                                                    .replace("__TMUX__", tmux_command)
                                                    .replace(
                                                        "__SESSION__",
                                                        quote(session_name).to_string().as_str(),
                                                    )
                                                    .replace(
                                                        "__WINDOW__",
                                                        quote(target_window).to_string().as_str(),
                                                    )
                                                    .replace(
                                                        "__PANE__",
                                                        quote(target_pane).to_string().as_str(),
                                                    )
                                                    .as_str(),
                                            ]),
                                            // pane's clear
                                            if pane.clear {
                                                join(vec![
                                                    "run-shell",
                                                    project
                                                        .tmux(vec![
                                                            "send-keys",
                                                            "-t",
                                                            target_pane,
                                                            "C-l",
                                                        ])?
                                                        .as_str(),
                                                ])
                                            } else {
                                                String::new()
                                            },
                                        ]
                                        .join(";"))
                                    },
                                )
                                .collect::<Result<Vec<String>, Box<dyn error::Error>>>()?
                                .join(";"),
                            // Window layout
                            if let Some(layout) = &window.layout {
                                join(vec!["select-layout", "-t", target_window, layout])
                            } else {
                                String::new()
                            },
                            // Clean up panes index env vars
                            join(vec![
                                "run-shell",
                                window
                                    .panes
                                    .iter()
                                    .enumerate()
                                    .map(|(pane_index, _)| {
                                        project.tmux(vec![
                                            "set-environment",
                                            "-gu",
                                            format!(
                                                "__RMUX_PANE_{}",
                                                pane_index + project.pane_base_index
                                            )
                                            .as_str(),
                                        ])
                                    })
                                    .collect::<Result<Vec<String>, Box<dyn error::Error>>>()?
                                    .join(";")
                                    .as_str(),
                            ]),
                            // Select first pane
                            join(vec![
                                "select-pane",
                                "-t",
                                format!("{}.{}", target_window, project.pane_base_index).as_str(),
                            ]),
                            // window post_create commands
                            if window.post_create.len() > 0 {
                                join(vec![
                                    "run-shell",
                                    window
                                        .post_create
                                        .join(";")
                                        .replace("__TMUX__", tmux_command)
                                        .replace(
                                            "__SESSION__",
                                            quote(session_name).to_string().as_str(),
                                        )
                                        .replace(
                                            "__WINDOW__",
                                            quote(target_window).to_string().as_str(),
                                        )
                                        .as_str(),
                                ])
                            } else {
                                String::new()
                            },
                            // Flag session as updated
                            join(vec!["set-environment", "-g", "__RMUX_SESSION_UPDATED", "1"]),
                        ]
                        .join(";");

                        Ok(join(vec![
                            "if-shell",
                            if_command.as_str(),
                            run_shell_command.as_str(),
                        ]))
                    },
                )
                .collect::<Result<Vec<String>, Box<dyn error::Error>>>()?
                .join(";"),
            // Post-window creation routing for when the session is freshly created
            join(vec![
                "if-shell",
                "-F",
                "#{__RMUX_SESSION_CREATED}",
                vec![
                    // Remove the original window
                    join(vec![
                        "kill-window",
                        "-t",
                        (format!("{}:999999", session_name).as_str()),
                    ]),
                    // Set startup window
                    join(vec![
                        "select-window",
                        "-t",
                        match &project.startup_window {
                            StartupWindow::Index(startup_window) => {
                                format!("{}:{}", session_name, startup_window)
                            }
                            StartupWindow::Name(startup_window) => {
                                format!("{}:{}", session_name, startup_window)
                            }
                            StartupWindow::Default => format!("{}:^", session_name),
                        }
                        .as_str(),
                    ]),
                    // Set startup pane
                    join(vec![
                        "select-pane",
                        "-t",
                        match &project.startup_pane {
                            None => project.pane_base_index,
                            Some(pane) => *pane,
                        }
                        .to_string()
                        .as_str(),
                    ]),
                ]
                .join(";")
                .as_str(),
            ]),
            // post_create commands
            if project.post_create.len() > 0 {
                join(vec![
                    "run-shell",
                    project
                        .post_create
                        .join(";")
                        .replace("__TMUX__", tmux_command)
                        .replace("__SESSION__", quote(session_name).to_string().as_str())
                        .as_str(),
                ])
            } else {
                String::new()
            },
            // Show indicator message
            if verbose {
                vec![
                    String::from("display -p \"#{?__RMUX_SESSION_CREATED,created new session:,"),
                    String::from("#{?__RMUX_SESSION_UPDATED,updated session:,"),
                    String::from("no changes to existing session:}} "),
                    session_name.to_string(),
                    String::from("\""),
                ]
                .join("")
            } else {
                String::new()
            },
            // Clear variables
            join(vec!["set-environment", "-gu", "__RMUX_SESSION_CREATED"]),
            join(vec!["set-environment", "-gu", "__RMUX_SESSION_UPDATED"]),
        ]
        .join(";");

        Ok(source)
    }

    pub struct TmuxDummySession<'a> {
        project: &'a Project,
    }

    impl<'a> TmuxDummySession<'a> {
        pub fn new(project: &'a Project) -> Result<TmuxDummySession, Box<dyn error::Error>> {
            // Create dummy tmux session to make sure the tmux server is up and running
            let (tmux_command, tmux_args) = project.tmux_command(vec![
                OsString::from("new-session"),
                OsString::from("-s"),
                OsString::from("__rmux_dummy_session_"),
                OsString::from("-d"),
            ])?;

            let _ = Command::new(tmux_command)
                .args(tmux_args)
                .env_remove("TMUX")
                .spawn()?
                .wait();

            Ok(TmuxDummySession { project })
        }
    }

    impl<'a> Drop for TmuxDummySession<'a> {
        fn drop(&mut self) {
            // Remove dummy session
            if let Ok((tmux_command, tmux_args)) = self.project.tmux_command(vec![
                OsString::from("kill-session"),
                OsString::from("-t"),
                OsString::from("__rmux_dummy_session_"),
            ]) {
                if let Ok(mut child) = Command::new(tmux_command).args(tmux_args).spawn() {
                    let _ = child.wait();
                }
            }
        }
    }
}

mod edit {
    use super::*;

    pub fn create_project<S, P>(
        project_name: S,
        project_path: P,
        extension: &str,
        content: Option<String>,
    ) -> Result<(), Box<dyn error::Error>>
    where
        S: AsRef<OsStr>,
        P: AsRef<Path>,
    {
        let project_name = project_name.as_ref();
        let project_name = strip_extension_from_project_name(project_name);

        let project_path = project_path.as_ref();
        let mut file = fs::File::create(&project_path)?;

        let content = match content {
            Some(content) => content,
            None => {
                let as_json = extension == "json";

                let content = if as_json {
                    include_str!("assets/default_project.json")
                } else {
                    include_str!("assets/default_project.yml")
                };

                let project_name = project_name.to_string_lossy();
                let project_name = if as_json {
                    serde_json::to_string(&project_name)?
                } else {
                    // serde_yaml adds '---\n' at the beginning that we need to get rid of before using the name
                    let serialized = serde_yaml::to_string(&project_name)?;
                    serialized[4..].to_string()
                };

                content.replace("__PROJECT_NAME__", &project_name)
            }
        };

        file.write_all(content.as_bytes())?;
        file.sync_all()?;

        Ok(())
    }

    pub fn check_supported_extension<S>(extension: S) -> Result<(), Box<dyn error::Error>>
    where
        S: AsRef<OsStr>,
    {
        let extension = extension.as_ref();

        ensure!(
            FILE_EXTENSIONS.contains(&extension.to_string_lossy().to_lowercase().as_str()),
            UnsupportedFileExtension {
                extension: extension.to_os_string()
            }
        );

        Ok(())
    }

    // Disguise the project name as a Path for easy access to .with_extension()
    pub fn strip_extension_from_project_name<P>(project_name: P) -> OsString
    where
        P: AsRef<Path>,
    {
        OsString::from(project_name.as_ref().with_extension(""))
    }

    pub fn open_in_editor<S>(
        config: &Config,
        project_name: OsString,
        project_file: PathBuf,
        extension: &str,
        editor: S,
        content: Option<String>,
        no_check: bool,
        args: Vec<String>,
    ) -> Result<(), Box<dyn error::Error>>
    where
        S: AsRef<OsStr>,
    {
        let editor = editor.as_ref();
        ensure!(!editor.is_empty(), EditorEmpty {});

        // Make sure the project's parent directory exists
        if let Some(parent) = project_file.parent() {
            mkdirp(parent)?;
        }

        // Make sure the project file exists
        ensure!(
            !project_file.is_dir(),
            ProjectFileIsADirectory { path: project_file }
        );

        if !project_file.exists() || content.is_some() {
            edit::create_project(&project_name, &project_file, extension, content)?;
        }

        // Open it with editor
        let editor = OsString::from(editor);
        let (command, command_args) =
            utils::parse_command(editor.as_os_str(), &[OsString::from(&project_file)])?;
        let mut child = Command::new(command).args(command_args).spawn()?;

        if !no_check {
            child.wait()?;

            // Perform a check on the project
            let project = project::load(config, &project_name, &project_file, None, &args)?;
            project.check()?;
        }

        Ok(())
    }
}

mod list {
    use super::*;

    pub fn get_projects<P>(path: P) -> Result<Vec<OsString>, Box<dyn error::Error>>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        let mut projects = vec![];

        for entry in path.read_dir()? {
            let entry = entry?;
            let entry_path = entry.path();

            if entry_path.is_file() {
                // Ignore file if it doesn't have a supported file extension
                if let Some(extension) = entry_path.extension() {
                    if let Ok(_) = edit::check_supported_extension(extension) {
                        let file_path = entry_path.strip_prefix(path)?;
                        projects.push(OsString::from(file_path.with_extension("")));
                    }
                }
            } else if entry_path.is_dir() {
                // Check for symlink loops
                let subdir = if entry.file_type()?.is_symlink() {
                    let subdir = entry_path.read_link()?;

                    if entry_path.starts_with(&subdir) {
                        continue;
                    }

                    subdir
                } else {
                    entry_path.clone()
                };

                let file_path = entry_path.strip_prefix(path)?;
                let mut subdir_projects = list::get_projects(&subdir)?
                    .into_iter()
                    .map(|entry| OsString::from(file_path.join(entry)))
                    .collect();
                projects.append(&mut subdir_projects);
            }
        }

        Ok(projects)
    }
}

mod freeze {
    use super::*;

    pub fn get_project(config: &Config) -> Result<Project, Box<dyn error::Error>> {
        let mut project = Project {
            windows: vec![],
            ..Project::default()
        };

        let session_id = freeze::get_tmux_value(config, "session_id", None)?;

        project.session_name = Some(freeze::get_tmux_value(
            config,
            "session_name",
            Some(session_id.as_str()),
        )?);

        let mut window_working_dir_map: HashMap<PathBuf, usize> = HashMap::new();
        let mut window_most_used_working_dir = PathBuf::new();
        let mut window_most_used_working_dir_count = 0;

        let window_ids =
            freeze::get_tmux_list_values(config, "list-windows", "window_id", session_id.as_str())?;
        for window_id in &window_ids {
            let mut window = Window {
                panes: vec![],
                ..Window::default()
            };

            let window_name =
                freeze::get_tmux_value(config, "window_name", Some(window_id.as_str()))?;
            let mut window_name = if window_name.is_empty() {
                None
            } else {
                Some(window_name)
            };

            let mut pane_working_dir_map: HashMap<PathBuf, usize> = HashMap::new();
            let mut pane_most_used_working_dir = PathBuf::new();
            let mut pane_most_used_working_dir_count = 0;

            let pane_ids =
                freeze::get_tmux_list_values(config, "list-panes", "pane_id", window_id.as_str())?;
            for pane_id in &pane_ids {
                let mut pane = Pane { ..Pane::default() };

                let pane_current_path = PathBuf::from(freeze::get_tmux_value(
                    config,
                    "pane_current_path",
                    Some(pane_id.as_str()),
                )?);
                pane.working_dir = Some(pane_current_path.to_owned());

                let pane_shell_path =
                    freeze::get_tmux_value(config, "SHELL", Some(pane_id.as_str()))?;

                let pane_shell = PathBuf::from(&pane_shell_path)
                    .file_name()
                    .map_or_else(|| String::new(), |s| s.to_string_lossy().to_string());

                let pane_command_path =
                    freeze::get_tmux_value(config, "pane_current_command", Some(pane_id.as_str()))?;

                let pane_command = PathBuf::from(&pane_command_path)
                    .file_name()
                    .map_or_else(|| String::new(), |s| s.to_string_lossy().to_string());

                let process_name = std::env::current_exe()?
                    .file_name()
                    .map_or(String::from("rmux"), |n| n.to_string_lossy().to_string());

                if let Some(name) = &window_name {
                    if name == &pane_command || name == &pane_shell || name == &process_name {
                        window_name = None
                    }
                }

                match pane_working_dir_map.get(&pane_current_path) {
                    Some(count_value) => {
                        let count_value = count_value + 1;
                        pane_working_dir_map.insert(pane_current_path.to_owned(), count_value);

                        if count_value > pane_most_used_working_dir_count {
                            pane_most_used_working_dir = pane_current_path;
                            pane_most_used_working_dir_count = count_value;
                        }
                    }
                    None => {
                        let count_value = 1;
                        pane_working_dir_map.insert(pane_current_path.to_owned(), count_value);

                        if count_value >= pane_most_used_working_dir_count {
                            pane_most_used_working_dir = pane_current_path;
                            pane_most_used_working_dir_count = count_value;
                        }
                    }
                }

                window.panes.push(pane);
            }

            // Set window name
            window.name = window_name;

            // Set working directory if any
            if pane_most_used_working_dir_count > 0 {
                window.working_dir = Some(pane_most_used_working_dir.to_owned());

                for pane in &mut window.panes {
                    if let Some(working_dir) = &pane.working_dir {
                        if working_dir == &pane_most_used_working_dir {
                            pane.working_dir = None;
                        }
                    }
                }

                match window_working_dir_map.get(&pane_most_used_working_dir) {
                    Some(count_value) => {
                        let count_value = count_value + 1;
                        window_working_dir_map
                            .insert(pane_most_used_working_dir.to_owned(), count_value);

                        if count_value > window_most_used_working_dir_count {
                            window_most_used_working_dir = pane_most_used_working_dir;
                            window_most_used_working_dir_count = count_value;
                        }
                    }
                    None => {
                        let count_value = 1;
                        window_working_dir_map
                            .insert(pane_most_used_working_dir.to_owned(), count_value);

                        if count_value >= window_most_used_working_dir_count {
                            window_most_used_working_dir = pane_most_used_working_dir;
                            window_most_used_working_dir_count = count_value;
                        }
                    }
                }
            }

            // Set layout
            let layout = freeze::get_tmux_value(config, "window_layout", Some(window_id.as_str()))?;
            window.layout = Some(layout);

            // Add window to project's window list
            project.windows.push(window)
        }

        if window_most_used_working_dir_count > 0 {
            project.working_dir = Some(window_most_used_working_dir.to_owned());

            for window in &mut project.windows {
                if let Some(working_dir) = &window.working_dir {
                    if working_dir == &window_most_used_working_dir {
                        window.working_dir = None;
                    }
                }
            }
        }

        Ok(project)
    }

    pub fn get_tmux_value(
        config: &Config,
        value: &str,
        target: Option<&str>,
    ) -> Result<String, Box<dyn error::Error>> {
        ensure!(std::option_env!("TMUX").is_some(), NoActiveTmuxSession);

        let mut tmux_args = vec![OsString::from("display")];

        if let Some(target) = target {
            tmux_args.append(&mut vec![OsString::from("-t"), OsString::from(target)]);
        }

        tmux_args.append(&mut vec![
            OsString::from("-p"),
            OsString::from(format!("#{{{}}}", value)),
        ]);

        let (tmux, arguments) = config.get_tmux_command(tmux_args)?;

        let value = String::from_utf8(Command::new(tmux).args(arguments).output()?.stdout)?
            .trim()
            .to_string();
        Ok(value)
    }

    pub fn get_tmux_list_values(
        config: &Config,
        list_command: &str,
        value: &str,
        target: &str,
    ) -> Result<Vec<String>, Box<dyn error::Error>> {
        let tmux_args = vec![
            OsString::from(list_command),
            OsString::from("-t"),
            OsString::from(target),
            OsString::from("-F"),
            OsString::from(format!("#{{{}}}", value)),
        ];
        let (tmux, arguments) = config.get_tmux_command(tmux_args)?;

        let values =
            String::from_utf8(Command::new(tmux).args(arguments).output()?.stdout)?.to_string();

        let values = values
            .split("\n")
            .filter_map(|window_id| {
                let window_id = window_id.trim();
                if window_id.is_empty() {
                    None
                } else {
                    Some(window_id.to_string())
                }
            })
            .collect();
        Ok(values)
    }
}

#[cfg(test)]
#[path = "test/actions.rs"]
mod tests;
