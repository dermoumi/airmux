use app_dirs::{get_app_root, AppDataType, AppInfo};
use clap::ArgMatches;
use mkdirp::mkdirp;
use snafu::{ensure, Snafu};
use std::error;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

const PROJECTS_SUBDIR: &'static str = "projects";

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("app_name cannot be empty"))]
    AppNameEmpty {},
    #[snafu(display("app_author cannot be empty"))]
    AppAuthorEmpty {},
    #[snafu(display("tmux command cannot be empty"))]
    TmuxCommandEmpty {},
    #[snafu(display("config-dir {:?} should be a directory", path))]
    ConfigDirIsNotADirectory { path: OsString }, // nocov
}

pub struct Config {
    pub app_name: &'static str,
    pub app_author: &'static str,
    pub tmux_command: OsString,
    pub config_dir: Option<PathBuf>,
}

impl Config {
    pub fn from_args(
        app_name: &'static str,
        app_author: &'static str,
        matches: &ArgMatches,
    ) -> Config {
        let tmux_command = match matches.value_of_os("tmux_command") {
            Some(command) => command.to_os_string(),
            _ => OsString::from("tmux"),
        };
        let config_dir = matches.value_of_os("config_dir").map(|x| PathBuf::from(x));

        Config {
            app_name,
            app_author,
            tmux_command,
            config_dir,
        }
    }

    pub fn check(self) -> Result<Self, Box<dyn error::Error>> {
        ensure!(!&self.app_name.is_empty(), AppNameEmpty {});
        ensure!(!&self.app_author.is_empty(), AppAuthorEmpty {});
        ensure!(!&self.tmux_command.is_empty(), TmuxCommandEmpty {});

        if let Some(config_dir) = &self.config_dir {
            let path = PathBuf::from(config_dir);
            ensure!(!path.is_file(), ConfigDirIsNotADirectory { path });

            mkdirp(&config_dir)?;
        };

        Ok(self)
    }

    pub fn get_config_dir<P: AsRef<Path>>(
        &self,
        sub_path: P,
    ) -> Result<PathBuf, Box<dyn error::Error>> {
        let path;
        if let Some(dir) = &self.config_dir {
            path = PathBuf::from(dir);
        } else {
            path = get_app_root(
                AppDataType::UserConfig,
                &AppInfo {
                    name: &self.app_name,
                    author: &self.app_author,
                },
            )?;
        };

        let path = path.join(&sub_path);
        mkdirp(&path)?;

        Ok(path)
    }

    pub fn get_projects_dir<P: AsRef<Path>>(
        &self,
        sub_path: P,
    ) -> Result<PathBuf, Box<dyn error::Error>> {
        let projects_path = PathBuf::from(PROJECTS_SUBDIR);

        self.get_config_dir(projects_path.join(sub_path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_dirs::AppDirsError;
    use clap::{crate_name, App, Arg};
    use std::env;
    use std::fs;
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
    fn from_args_matches_commands_correctly() {
        let tmux_command = "my_tmux";
        let config_dir = "my_config_dir";

        let app = App::new("test_app")
            .arg(Arg::with_name("tmux_command").short("t").takes_value(true))
            .arg(Arg::with_name("config_dir").short("c").takes_value(true));
        let matches = app.get_matches_from(vec!["rmux", "-t", tmux_command, "-c", config_dir]);

        let test_config = Config::from_args(APP_NAME, APP_AUTHOR, &matches);
        assert_eq!(test_config.tmux_command, tmux_command);
        assert_eq!(test_config.config_dir, Some(PathBuf::from(config_dir)));
    }

    #[test]
    fn from_args_matches_handles_missing_tmux_command() {
        let default_tmux_command = "tmux";
        let config_dir = "my_config_dir";

        let app = App::new("test_app")
            .arg(Arg::with_name("tmux_command").short("t").takes_value(true))
            .arg(Arg::with_name("config_dir").short("c").takes_value(true));
        let matches = app.get_matches_from(vec!["rmux", "-c", config_dir]);

        let test_config = Config::from_args(APP_NAME, APP_AUTHOR, &matches);
        assert_eq!(test_config.tmux_command, default_tmux_command);
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
    fn check_fails_when_tmux_command_is_empty() {
        let temp_dir = tempdir().unwrap();
        let temp_dir = temp_dir.path().to_path_buf();
        let test_config = make_config(None, None, Some(OsString::new()), Some(temp_dir));

        let result = test_config.check();
        assert!(result.is_err());
        assert!(matches!(
            result.err().unwrap().downcast_ref::<Error>(),
            Some(&Error::TmuxCommandEmpty {})
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

        let expected_path = temp_dir.join(PROJECTS_SUBDIR);
        let test_config = make_config(None, None, None, Some(temp_dir));

        let result = test_config.get_projects_dir("").unwrap();
        assert_eq!(expected_path, result);
    }

    #[test]
    fn get_projects_dir_returns_correct_subdir_path() {
        let temp_dir = tempdir().unwrap();
        let temp_dir = temp_dir.path().to_path_buf();

        let subdir = "my_subdir";
        let expected_path = temp_dir.join(PROJECTS_SUBDIR).join(subdir);
        let test_config = make_config(None, None, None, Some(temp_dir));

        let result = test_config.get_projects_dir(subdir).unwrap();
        assert_eq!(expected_path, result);
    }
}
