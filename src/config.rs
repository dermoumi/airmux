use crate::utils;

use app_dirs::{get_app_root, AppDataType, AppInfo};
use clap::ArgMatches;
use mkdirp::mkdirp;
use snafu::{ensure, Snafu};

use std::error;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("app_name cannot be empty"))]
    AppNameEmpty {},
    #[snafu(display("app_author cannot be empty"))]
    AppAuthorEmpty {},
    #[snafu(display("tmux command cannot be empty"))]
    TmuxCommandEmpty {},
    #[snafu(display("config-dir {:?} should be a directory", path))]
    ConfigDirIsNotADirectory { path: OsString },
}

pub struct Config {
    pub app_name: &'static str,
    pub app_author: &'static str,
    pub tmux_command: Option<String>,
    pub config_dir: Option<PathBuf>,
}

impl Config {
    pub fn from_args(
        app_name: &'static str,
        app_author: &'static str,
        matches: &ArgMatches,
    ) -> Config {
        let tmux_command = matches.value_of_lossy("tmux_command").map(String::from);
        let config_dir = matches.value_of_os("config_dir").map(PathBuf::from);

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
        self.get_config_dir(sub_path)
    }

    pub fn get_tmux_command(
        &self,
        args: &[&str],
    ) -> Result<(String, Vec<String>), Box<dyn error::Error>> {
        let command = self
            .tmux_command
            .to_owned()
            .map_or_else(|| String::from("tmux"), |cmd| cmd.to_owned());

        utils::parse_command(&command, args)
    }
}

#[cfg(test)]
#[path = "test/config.rs"]
mod tests;
