use crate::config::Config;
use crate::utils::{parse_command, valid_tmux_identifier};

use serde::de;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use shell_words::{quote, split};

use std::error::Error;
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Project {
    #[serde(default, alias = "name")]
    pub session_name: Option<String>,
    #[serde(default)]
    pub tmux_command: Option<String>,
    #[serde(default)]
    pub tmux_options: Option<String>,
    #[serde(default, alias = "socket_name")]
    pub tmux_socket: Option<String>,
    #[serde(default, alias = "root", deserialize_with = "Project::de_working_dir")]
    pub working_dir: Option<PathBuf>,
    #[serde(default = "Project::default_window_base_index")]
    pub window_base_index: u32,
    #[serde(default = "Project::default_pane_base_index")]
    pub pane_base_index: u32,
    #[serde(default)]
    pub template: ProjectTemplate,
    #[serde(default = "Project::default_attach")]
    pub attach: bool,
    #[serde(
        default = "Project::default_windows",
        deserialize_with = "Project::de_windows",
        alias = "window"
    )]
    pub windows: Vec<Window>,
}

impl Project {
    pub fn prepare(self, config: &Config, project_name: &str, force_attach: Option<bool>) -> Self {
        let mut project = Self {
            session_name: self.session_name.or(Some(project_name.into())),
            ..self
        };

        if let Some(attach) = force_attach {
            project.attach = attach;
        }

        if let Some(tmux_command) = &config.tmux_command {
            project.tmux_command = Some(tmux_command.to_string_lossy().into());
        } else if project.tmux_command.is_none() {
            project.tmux_command = Some("tmux".into());
        }

        project
    }

    pub fn check(&self) -> Result<(), Box<dyn Error>> {
        if let Some(session_name) = &self.session_name {
            valid_tmux_identifier(session_name)?;
        }

        self.windows
            .iter()
            .map(|w| w.check())
            .collect::<Result<_, _>>()
    }

    // Makes sure that any arguments passed in tmux_command are instead added as arguments
    pub fn get_tmux_command(
        &self,
        args: Vec<OsString>,
    ) -> Result<(OsString, Vec<OsString>), Box<dyn Error>> {
        let command = OsString::from(self.tmux_command.as_ref().ok_or("tmux command not set")?);

        let socket_args: Vec<OsString> = match &self.tmux_socket {
            Some(tmux_socket) => vec![OsString::from("-L"), OsString::from(tmux_socket)],
            None => vec![],
        };

        let mut extra_args: Vec<OsString> = match &self.tmux_options {
            Some(tmux_options) => split(&tmux_options)?
                .into_iter()
                .map(|o| OsString::from(o))
                .collect(),
            None => vec![],
        };

        let mut full_args = socket_args;
        full_args.append(&mut extra_args);
        full_args.append(&mut args.into_iter().collect());

        parse_command(&command, &full_args)
    }

    // Sanitizes tmux_command for use in the template file
    pub fn get_tmux_command_for_template(&self) -> Result<String, Box<dyn Error>> {
        let (command, args) = self.get_tmux_command(vec![])?;

        Ok(format!(
            "{} {}",
            command.to_string_lossy(),
            args.iter()
                .map(|s| quote(&String::from(s.to_string_lossy())).into())
                .collect::<Vec<String>>()
                .join(" ")
        ))
    }

    fn default_window_base_index() -> u32 {
        1
    }

    fn default_pane_base_index() -> u32 {
        1
    }

    fn default_attach() -> bool {
        true
    }

    fn default_windows() -> Vec<Window> {
        vec![Window::default()]
    }

    fn de_working_dir<'de, D>(deserializer: D) -> Result<Option<PathBuf>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let val: Value = de::Deserialize::deserialize(deserializer)?;

        match val {
            p if p.is_string() => Ok(Some(p.as_str().unwrap().into())),
            p if p.is_null() => Ok(Some("~".into())),
            _ => Err("expected working_dir to be a string or null"),
        }
        .map_err(de::Error::custom)
    }

    fn de_windows<'de, D>(deserializer: D) -> Result<Vec<Window>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let val: Value = de::Deserialize::deserialize(deserializer)?;

        match val.as_sequence() {
            Some(seq) => Self::de_windows_from_sequence(seq).map_err(de::Error::custom),
            None => Ok(vec![Window::from_value(&val).map_err(de::Error::custom)?]),
        }
    }

    fn de_windows_from_sequence(seq: &serde_yaml::Sequence) -> Result<Vec<Window>, Box<dyn Error>> {
        seq.into_iter()
            .map(|val| Window::from_value(val))
            .collect::<Result<Vec<_>, _>>()
    }
}

impl Default for Project {
    fn default() -> Self {
        Self {
            session_name: None,
            tmux_command: None,
            tmux_options: None,
            tmux_socket: None,
            working_dir: None,
            window_base_index: Self::default_window_base_index(),
            pane_base_index: Self::default_pane_base_index(),
            template: ProjectTemplate::default(),
            attach: true,
            windows: vec![Window::default()],
        }
    }
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProjectTemplate {
    Raw(String),
    File(PathBuf),
    Default,
}

impl ProjectTemplate {
    fn from_value(val: &Value) -> Result<Self, Box<dyn Error>> {
        match val {
            v if v.is_null() => Ok(Self::Default),
            v if v.is_string() => Ok(v.as_str().unwrap().into()),
            v if v.is_mapping() => Self::from_mapping(v.as_mapping().unwrap()),
            v => Err(format!("invalid value for field 'template': {:?}", v).into()),
        }
    }

    fn from_mapping(map: &serde_yaml::Mapping) -> Result<Self, Box<dyn Error>> {
        match map.get(&"file".into()) {
            Some(filename) => match filename.as_str() {
                Some(path) => Ok(Self::File(path.into())),
                _ => Err("expected file to be a string".into()),
            },
            _ => Err("missing 'file' field".into()),
        }
    }
}

impl Default for ProjectTemplate {
    fn default() -> Self {
        ProjectTemplate::Default
    }
}

impl From<&str> for ProjectTemplate {
    fn from(content: &str) -> Self {
        Self::Raw(content.into())
    }
}

impl<'de> Deserialize<'de> for ProjectTemplate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let val: Value = de::Deserialize::deserialize(deserializer)?;

        Self::from_value(&val).map_err(de::Error::custom)
    }
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Window {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    #[serde(alias = "root")]
    pub working_dir: Option<PathBuf>,
    #[serde(default)]
    pub layout: Option<String>,
    #[serde(default)]
    pub panes: Vec<Pane>,
}

impl Window {
    pub fn check(&self) -> Result<(), Box<dyn Error>> {
        if let Some(name) = &self.name {
            valid_tmux_identifier(name)?;
        }

        for pane in &self.panes {
            pane.check()?;

            if let Some(split_from) = pane.split_from {
                if split_from >= self.panes.len() {
                    Err(format!(
                        "split_from: there is no pane with index {}",
                        split_from
                    ))?;
                }
            }
        }

        self.panes
            .iter()
            .map(|p| p.check())
            .collect::<Result<_, _>>()
    }

    fn from_value(val: &Value) -> Result<Self, Box<dyn Error>> {
        match val.as_mapping() {
            Some(map) => Window::from_mapping(map),
            None => Ok(Self {
                name: None,
                working_dir: None,
                panes: Self::de_panes_from_val(val)?,
                layout: None,
            }),
        }
    }

    fn from_mapping(map: &serde_yaml::Mapping) -> Result<Self, Box<dyn Error>> {
        if map.len() != 1 {
            Err("expected window definition to be a single-value hashmap")?;
        }

        let (name, definition) = map.iter().next().unwrap();

        Self::de_windef(Self::de_name(name)?, definition)
    }

    fn de_name(val: &Value) -> Result<Option<String>, Box<dyn Error>> {
        Ok(match val {
            n if n.is_string() => n.as_str().map(|x| x.into()),
            n if n.is_null() => None,
            _ => Err("expected window name to be a string")?,
        })
    }

    fn de_windef(name: Option<String>, definition: &Value) -> Result<Self, Box<dyn Error>> {
        match definition.as_mapping() {
            Some(map) => Self::de_windef_from_mapping(name, map),
            None => Ok(Self {
                name,
                working_dir: None,
                panes: Self::de_panes_from_val(definition)?,
                layout: None,
            }),
        }
    }

    fn de_windef_from_mapping(
        name: Option<String>,
        definition: &serde_yaml::Mapping,
    ) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            name,
            working_dir: Self::de_working_dir(
                definition
                    .get(&"working_dir".into())
                    .map_or_else(|| definition.get(&"root".into()), Option::from),
            )?,
            layout: Self::de_layout(definition.get(&"layout".into()))?,
            panes: Self::de_panes(definition.get(&"panes".into()))?,
        })
    }

    fn de_working_dir(val: Option<&Value>) -> Result<Option<PathBuf>, Box<dyn Error>> {
        Ok(match val {
            Some(x) => Some(match x {
                p if p.is_string() => p.as_str().unwrap().into(),
                p if p.is_null() => PathBuf::from("~"),
                _ => Err("expected working_dir to be a string or null")?,
            }),
            None => None,
        })
    }

    fn de_layout(val: Option<&Value>) -> Result<Option<String>, Box<dyn Error>> {
        Ok(match val {
            Some(layout) => Some(match layout.as_str() {
                Some(l) => l.into(),
                _ => Err("expected layout to be a string")?,
            }),
            None => None,
        })
    }

    fn de_panes(val: Option<&Value>) -> Result<Vec<Pane>, Box<dyn Error>> {
        match val {
            Some(panes) => Self::de_panes_from_val(panes),
            None => Ok(vec![]),
        }
    }

    fn de_panes_from_val(val: &Value) -> Result<Vec<Pane>, Box<dyn Error>> {
        match val.as_sequence() {
            Some(seq) => Self::de_panes_from_sequence(seq),
            None => Ok(vec![Pane::from_value(val)?]),
        }
    }

    fn de_panes_from_sequence(seq: &serde_yaml::Sequence) -> Result<Vec<Pane>, Box<dyn Error>> {
        seq.into_iter()
            .map(|val| Pane::from_value(val))
            .collect::<Result<Vec<_>, _>>()
    }
}

impl<'de> Deserialize<'de> for Window {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let val: Value = de::Deserialize::deserialize(deserializer)?;

        Self::from_value(&val).map_err(de::Error::custom)
    }
}

impl Default for Window {
    fn default() -> Self {
        Self {
            name: None,
            working_dir: None,
            layout: None,
            panes: vec![Pane::default()],
        }
    }
}

#[derive(Serialize, Default, Debug, PartialEq)]
pub struct Pane {
    #[serde(default)]
    pub working_dir: Option<PathBuf>,
    #[serde(default)]
    pub split: Option<PaneSplit>,
    #[serde(default)]
    pub split_from: Option<usize>,
    #[serde(default)]
    pub split_size: Option<String>,
    #[serde(default)]
    pub post_create: Vec<String>,
    #[serde(default)]
    pub commands: Vec<String>,
}

impl Pane {
    pub fn check(&self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn from_value(val: &Value) -> Result<Self, Box<dyn Error>> {
        match val.as_mapping() {
            Some(m) => Self::from_mapping(&m),
            None => Ok(Self {
                commands: Self::de_commands_from_val(val)?,
                ..Self::default()
            }),
        }
    }

    fn from_mapping(map: &serde_yaml::Mapping) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            working_dir: Self::de_working_dir(
                map.get(&"working_dir".into())
                    .map_or_else(|| map.get(&"root".into()), Option::from),
            )?,
            split: Self::de_split(map.get(&"split".into()))?,
            split_from: Self::de_split_from(map.get(&"split_from".into()))?,
            split_size: Self::de_split_size(map.get(&"split_size".into()))?,
            post_create: Self::de_commands(map.get(&"post_create".into()))?,
            commands: Self::de_commands(
                map.get(&"commands".into())
                    .map_or_else(|| map.get(&"command".into()), Option::from),
            )?,
        })
    }

    fn de_working_dir(val: Option<&Value>) -> Result<Option<PathBuf>, Box<dyn Error>> {
        Ok(match val {
            Some(x) => Some(match x {
                p if p.is_string() => p.as_str().unwrap().into(),
                p if p.is_null() => PathBuf::from("~"),
                _ => Err("expected working_dir to be a string or null")?,
            }),
            None => None,
        })
    }

    fn de_split(val: Option<&Value>) -> Result<Option<PaneSplit>, Box<dyn Error>> {
        Ok(match val {
            Some(x) => Some(match x.as_str() {
                Some(x) if ["v", "vertical"].contains(&x.to_lowercase().as_str()) => {
                    PaneSplit::Vertical
                }
                Some(x) if ["h", "horizontal"].contains(&x.to_lowercase().as_str()) => {
                    PaneSplit::Horizontal
                }
                _ => Err("expected split value to match v|h|vertical|horizontal")?,
            }),
            None => None,
        })
    }

    fn de_split_from(val: Option<&Value>) -> Result<Option<usize>, Box<dyn Error>> {
        Ok(match val {
            Some(x) => Some(match x.as_u64() {
                Some(x) => x as usize,
                None => Err("expected split_from to be a positive integer")?,
            }),
            None => None,
        })
    }

    fn de_split_size(val: Option<&Value>) -> Result<Option<String>, Box<dyn Error>> {
        Ok(match val {
            Some(x) => Some(match x {
                x if x.is_u64() => x.as_u64().unwrap().to_string(),
                x if x.is_string() => x.as_str().unwrap().into(),
                _ => Err("expected split_size to be either a positive integer or a string")?,
            }),
            None => None,
        })
    }

    fn de_commands(val: Option<&Value>) -> Result<Vec<String>, Box<dyn Error>> {
        match val {
            Some(x) => Self::de_commands_from_val(x),
            None => Ok(vec![]),
        }
    }

    fn de_commands_from_val(val: &Value) -> Result<Vec<String>, Box<dyn Error>> {
        Ok(match val {
            s if s.is_sequence() => s
                .as_sequence()
                .unwrap()
                .into_iter()
                .map(|x| serde_yaml::from_value(x.clone()))
                .collect::<Result<Vec<_>, _>>()?,
            s if s.is_string() => vec![s.as_str().unwrap().into()],
            n if n.is_null() => vec![],
            _ => Err("expected commands to be null, a string or a list of strings")?,
        })
    }
}

impl From<&str> for Pane {
    fn from(command: &str) -> Self {
        Self {
            commands: vec![command.into()],
            ..Self::default()
        }
    }
}

impl<'de> Deserialize<'de> for Pane {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let val: Value = de::Deserialize::deserialize(deserializer)?;

        Self::from_value(&val).map_err(de::Error::custom)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum PaneSplit {
    #[serde(rename = "horizontal")]
    Horizontal,
    #[serde(rename = "vertical")]
    Vertical,
}

#[cfg(test)]
#[path = "test/data.rs"]
mod tests;
