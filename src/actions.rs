use crate::config;
use crate::utils;
use config::Config;
use mkdirp::mkdirp;
use snafu::{ensure, Snafu};
use std::error;
use std::ffi::OsStr;
use std::fs;
use std::io::prelude::*;
use std::process::Command;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("the EDITOR variable should not be empty"))]
    EditorEmpty {},
}

pub fn start_project(
    _: &Config,
    project_name: &OsStr,
    attach: bool,
) -> Result<(), Box<dyn error::Error>> {
    println!("Start {:?} and attaching? {:?}", project_name, attach);

    // Parse yaml file
    // Build tmux commands
    // Run tmux commands
    // Attach if requested

    Ok(()) // nocov
}

pub fn edit_project(
    config: &Config,
    project_name: &OsStr,
    editor: &OsStr,
) -> Result<(), Box<dyn error::Error>> {
    // Make sure editor is not empty
    ensure!(!editor.is_empty(), EditorEmpty {});

    // Make sure the project's parent directory exists
    let namespace = utils::get_project_namespace(project_name)?;
    let data_dir = config.get_projects_dir(&namespace)?;
    mkdirp(&data_dir)?;

    // Make sure the project's yml file exists
    let mut project_path = data_dir.join(project_name);
    project_path.set_extension("yml");
    if !project_path.exists() {
        let default_project_yml = include_str!("yaml/default_project.yml")
            .replace("__PROJECT__", &project_name.to_string_lossy());

        let mut file = fs::File::create(&project_path)?;
        file.write_all(default_project_yml.as_bytes())?;
        file.sync_all()?;
    }

    // Open it with editor
    let (command, args) = utils::parse_command(editor, &[project_path.as_os_str()])?;
    Command::new(command).args(args).output()?;

    // TODO: Perform a yaml check on the file

    Ok(())
}

pub fn remove_project(
    _: &Config,
    project_name: &OsStr,
    no_input: bool,
) -> Result<(), Box<dyn error::Error>> {
    println!("Remove {:?}. No input? {:?}", project_name, no_input);

    // Get project subdirectory
    // If project exists: Remove project file
    // Attempt to remove subdirectory, fail silently
    // If project does not exist; fail

    Ok(())
}

pub fn list_projects(_: &Config) -> Result<(), Box<dyn error::Error>> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::crate_name;
    use std::ffi::OsString;
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
        let temp_dir = tempdir().unwrap().path().to_path_buf();
        let test_config = make_config(None, None, None, Some(temp_dir));
        let project_name = OsString::from("__test_project_edit0__");
        let editor = OsString::new();

        assert!(matches!(
            edit_project(&test_config, &project_name, &editor)
                .err()
                .unwrap()
                .downcast_ref::<Error>()
                .unwrap(),
            Error::EditorEmpty {}
        ))
    }

    #[test]
    fn edit_project_succeeds_when_project_file_does_not_exist() {
        let temp_dir = tempdir().unwrap().path().to_path_buf();
        let test_config = make_config(None, None, None, Some(temp_dir));
        let project_name = OsString::from("__test_project_edit1__");
        let editor = OsString::from("test");

        // Make sure the file does not exist
        let project_path = test_config
            .get_projects_dir(&project_name)
            .unwrap()
            .with_extension("yml");

        // Run edit_project
        let result = edit_project(&test_config, &project_name, &editor);

        assert!(project_path.is_file());
        assert!(result.is_ok());
    }

    #[test]
    fn edit_project_succeeds_when_project_file_exists() {
        let temp_dir = tempdir().unwrap().path().to_path_buf();
        let test_config = make_config(None, None, None, Some(temp_dir));
        let project_name = OsString::from("__test_project_edit2__");
        let editor = OsString::from("test");

        // Make sure the file exists
        let projects_dir = test_config.get_projects_dir("").unwrap();
        let project_path = projects_dir.join(&project_name).with_extension("yml");

        mkdirp(projects_dir).unwrap();

        let mut file = fs::File::create(&project_path).unwrap();
        file.write_all(":D".as_bytes()).unwrap();
        file.sync_all().unwrap();

        assert!(project_path.is_file());

        // Run edit_project
        let result = edit_project(&test_config, &project_name, &editor);

        assert!(project_path.is_file());
        assert!(result.is_ok());
    }
}
