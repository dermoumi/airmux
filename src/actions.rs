use crate::config;
use crate::data;
use crate::utils;

use config::Config;

use dialoguer::Confirm;
use mkdirp::mkdirp;
use serde_json::{json, value::Value};
use shell_words::quote;
use snafu::{ensure, Snafu};
use tera::{Context, Tera};

use std::collections::HashMap;
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
}

pub fn start_project<S: AsRef<OsStr>>(
    config: &Config,
    project_name: S,
    template: Option<&str>,
    force_attach: Option<bool>,
    show_source: bool,
) -> Result<(), Box<dyn error::Error>> {
    let tmux_command = &config.tmux_command;

    let project = project::load(config, project_name, force_attach)?;
    project.check()?;

    // Build and run tmux commands
    let mut context = Context::new();
    context.insert("tmux_command", &tmux_command.to_string_lossy());
    context.insert("project", &project);

    let mut tera = Tera::default();
    tera.register_filter("quote", source::QuoteFilter {});

    let template_content;
    let template = match project.template {
        data::ProjectTemplate::Raw(content) => {
            template_content = content;
            template_content.as_str()
        }
        data::ProjectTemplate::File(filename) => {
            template_content = fs::read_to_string(filename)?;
            template_content.as_str()
        }
        data::ProjectTemplate::Default => {
            template.unwrap_or(include_str!("assets/default_template.tera"))
        }
    };

    let source = tera.render_str(template, &context)?;

    // Run tmux
    if show_source {
        println!("{}", source);
    } else {
        let mut child = Command::new(tmux_command)
            .args(vec!["source", "-"])
            .stdin(Stdio::piped())
            .spawn()?;
        child
            .stdin
            .as_mut()
            .ok_or(Error::CannotPipeToTmux)?
            .write_all(source.as_bytes())?;

        // Wait until tmux completely finished processing input
        child.wait()?;

        // Attach
        if project.attach {
            Command::new(tmux_command)
                .args(vec!["attach-session", "-t", &project.session_name.unwrap()])
                .spawn()?
                .wait()?;
        }
    }

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
    let (command, args) = utils::parse_command(editor, &[project_path.as_os_str()])?;
    let mut child = Command::new(command).args(args).spawn()?;

    if !no_check {
        child.wait()?;

        // Perform a check on the project
        let project = project::load(config, project_name, None)?;
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
    ) -> Result<data::Project, Box<dyn error::Error>> {
        let project_name = project_name.as_ref();
        ensure!(!project_name.is_empty(), ProjectNameEmpty {});

        let projects_dir = config.get_projects_dir("")?;
        let project_path = projects_dir.join(project_name).with_extension("yml");
        ensure!(project_path.is_file(), ProjectDoesNotExist { project_name });

        let project_yaml = fs::read_to_string(project_path)?;

        Ok(serde_yaml::from_str::<data::Project>(&project_yaml)?
            .prepare(&project_name.to_string_lossy(), force_attach))
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
