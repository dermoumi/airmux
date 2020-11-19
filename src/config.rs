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
#[path = "test/config.rs"]
mod tests;
