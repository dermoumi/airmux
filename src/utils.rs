use app_dirs::{get_app_root, AppDataType, AppInfo};
use clap::{crate_description, crate_name, crate_version};
use shell_words::split;
use snafu::{ensure, Snafu};
use std::error;
use std::ffi::{OsStr, OsString};
use std::path;
use std::path::PathBuf;

pub const APP_NAME: &'static str = crate_name!();
pub const APP_AUTHOR: &'static str = "dermoumi";
pub const APP_VERSION: &'static str = crate_version!();
pub const APP_DESCRIPTION: &'static str = crate_description!();

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Project name should not have a trailing slash"))]
    ProjectNameTrailingSlash {},
    #[snafu(display("Project name should not be an absolute path"))]
    ProjectNameAbsolutePath {},
    #[snafu(display("Command cannot be empty"))]
    EmptyCommand {},
}

pub fn get_data_dir(
    app_name: &'static str,
    app_author: &'static str,
) -> Result<PathBuf, Box<dyn error::Error>> {
    get_app_root(
        AppDataType::UserConfig,
        &AppInfo {
            name: &app_name,
            author: &app_author,
        },
    )
    .map_err(|x| x.into())
}

pub fn get_project_namespace(project_name: &OsStr) -> Result<PathBuf, Box<dyn error::Error>> {
    let has_trailing_slash = project_name
        .to_string_lossy()
        .ends_with(path::MAIN_SEPARATOR);
    ensure!(!has_trailing_slash, ProjectNameTrailingSlash {});

    let path = PathBuf::from(project_name);
    ensure!(!path.has_root(), ProjectNameAbsolutePath {});

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
    use app_dirs::AppDirsError;

    #[test]
    fn get_data_dir_works() {
        assert!(get_data_dir(APP_NAME, APP_AUTHOR).is_ok());
    }

    #[test]
    fn get_data_dir_wraps_errors_correctly() {
        assert!(matches!(
            get_data_dir("", "")
                .err()
                .unwrap()
                .downcast_ref::<AppDirsError>()
                .unwrap(),
            AppDirsError::InvalidAppInfo
        ));
    }

    #[test]
    fn parses_empty_namespace() {
        let expected_result = PathBuf::new();

        let project_name = OsString::from("project");

        assert_eq!(
            expected_result,
            get_project_namespace(&project_name).unwrap()
        );
    }

    #[test]
    fn parses_namespace() {
        let expected_result = PathBuf::from("namespace");

        let project_name = OsString::from("namespace/project");

        assert_eq!(
            expected_result,
            get_project_namespace(&project_name).unwrap()
        );
    }

    #[test]
    fn parses_multilevel_namspaces() {
        let expected_result = PathBuf::from("my/name/space");

        let project_name = OsString::from("my/name/space/project");

        assert_eq!(
            expected_result,
            get_project_namespace(&project_name).unwrap()
        );
    }

    #[test]
    fn fails_when_project_name_has_a_trailing_slash() {
        let project_name = OsString::from("project/");

        assert!(matches!(
            get_project_namespace(&project_name)
                .err()
                .unwrap()
                .downcast_ref::<Error>()
                .unwrap(),
            Error::ProjectNameTrailingSlash {}
        ));
    }

    #[test]
    #[cfg(unix)]
    fn fails_when_project_name_is_an_absolute_path() {
        let project_name = OsString::from("/project");
        assert!(matches!(
            get_project_namespace(&project_name)
                .err()
                .unwrap()
                .downcast_ref::<Error>()
                .unwrap(),
            Error::ProjectNameAbsolutePath {}
        ));
    }

    #[test]
    #[cfg(windows)]
    fn fails_when_project_name_is_an_absolute_path_windows() {
        let project_name = OsString::from("c:/project");
        assert!(matches!(
            get_project_namespace(&project_name)
                .err()
                .unwrap()
                .downcast_ref::<Error>()
                .unwrap(),
            Error::ProjectNameAbsolutePath {}
        ));
    }

    #[test]
    fn correct_command_parses_single_command() {
        let expected_result = (OsString::from("cmd"), vec![]);

        let command = OsString::from("cmd");
        let args = &[];

        assert_eq!(expected_result, parse_command(&command, args).unwrap());
    }

    #[test]
    fn correct_command_parses_command_with_flags_command() {
        let expected_result = (OsString::from("cmd"), vec![OsString::from("-flag")]);

        let command = OsString::from("cmd -flag");
        let args = &[];

        assert_eq!(expected_result, parse_command(&command, args).unwrap());
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

        assert_eq!(expected_result, parse_command(&command, args).unwrap());
    }

    #[test]
    fn correct_command_fails_on_empty_command() {
        let command = OsString::from("");
        let args = &[];

        assert!(matches!(
            parse_command(&command, args)
                .err()
                .unwrap()
                .downcast_ref::<Error>()
                .unwrap(),
            Error::EmptyCommand {}
        ));
    }
}
