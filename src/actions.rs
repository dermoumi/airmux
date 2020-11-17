use crate::utils;
use mkdirp::mkdirp;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::File;
use std::io::prelude::*;
use std::process::Command;

pub fn start_project(_: &OsStr, project_name: &OsStr, attach: bool) -> Result<(), Box<dyn Error>> {
    println!("Start {:?} and attaching? {:?}", project_name, attach);

    // Parse yaml file
    // Build tmux commands
    // Run tmux commands
    // Attach if requested

    Ok(())
}

pub fn edit_project(_: &OsStr, project_name: &OsStr, editor: &OsStr) -> Result<(), Box<dyn Error>> {
    // Make sure editor is not empty
    if editor.is_empty() {
        return Err("the EDITOR variable should not be empty".into());
    }

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

        let mut file = File::create(&project_path)?;
        file.write_all(default_project_yml.as_bytes())?;
        file.sync_data()?;
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
) -> Result<(), Box<dyn Error>> {
    println!("Remove {:?}. No input? {:?}", project_name, no_input);

    // Get project subdirectory
    // If project exists: Remove project file
    // Attempt to remove subdirectory, fail silently
    // If project does not exist; fail

    Ok(())
}
