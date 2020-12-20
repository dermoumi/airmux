use shell_words::split;
use snafu::{ensure, Snafu};
use std::error;
use std::ffi::{OsStr, OsString};
use std::path;
use std::path::PathBuf;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Project name {:?} cannot not have a trailing slash", project_name))]
    ProjectNameTrailingSlash { project_name: OsString },
    #[snafu(display("Project name {:?} cannot not be an absolute path", project_name))]
    ProjectNameAbsolutePath { project_name: OsString },
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

pub fn get_project_namespace<S: AsRef<OsStr>>(
    project_name: S,
) -> Result<PathBuf, Box<dyn error::Error>> {
    let project_name = project_name.as_ref();

    let has_trailing_slash = project_name
        .to_string_lossy()
        .ends_with(path::MAIN_SEPARATOR);
    ensure!(
        !has_trailing_slash,
        ProjectNameTrailingSlash { project_name }
    );

    let path = PathBuf::from(project_name);
    ensure!(!path.has_root(), ProjectNameAbsolutePath { project_name });

    Ok(path.parent().unwrap().to_path_buf())
}

pub fn parse_command(
    command: &OsStr,
    args: &[OsString],
) -> Result<(OsString, Vec<OsString>), Box<dyn error::Error>> {
    ensure!(!command.is_empty(), EmptyCommand {});

    let mut command_parts = split(&command.to_string_lossy())?
        .into_iter()
        .map(OsString::from)
        .chain(args.iter().map(OsString::from));

    let new_command = command_parts.next().unwrap();
    let new_args: Vec<OsString> = command_parts.collect();
    Ok((new_command, new_args))
}

pub fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    t == &T::default()
}

#[cfg(test)]
#[path = "test/utils.rs"]
mod tests;
