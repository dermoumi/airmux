use shell_words::split;
use snafu::{ensure, Snafu};
use std::error;
use std::ffi::{OsStr, OsString};
use std::path;
use std::path::PathBuf;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Project name {:?} cannot not have a trailing slash", project_name))]
    ProjectNameTrailingSlash { project_name: OsString }, // nocov
    #[snafu(display("Project name {:?} cannot not be an absolute path", project_name))]
    ProjectNameAbsolutePath { project_name: OsString }, // nocov
    #[snafu(display("Command cannot be empty"))]
    EmptyCommand {},
}

pub fn get_project_namespace(project_name: &OsStr) -> Result<PathBuf, Box<dyn error::Error>> {
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
    args: &[&OsStr],
) -> Result<(OsString, Vec<OsString>), Box<dyn error::Error>> {
    ensure!(!command.is_empty(), EmptyCommand {});

    let mut command_parts = split(&command.to_string_lossy())?
        .into_iter()
        .map(|x| OsString::from(x))
        .chain(args.into_iter().map(|x| x.to_os_string()));

    let new_command = command_parts.next().unwrap();
    let new_args: Vec<OsString> = command_parts.collect();
    Ok((new_command, new_args))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_empty_namespace() {
        let expected_result = PathBuf::new();
        let project_name = OsString::from("project");

        let result = get_project_namespace(&project_name).unwrap();
        assert_eq!(result, expected_result);
    }

    #[test]
    fn parses_namespace() {
        let expected_result = PathBuf::from("namespace");
        let project_name = OsString::from("namespace/project");

        let result = get_project_namespace(&project_name).unwrap();
        assert_eq!(result, expected_result);
    }

    #[test]
    fn parses_multilevel_namspaces() {
        let expected_result = PathBuf::from("my/name/space");
        let project_name = OsString::from("my/name/space/project");

        let result = get_project_namespace(&project_name).unwrap();
        assert_eq!(result, expected_result);
    }

    #[test]
    fn fails_when_project_name_has_a_trailing_slash() {
        let name = OsString::from("project/");

        let result = get_project_namespace(&name);
        assert!(result.is_err());
        assert!(matches!(
            result.err().unwrap().downcast_ref::<Error>().unwrap(),
            Error::ProjectNameTrailingSlash { project_name } if *project_name == name
        ));
    }

    #[test]
    #[cfg(unix)]
    fn fails_when_project_name_is_an_absolute_path() {
        let name = OsString::from("/project");

        let result = get_project_namespace(&name);
        assert!(result.is_err());
        assert!(matches!(
            result.err().unwrap().downcast_ref::<Error>().unwrap(),
            Error::ProjectNameAbsolutePath { project_name } if *project_name == name
        ));
    }

    #[test]
    #[cfg(windows)]
    fn fails_when_project_name_is_an_absolute_path_windows() {
        let name = OsString::from("c:/project");

        let result = get_project_namespace(&name);
        assert!(result.is_err());
        assert!(matches!(
            result.err().unwrap().downcast_ref::<Error>().unwrap(),
            Error::ProjectNameAbsolutePath { project_name } if *project_name == name
        ));
    }

    #[test]
    fn correct_command_parses_single_command() {
        let expected_result = (OsString::from("cmd"), vec![]);
        let command = OsString::from("cmd");
        let args = &[];

        let result = parse_command(&command, args).unwrap();
        assert_eq!(result, expected_result);
    }

    #[test]
    fn correct_command_parses_command_with_flags_command() {
        let expected_result = (OsString::from("cmd"), vec![OsString::from("-flag")]);
        let command = OsString::from("cmd -flag");
        let args = &[];

        let result = parse_command(&command, args).unwrap();
        assert_eq!(result, expected_result);
    }

    #[test]
    fn correct_command_parses_returns_correct_arguments() {
        let expected_result = (
            OsString::from("cmd"),
            vec![OsString::from("-flag"), OsString::from("file")],
        );
        let command = OsString::from("cmd -flag");
        let arg1 = OsString::from("file");
        let args = &[arg1.as_os_str()];

        let result = parse_command(&command, args).unwrap();
        assert_eq!(result, expected_result);
    }

    #[test]
    fn correct_command_fails_on_empty_command() {
        let command = OsString::from("");
        let args = &[];

        let result = parse_command(&command, args);
        assert!(result.is_err());
        assert!(matches!(
            result.err().unwrap().downcast_ref::<Error>().unwrap(),
            Error::EmptyCommand {}
        ));
    }
}
