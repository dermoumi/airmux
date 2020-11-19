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
use std::path::Path;
use std::process::Command;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("editor cannot be empty"))]
    EditorEmpty {},
    #[snafu(display("project {:?} does not exist", project_name))]
    ProjectDoesNotExist { project_name: OsString },
}

pub fn start_project<S: AsRef<OsStr>>(
    _: &Config,
    project_name: S,
    attach: bool,
) -> Result<(), Box<dyn error::Error>> {
    let project_name = project_name.as_ref();

    println!("Start {:?} and attaching? {:?}", project_name, attach);

    // Parse yaml file
    // Build tmux commands
    // Run tmux commands
    // Attach if requested

    Ok(()) // nocov
}

pub fn edit_project<S1: AsRef<OsStr>, S2: AsRef<OsStr>>(
    config: &Config,
    project_name: S1,
    editor: S2,
) -> Result<(), Box<dyn error::Error>> {
    let project_name = project_name.as_ref();
    let editor = editor.as_ref();

    // Make sure editor is not empty
    ensure!(!editor.is_empty(), EditorEmpty {});

    // Make sure the project's parent directory exists
    let namespace = utils::get_project_namespace(project_name)?;
    let data_dir = config.get_projects_dir("")?;
    mkdirp(data_dir.join(&namespace))?;

    // Make sure the project's yml file exists
    let project_path = data_dir.join(project_name).with_extension("yml");
    if !project_path.exists() {
        edit::create_project(project_name, &project_path)?;
    }

    // Open it with editor
    let (command, args) = utils::parse_command(editor, &[project_path.as_os_str()])?;
    Command::new(command).args(args).output()?;

    // TODO: Perform a yaml check on the file

    Ok(())
}

pub fn remove_project<S: AsRef<OsStr>>(
    config: &Config,
    project_name: S,
    no_input: bool,
) -> Result<(), Box<dyn error::Error>> {
    let project_name = project_name.as_ref();

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
        return Ok(());
    }

    fs::remove_file(&project_path)?;
    for parent in project_path.ancestors() {
        if parent == projects_dir {
            break;
        }

        let _ = fs::remove_dir(parent);
    }

    Ok(())
}

pub fn list_projects(config: &Config) -> Result<(), Box<dyn error::Error>> {
    let data_dir = config.get_projects_dir("")?;

    println!(
        "{}",
        list::get_projects(data_dir)?
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

        let default_project_yml = include_str!("yaml/default_project.yml")
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
mod tests {
    use super::*;
    use clap::crate_name;
    use std::ffi::OsString;
    use std::os;
    use std::path::PathBuf;
    use tempfile::tempdir;

    const APP_NAME: &'static str = crate_name!();
    const APP_AUTHOR: &'static str = "dermoumi";

    fn make_config(
        app_name: Option<&'static str>,
        app_author: Option<&'static str>,
        tmux_command: Option<OsString>,
        config_dir: Option<PathBuf>,
    ) -> Config {
        Config {
            app_name: app_name.unwrap_or(APP_NAME),
            app_author: app_author.unwrap_or(APP_AUTHOR),
            tmux_command: tmux_command.unwrap_or(OsString::from("tmux")),
            config_dir,
        }
    }

    #[test]
    fn edit_project_fails_when_editor_is_empty() {
        let temp_dir = tempdir().unwrap();
        let temp_dir = temp_dir.path().to_path_buf();
        let test_config = make_config(None, None, None, Some(temp_dir));

        assert!(matches!(
            edit_project(&test_config, "project", "")
                .err()
                .unwrap()
                .downcast_ref::<Error>()
                .unwrap(),
            Error::EditorEmpty {}
        ))
    }

    #[test]
    fn edit_project_succeeds_when_project_file_does_not_exist() {
        let temp_dir = tempdir().unwrap();
        let temp_dir = temp_dir.path().to_path_buf();
        let test_config = make_config(None, None, None, Some(temp_dir));
        let project_name = "project";
        let project_path = test_config
            .get_projects_dir(project_name)
            .unwrap()
            .with_extension("yml");

        let result = edit_project(&test_config, project_name, "test");

        assert!(project_path.is_file());
        assert!(result.is_ok());
    }

    #[test]
    fn edit_project_succeeds_when_project_file_exists() {
        let temp_dir = tempdir().unwrap();
        let temp_dir = temp_dir.path().to_path_buf();
        let test_config = make_config(None, None, None, Some(temp_dir));
        let project_name = "project";

        // Make sure the file exists
        let projects_dir = test_config.get_projects_dir("").unwrap();
        let project_path = projects_dir.join(project_name).with_extension("yml");
        mkdirp(projects_dir).unwrap();
        edit::create_project(project_name, &project_path).unwrap();
        assert!(project_path.is_file());

        // Run edit_project
        let result = edit_project(&test_config, project_name, "test");

        assert!(project_path.is_file());
        assert!(result.is_ok());
    }

    #[test]
    fn edit_project_creates_sub_directories_as_needed() {
        let temp_dir = tempdir().unwrap();
        let temp_dir = temp_dir.path().to_path_buf();
        let test_config = make_config(None, None, None, Some(temp_dir));
        let project_name = "subdir1/subdir2/project";
        let project_path = test_config
            .get_projects_dir(project_name)
            .unwrap()
            .with_extension("yml");
        let subdir_path = test_config.get_projects_dir("subdir1/subdir2").unwrap();

        let result = edit_project(&test_config, project_name, "test");

        assert!(subdir_path.is_dir());
        assert!(project_path.is_file());
        assert!(result.is_ok());
    }

    #[test]
    fn remove_project_removes_existing_project() {
        let temp_dir = tempdir().unwrap();
        let temp_dir = temp_dir.path().to_path_buf();
        let test_config = make_config(None, None, None, Some(temp_dir));
        let project_name = "project";

        // Make sure the file exists
        let projects_dir = test_config.get_projects_dir("").unwrap();
        let project_path = projects_dir.join(project_name).with_extension("yml");
        mkdirp(projects_dir).unwrap();
        edit::create_project(project_name, &project_path).unwrap();
        assert!(project_path.is_file());

        let result = remove_project(&test_config, project_name, true);
        assert!(result.is_ok());
        assert!(!project_path.exists());
    }

    #[test]
    fn remove_project_removes_parent_subdirectories_if_empty() {
        let temp_dir = tempdir().unwrap();
        let temp_dir = temp_dir.path().to_path_buf();
        let test_config = make_config(None, None, None, Some(temp_dir));
        let project_name = "subdir1/subdir2/project";

        // Make sure the project's parent directory exists
        let namespace = utils::get_project_namespace(project_name).unwrap();
        let data_dir = test_config.get_projects_dir("").unwrap();
        mkdirp(data_dir.join(&namespace)).unwrap();

        // Make sure the file exists
        let projects_dir = test_config.get_projects_dir("").unwrap();
        let project_path = projects_dir.join(project_name).with_extension("yml");
        edit::create_project(project_name, &project_path).unwrap();
        assert!(project_path.is_file());

        let result = remove_project(&test_config, project_name, true);
        assert!(result.is_ok());
        assert!(!project_path.exists());
        assert!(!project_path.parent().unwrap().exists());
        assert!(!project_path.parent().unwrap().parent().unwrap().exists());
    }

    #[test]
    fn remove_project_does_not_remove_parent_subdirs_if_not_empty() {
        let temp_dir = tempdir().unwrap();
        let temp_dir = temp_dir.path().to_path_buf();
        let test_config = make_config(None, None, None, Some(temp_dir));
        let project1_name = "subdir1/subdir2/project1";
        let project2_name = "subdir1/project2";

        // Make sure the project's parent directory exists
        let namespace = utils::get_project_namespace(project1_name).unwrap();
        let data_dir = test_config.get_projects_dir("").unwrap();
        mkdirp(data_dir.join(&namespace)).unwrap();

        // Make sure the file exists
        let projects_dir = test_config.get_projects_dir("").unwrap();
        let project1_path = projects_dir.join(project1_name).with_extension("yml");
        edit::create_project(project1_name, &project1_path).unwrap();
        assert!(project1_path.is_file());
        let project2_path = projects_dir.join(project2_name).with_extension("yml");
        edit::create_project(project2_name, &project2_path).unwrap();
        assert!(project2_path.is_file());

        let result = remove_project(&test_config, project1_name, true);
        assert!(result.is_ok());
        assert!(!project1_path.exists());
        assert!(!project1_path.parent().unwrap().exists());
        assert!(project1_path.parent().unwrap().parent().unwrap().exists());
    }

    #[test]
    fn remove_project_fails_if_project_does_not_exist() {
        let temp_dir = tempdir().unwrap();
        let temp_dir = temp_dir.path().to_path_buf();
        let test_config = make_config(None, None, None, Some(temp_dir));
        let project1_name = "project";

        let result = remove_project(&test_config, project1_name, true);
        assert!(result.is_err());
        assert!(matches!(
            result.err().unwrap().downcast_ref::<Error>().unwrap(),
            Error::ProjectDoesNotExist { project_name } if project_name == project1_name
        ));
    }

    #[test]
    fn list_project_does_not_fail() {
        let temp_dir = tempdir().unwrap();
        let temp_dir = temp_dir.path().to_path_buf();
        let test_config = make_config(None, None, None, Some(temp_dir));

        list_projects(&test_config).unwrap();
    }

    #[test]
    fn get_project_list_returns_projects_without_extensions() {
        let temp_dir = tempdir().unwrap();
        let temp_dir = temp_dir.path().to_path_buf();

        let mut expected_project_list = Vec::with_capacity(5);
        for n in 0..5 {
            let project_name = OsString::from(format!("project{}", n));

            edit::create_project(&project_name, temp_dir.join(&project_name)).unwrap();
            expected_project_list.push(project_name);
        }
        expected_project_list.sort();

        let mut project_list = list::get_projects(&temp_dir).unwrap();
        project_list.sort();

        assert_eq!(project_list, expected_project_list);
    }

    #[test]
    fn list_shows_projects_in_subdirectories() {
        let temp_dir = tempdir().unwrap();
        let temp_dir = temp_dir.path().to_path_buf();

        let mut expected_project_list = Vec::with_capacity(4);
        for n in 0..2 {
            let project_name = OsString::from(format!("project{}", n));

            edit::create_project(&project_name, temp_dir.join(&project_name)).unwrap();
            expected_project_list.push(project_name);
        }
        for n in 2..4 {
            let project_name = OsString::from(format!("subdir1/project{}", n));
            mkdirp(temp_dir.join("subdir1")).unwrap();

            edit::create_project(&project_name, temp_dir.join(&project_name)).unwrap();
            expected_project_list.push(project_name);
        }
        for n in 4..6 {
            let project_name = OsString::from(format!("subdir2/project{}", n));
            mkdirp(temp_dir.join("subdir2")).unwrap();

            edit::create_project(&project_name, temp_dir.join(&project_name)).unwrap();
            expected_project_list.push(project_name);
        }
        expected_project_list.sort();

        let mut project_list = list::get_projects(&temp_dir).unwrap();
        project_list.sort();

        assert_eq!(project_list, expected_project_list);
    }

    #[test]
    fn list_follows_symlinks() {
        let temp_dir = tempdir().unwrap();
        let temp_dir = temp_dir.path().to_path_buf();

        let mut expected_project_list = Vec::with_capacity(4);
        for n in 0..2 {
            let project_name = OsString::from(format!("project{}", n));

            edit::create_project(&project_name, temp_dir.join(&project_name)).unwrap();
            expected_project_list.push(project_name);
        }
        for n in 2..4 {
            let project_name = OsString::from(format!("subdir1/project{}", n));
            mkdirp(temp_dir.join("subdir1")).unwrap();

            edit::create_project(&project_name, temp_dir.join(&project_name)).unwrap();
            expected_project_list.push(project_name);
        }
        for n in 2..4 {
            let project_name = OsString::from(format!("subdir2/project{}", n));

            expected_project_list.push(project_name);
        }
        expected_project_list.sort();

        #[cfg(windows)]
        os::windows::fs::symlink_dir(temp_dir.join("subdir1"), temp_dir.join("subdir2")).unwrap();
        #[cfg(unix)]
        os::unix::fs::symlink(temp_dir.join("subdir1"), temp_dir.join("subdir2")).unwrap();
        assert!(temp_dir.join("subdir2").is_dir());

        let mut project_list = list::get_projects(&temp_dir).unwrap();
        project_list.sort();

        assert_eq!(project_list, expected_project_list);
    }

    #[test]
    fn list_detects_symlink_loops() {
        let temp_dir = tempdir().unwrap();
        let temp_dir = temp_dir.path().to_path_buf();

        let mut expected_project_list = Vec::with_capacity(4);
        for n in 0..2 {
            let project_name = OsString::from(format!("project{}", n));

            edit::create_project(&project_name, temp_dir.join(&project_name)).unwrap();
            expected_project_list.push(project_name);
        }
        for n in 2..4 {
            let project_name = OsString::from(format!("subdir1/project{}", n));
            mkdirp(temp_dir.join("subdir1")).unwrap();

            edit::create_project(&project_name, temp_dir.join(&project_name)).unwrap();
            expected_project_list.push(project_name);
        }
        expected_project_list.sort();

        #[cfg(windows)]
        os::windows::fs::symlink_dir(&temp_dir, temp_dir.join("subdir2")).unwrap();
        #[cfg(unix)]
        os::unix::fs::symlink(&temp_dir, temp_dir.join("subdir2")).unwrap();
        assert!(temp_dir.join("subdir2").is_dir());

        let mut project_list = list::get_projects(&temp_dir).unwrap();
        project_list.sort();

        assert_eq!(project_list, expected_project_list);
    }
}
