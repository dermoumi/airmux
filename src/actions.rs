use crate::utils;
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
    _: &OsStr,
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
    _: &OsStr,
    project_name: &OsStr,
    editor: &OsStr,
) -> Result<(), Box<dyn error::Error>> {
    // Make sure editor is not empty
    ensure!(!editor.is_empty(), EditorEmpty {});

    // Make sure the project's parent directory exists
    let data_dir = utils::get_data_dir(utils::APP_NAME, utils::APP_AUTHOR)?;
    let namespace = utils::get_project_namespace(project_name)?;
    let sub_dir_path = data_dir.join(namespace);
    mkdirp(sub_dir_path)?;

    // Make sure the project's yml file exists
    let mut project_path = data_dir.join(project_name);
    project_path.set_extension("yml");
    if !project_path.exists() {
        let default_project_yml = include_str!("config/default_project.yml")
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
    _: &OsStr,
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

pub fn list_projects() -> Result<(), Box<dyn error::Error>> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn edit_project_fails_when_editor_is_empty() {
        let tmux_command = OsString::from("tmux");
        let project_name = OsString::from("__rmux_test_project_edit0__");
        let editor = OsString::new();

        assert!(matches!(
            edit_project(&tmux_command, &project_name, &editor)
                .err()
                .unwrap()
                .downcast_ref::<Error>()
                .unwrap(),
            Error::EditorEmpty {}
        ))
    }

    #[test]
    fn edit_project_succeeds_when_project_file_does_not_exist() {
        let tmux_command = OsString::from("tmux");
        let project_name = OsString::from("__rmux_test_project_edit1__");
        let editor = OsString::from("test");

        // Make sure the file does not exist
        let mut project_path = utils::get_data_dir(utils::APP_NAME, utils::APP_AUTHOR)
            .unwrap()
            .join(&project_name);
        project_path.set_extension("yml");
        let _ = fs::remove_file(&project_path);
        let _ = fs::remove_dir_all(&project_path);

        assert!(!project_path.exists());

        // Run edit_project
        let result = edit_project(&tmux_command, &project_name, &editor);

        // Save file state and clean up
        let file_exists = project_path.is_file();
        let _ = fs::remove_file(project_path);

        // Assert
        assert!(file_exists);
        assert!(result.is_ok());
    }

    #[test]
    fn edit_project_succeeds_when_project_file_exists() {
        let tmux_command = OsString::from("tmux");
        let project_name = OsString::from("__rmux_test_project_edit2__");
        let editor = OsString::from("test");

        // Make sure the file exists
        let data_dir = utils::get_data_dir(utils::APP_NAME, utils::APP_AUTHOR).unwrap();
        let mut project_path = data_dir.join(&project_name);
        project_path.set_extension("yml");

        mkdirp(data_dir).unwrap();

        let default_project_yml = include_str!("config/default_project.yml")
            .replace("__PROJECT__", &project_name.to_string_lossy());
        let mut file = fs::File::create(&project_path).unwrap();
        file.write_all(default_project_yml.as_bytes()).unwrap();
        file.sync_all().unwrap();

        assert!(project_path.is_file());

        // Run edit_project
        let result = edit_project(&tmux_command, &project_name, &editor);

        // Save file state and clean up
        let file_exists = project_path.is_file();
        let _ = fs::remove_file(project_path);

        // Assert
        assert!(file_exists);
        assert!(result.is_ok());
    }
}
