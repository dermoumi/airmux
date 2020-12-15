use crate::utils;

use crate::config::Config;
use crate::project::Project;
use crate::project_template::ProjectTemplate;

use dialoguer::Confirm;
use mkdirp::mkdirp;
use serde_json::{json, value::Value};
use shell_words::quote;
use shellexpand::env_with_context;
use snafu::{ensure, Snafu};
use tera::{Context, Tera};

use std::collections::HashMap;
use std::env;
use std::error;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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
}

pub fn start_project<S: AsRef<OsStr>>(
    config: &Config,
    project_name: S,
    template: Option<&str>,
    force_attach: Option<bool>,
    show_source: bool,
    verbose: bool,
    args: Vec<String>,
) -> Result<(), Box<dyn error::Error>> {
    let project = project::load(config, &project_name, force_attach, &args)?;
    project.check()?;

    // Build and run tmux commands
    let mut context = Context::new();
    context.insert("project", &project);
    context.insert("verbose", &verbose);

    let template_comand = project.get_tmux_command_for_template()?;
    context.insert("tmux_command", &template_comand);

    let mut tera = Tera::default();
    tera.register_filter("quote", source::QuoteFilter {});

    let template_content: String;
    let template = match &project.template {
        ProjectTemplate::Raw(content) => {
            template_content = content.into();
            template_content.as_str()
        }
        ProjectTemplate::File(filename) => {
            let full_path = if filename.has_root() {
                filename.to_owned()
            } else {
                let project_dir = config
                    .get_projects_dir(project_name.as_ref())?
                    .parent()
                    .map_or_else(|| PathBuf::new(), |p| PathBuf::from(p));
                PathBuf::from(project_dir.join(filename).canonicalize()?)
            };

            template_content = fs::read_to_string(full_path)?;
            template_content.as_str()
        }
        ProjectTemplate::Default => {
            template.unwrap_or(include_str!("assets/default_template.tera"))
        }
    };

    let source = tera.render_str(template, &context)?;

    // Run tmux
    if show_source {
        println!("{}", source);
    } else {
        // Create dummy tmux session to make sure the tmux server is up and running
        let (tmux_command, tmux_args) = project.get_tmux_command(vec![
            OsString::from("new-session"),
            OsString::from("-s"),
            OsString::from("__rmux_dummy_session_"),
            OsString::from("-d"),
        ])?;
        let mut dummy_command = Command::new(tmux_command);
        dummy_command.args(tmux_args).output()?;

        // Source our tmux config file
        let (tmux_command, tmux_args) =
            project.get_tmux_command(vec![OsString::from("source"), OsString::from("-")])?;

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

        // Remove dummy session
        let (tmux_command, tmux_args) = project.get_tmux_command(vec![
            OsString::from("kill-session"),
            OsString::from("-t"),
            OsString::from("__rmux_dummy_session_"),
        ])?;
        let mut dummy_command = Command::new(tmux_command);
        let _ = dummy_command.args(tmux_args).output();

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
            let (tmux_command, tmux_args) = project.get_tmux_command(vec![
                OsString::from("attach-session"),
                OsString::from("-t"),
                OsString::from(session_name),
            ])?;
            Command::new(tmux_command).args(tmux_args).spawn()?.wait()?;
        }
    }

    Ok(())
}

pub fn kill_project<S: AsRef<OsStr>>(
    config: &Config,
    project_name: S,
    args: Vec<String>,
) -> Result<(), Box<dyn error::Error>> {
    let project = project::load(config, &project_name, None, &args)?;
    project.check()?;

    let session_name = project
        .session_name
        .to_owned()
        .ok_or(/* should never happen */ Error::SessionNameNotSet {})?;

    // Run tmux
    let (tmux_command, tmux_args) = project.get_tmux_command(vec![
        OsString::from("kill-session"),
        OsString::from("-t"),
        OsString::from(session_name.to_owned()),
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

pub fn edit_project<S1: AsRef<OsStr>, S2: AsRef<OsStr>>(
    config: &Config,
    project_name: S1,
    editor: S2,
    no_check: bool,
) -> Result<(), Box<dyn error::Error>> {
    let project_name = project_name.as_ref();
    ensure!(!project_name.is_empty(), ProjectNameEmpty {});

    let editor = editor.as_ref();
    ensure!(!editor.is_empty(), EditorEmpty {});

    // Make sure the project's parent directory exists
    let namespace = utils::get_project_namespace(project_name)?;
    let data_dir = config.get_projects_dir("")?;
    mkdirp(data_dir.join(&namespace))?;

    // Make sure the project's yml file exists
    let project_path = data_dir.join(project_name).with_extension("yml");
    ensure!(
        !project_path.is_dir(),
        ProjectFileIsADirectory { path: project_path }
    );
    if !project_path.exists() {
        edit::create_project(project_name, &project_path)?;
    }

    // Open it with editor
    let (command, args) = utils::parse_command(editor, &[OsString::from(project_path)])?;
    let mut child = Command::new(command).args(args).spawn()?;

    if !no_check {
        child.wait()?;

        // Perform a check on the project
        let project = project::load(config, project_name, None, &vec![])?;
        project.check()?;
    }

    Ok(())
}

pub fn remove_project<S: AsRef<OsStr>>(
    config: &Config,
    project_name: S,
    no_input: bool,
) -> Result<(), Box<dyn error::Error>> {
    let project_name = project_name.as_ref();
    ensure!(!project_name.is_empty(), ProjectNameEmpty {});

    let projects_dir = config.get_projects_dir("")?;
    let project_path = projects_dir.join(project_name).with_extension("yml");
    ensure!(project_path.is_file(), ProjectDoesNotExist { project_name });

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

    fs::remove_file(&project_path)?;
    for parent in project_path.ancestors() {
        if parent == projects_dir {
            break;
        }

        let _ = fs::remove_dir(parent);
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

mod project {
    use super::*;

    pub fn load<S: AsRef<OsStr>>(
        config: &Config,
        project_name: S,
        force_attach: Option<bool>,
        args: &[String],
    ) -> Result<Project, Box<dyn error::Error>> {
        let project_name = project_name.as_ref();
        ensure!(!project_name.is_empty(), ProjectNameEmpty {});

        let projects_dir = config.get_projects_dir("")?;
        let project_path = projects_dir.join(project_name).with_extension("yml");
        ensure!(project_path.is_file(), ProjectDoesNotExist { project_name });

        let project_yaml = fs::read_to_string(project_path)?;
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
}

mod source {
    use super::*;

    pub struct QuoteFilter;

    impl tera::Filter for QuoteFilter {
        fn filter(
            &self,
            value: &Value,
            _args: &HashMap<String, Value>,
        ) -> Result<Value, tera::Error> {
            let str_value = value.as_str().ok_or(tera::Error::msg(format!(
                "cannot quote {:?}: not a string",
                value
            )))?;

            Ok(json!(String::from(quote(str_value))))
        }
    }
}

mod edit {
    use super::*;

    pub fn create_project<S: AsRef<OsStr>, P: AsRef<Path>>(
        project_name: S,
        project_path: P,
    ) -> Result<(), Box<dyn error::Error>> {
        let project_name = project_name.as_ref();
        let project_path = project_path.as_ref();

        let default_project_yml = include_str!("assets/default_project.yml")
            .replace("__PROJECT_NAME__", &project_name.to_string_lossy());

        let mut file = fs::File::create(&project_path)?;
        file.write_all(default_project_yml.as_bytes())?;
        file.sync_all()?;

        Ok(())
    }
}

mod list {
    use super::*;

    pub fn get_projects<P: AsRef<Path>>(path: P) -> Result<Vec<OsString>, Box<dyn error::Error>> {
        let path = path.as_ref();
        let mut projects = vec![];

        for entry in path.read_dir()? {
            let entry = entry?;
            let entry_path = entry.path();

            if entry_path.is_file() {
                let file_path = entry_path.strip_prefix(path)?;
                projects.push(OsString::from(file_path.with_extension("")));
            } else if entry_path.is_dir() {
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

#[cfg(test)]
#[path = "test/actions.rs"]
mod tests;
