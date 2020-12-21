use super::*;
use app_dirs::AppDirsError;
use clap::{App, Arg};
use std::fs;
use tempfile::tempdir;

const APP_NAME: &str = "test_app_name";
const APP_AUTHOR: &str = "test_app_author";

fn make_config(
    app_name: Option<&'static str>,
    app_author: Option<&'static str>,
    tmux_command: Option<&str>,
    config_dir: Option<PathBuf>,
) -> Config {
    Config {
        app_name: app_name.unwrap_or(APP_NAME),
        app_author: app_author.unwrap_or(APP_AUTHOR),
        tmux_command: Some(String::from(tmux_command.unwrap_or("tmux"))),
        config_dir,
    }
}

#[test]
fn from_args_matches_commands_correctly() {
    let tmux_command = "my_tmux";
    let config_dir = "my_config_dir";

    let app = App::new("test_app")
        .arg(Arg::with_name("tmux_command").short("t").takes_value(true))
        .arg(Arg::with_name("config_dir").short("c").takes_value(true));
    let matches = app.get_matches_from(vec!["rmux", "-t", tmux_command, "-c", config_dir]);

    let test_config = Config::from_args(APP_NAME, APP_AUTHOR, &matches);
    assert_eq!(test_config.tmux_command, Some(tmux_command.into()));
    assert_eq!(test_config.config_dir, Some(PathBuf::from(config_dir)));
}

#[test]
fn check_fails_when_app_name_is_empty() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();
    let test_config = make_config(Some(""), None, None, Some(temp_dir));

    let result = test_config.check();
    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap().downcast_ref::<Error>(),
        Some(&Error::AppNameEmpty {})
    ));
}

#[test]
fn check_fails_when_author_name_is_empty() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();
    let test_config = make_config(None, Some(""), None, Some(temp_dir));

    let result = test_config.check();
    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap().downcast_ref::<Error>(),
        Some(&Error::AppAuthorEmpty {})
    ));
}

#[test]
fn check_fails_when_config_dir_is_a_file() {
    let temp_dir = tempdir().unwrap();
    let temp_file_path = temp_dir.path().join("file");

    let file = fs::File::create(&temp_file_path).unwrap();
    file.sync_all().unwrap();
    drop(file);

    assert!(temp_file_path.is_file());
    let test_config = make_config(None, None, None, Some(temp_file_path.clone()));

    let result = test_config.check();
    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap().downcast_ref::<Error>().unwrap(),
        Error::ConfigDirIsNotADirectory { path } if path == temp_file_path.as_os_str()
    ));
}

#[test]
fn check_attemps_to_make_the_directory() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().join("config");

    let test_config = make_config(None, None, None, Some(temp_dir.clone()));
    assert!(!temp_dir.exists());

    let result = test_config.check();
    assert!(result.is_ok());
    assert!(temp_dir.is_dir());
}

#[test]
fn get_config_dir_fails_if_app_name_is_empty_and_config_dir_is_none() {
    let test_config = make_config(Some(""), None, None, None);

    let result = test_config.get_config_dir("");
    assert!(result.is_err());
    assert!(matches!(
        result
            .err()
            .unwrap()
            .downcast_ref::<AppDirsError>()
            .unwrap(),
        AppDirsError::InvalidAppInfo
    ));
}

#[test]
fn get_config_dir_returns_correct_path() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();
    let test_config = make_config(None, None, None, Some(temp_dir.clone()));

    let result = test_config.get_config_dir("").unwrap();
    assert_eq!(temp_dir, result);
}

#[test]
fn get_config_dir_returns_correct_subdir_path() {
    let subdir = "my_subdir";
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();

    let expected_path = temp_dir.join(subdir);
    let test_config = make_config(None, None, None, Some(temp_dir));

    let result = test_config.get_config_dir(subdir).unwrap();
    assert_eq!(expected_path, result);
}

#[test]
fn get_projects_dir_returns_correct_path() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();

    let expected_path = temp_dir.to_owned();
    let test_config = make_config(None, None, None, Some(temp_dir));

    let result = test_config.get_projects_dir("").unwrap();
    assert_eq!(expected_path, result);
}

#[test]
fn get_projects_dir_returns_correct_subdir_path() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();

    let subdir = "my_subdir";
    let expected_path = temp_dir.join(subdir);
    let test_config = make_config(None, None, None, Some(temp_dir));

    let result = test_config.get_projects_dir(subdir).unwrap();
    assert_eq!(expected_path, result);
}

#[test]
fn get_tmux_command_splits_commands_correctly() {
    let test_config = make_config(None, None, Some("tmuxor -o1 option1"), None);

    let (command, args) = test_config.get_tmux_command(&["-o2", "option2"]).unwrap();

    assert_eq!(command, "tmuxor");
    assert_eq!(
        args,
        vec![
            String::from("-o1"),
            String::from("option1"),
            String::from("-o2"),
            String::from("option2"),
        ],
    );
}
