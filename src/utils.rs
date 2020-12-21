use dialoguer::Confirm;
use shell_words::split;
use snafu::{ensure, Snafu};
use std::error;
use std::path;
use std::path::PathBuf;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Project name {:?} cannot not have a trailing slash", project_name))]
    ProjectNameTrailingSlash { project_name: String },
    #[snafu(display("Project name {:?} cannot not be an absolute path", project_name))]
    ProjectNameAbsolutePath { project_name: String },
    #[snafu(display("Command cannot be empty"))]
    EmptyCommand {},
    #[snafu(display("name {:?} cannot contain the following characters: .: ", identifier))]
    TmuxIdentifierIllegalCharacters { identifier: String },
    #[snafu(display("name cannot be empty"))]
    TmuxIdentifierEmpty {},
}

pub fn valid_tmux_identifier(identifier: &str) -> Result<(), Box<dyn error::Error>> {
    ensure!(
        identifier.find(&['.', ':'][..]).is_none(),
        TmuxIdentifierIllegalCharacters { identifier }
    );
    ensure!(!identifier.is_empty(), TmuxIdentifierEmpty {});

    Ok(())
}

pub fn get_project_namespace(project_name: &str) -> Result<PathBuf, Box<dyn error::Error>> {
    let has_trailing_slash = project_name.ends_with(path::MAIN_SEPARATOR);
    ensure!(
        !has_trailing_slash,
        ProjectNameTrailingSlash { project_name }
    );

    let path = PathBuf::from(project_name);
    ensure!(!path.has_root(), ProjectNameAbsolutePath { project_name });

    Ok(path.parent().unwrap().to_path_buf())
}

pub fn parse_command(
    command: &str,
    args: &[&str],
) -> Result<(String, Vec<String>), Box<dyn error::Error>> {
    ensure!(!command.is_empty(), EmptyCommand {});

    let args_iter = args.to_owned().into_iter().map(String::from);
    let mut command_parts = split(command)?.into_iter().chain(args_iter);

    let new_command = command_parts.next().unwrap();
    let new_args: Vec<String> = command_parts.collect();
    Ok((new_command, new_args))
}

pub fn is_default<T>(t: &T) -> bool
where
    T: Default + PartialEq,
{
    t == &T::default()
}

pub fn prompt_confirmation(message: &str, default: bool) -> Result<bool, Box<dyn error::Error>> {
    Ok(Confirm::new()
        .with_prompt(message)
        .default(default)
        .show_default(true)
        .interact()?)
}

#[cfg(test)]
#[path = "test/utils.rs"]
mod tests;
