use crate::config;
use crate::utils;

use config::Config;
use dialoguer::Confirm;
use mkdirp::mkdirp;
use snafu::{ensure, Snafu};

use std::error;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("editor cannot be empty"))]
    EditorEmpty {},
    #[snafu(display("project name cannot be empty"))]
    ProjectNameEmpty,
    #[snafu(display("project {:?} does not exist", project_name))]
    ProjectDoesNotExist { project_name: OsString }, // nocov
    #[snafu(display("project file {:?} is a directory", path))]
    ProjectFileIsADirectory { path: PathBuf }, // nocov
}

pub fn start_project<S: AsRef<OsStr>>(
    _: &Config,
    project_name: S,
    attach: bool,
) -> Result<(), Box<dyn error::Error>> {
    let project_name = project_name.as_ref();
    ensure!(!project_name.is_empty(), ProjectNameEmpty {});

    println!("Start {:?} and attaching? {:?}", project_name, attach);

    // Parse yaml file
    // Build and run tmux commands
    // Attach if requested

    Ok(()) // nocov
}

pub fn edit_project<S1: AsRef<OsStr>, S2: AsRef<OsStr>>(
    config: &Config,
    project_name: S1,
    editor: S2,
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
    child.wait()?;

    // TODO: Perform a yaml check on the file

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

mod edit {
    use super::*;

    pub fn create_project<S: AsRef<OsStr>, P: AsRef<Path>>(
        project_name: S,
        project_path: P,
    ) -> Result<(), Box<dyn error::Error>> {
        let project_name = project_name.as_ref();
        let project_path = project_path.as_ref();

        let default_project_yml = include_str!("default_project.yml")
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
