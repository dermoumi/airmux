use serde::de;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use std::error::Error;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct Project {
    #[serde(default, alias = "name")]
    pub session_name: Option<String>,
    #[serde(alias = "root")]
    pub working_dir: Option<PathBuf>,
    #[serde(default = "Project::default_window_base_index")]
    pub window_base_index: u32,
    #[serde(default = "Project::default_pane_base_index")]
    pub pane_base_index: u32,
    #[serde(default)]
    pub template: ProjectTemplate,
    pub windows: Vec<Window>,
}

impl Project {
    pub fn ensure_name(&mut self, project_name: &str) -> () {
        if self.session_name.is_none() {
            self.session_name = Some(project_name.into())
        }
    }

    pub fn default_window_base_index() -> u32 {
        1
    }

    pub fn default_pane_base_index() -> u32 {
        1
    }
}

#[derive(Serialize, Debug)]
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

#[derive(Serialize, Default, Debug)]
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
    pub fn from_value(val: &Value) -> Result<Self, Box<dyn Error>> {
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

    pub fn from_mapping(map: &serde_yaml::Mapping) -> Result<Self, Box<dyn Error>> {
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
            Some(x) => Some(match x.as_str() {
                Some(path) => path.into(),
                None => Err("expected working_dir to be a string")?,
            }),
            None => None,
        })
    }

    fn de_layout(val: Option<&Value>) -> Result<Option<String>, Box<dyn Error>> {
        Ok(match val {
            Some(layout) => match layout {
                w if w.is_string() => w.as_str().map(|x| x.into()),
                w if w.is_null() => None,
                _ => Err("expected layout to be a string")?,
            },
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

#[derive(Serialize, Default, Debug)]
pub struct Pane {
    #[serde(default)]
    pub working_dir: Option<PathBuf>,
    #[serde(default)]
    pub split: Option<PaneSplit>,
    #[serde(default)]
    pub split_from: Option<u64>,
    #[serde(default)]
    pub split_size: Option<String>,
    #[serde(default)]
    pub post_create: Vec<String>,
    #[serde(default)]
    pub commands: Vec<String>,
}

impl Pane {
    fn from_value(val: &Value) -> Result<Self, Box<dyn Error>> {
        match val.as_mapping() {
            Some(m) => Self::from_mapping(&m),
            None => Ok(Self {
                working_dir: None,
                split: None,
                split_from: None,
                split_size: None,
                post_create: vec![],
                commands: Self::de_commands_from_val(val)?,
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
            post_create: Self::de_post_create(map.get(&"post_create".into()))?,
            commands: Self::de_commands(
                map.get(&"commands".into())
                    .map_or_else(|| map.get(&"command".into()), Option::from),
            )?,
        })
    }

    fn de_working_dir(val: Option<&Value>) -> Result<Option<PathBuf>, Box<dyn Error>> {
        Ok(match val {
            Some(x) => Some(match x.as_str() {
                Some(path) => path.into(),
                None => Err("expected working_dir to be a string")?,
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

    fn de_split_from(val: Option<&Value>) -> Result<Option<u64>, Box<dyn Error>> {
        Ok(match val {
            Some(x) => Some(match x.as_u64() {
                None => Err("expected split_from to be a positive integer")?,
                Some(x) => x.into(),
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

    fn de_post_create(val: Option<&Value>) -> Result<Vec<String>, Box<dyn Error>> {
        Ok(match val {
            Some(x) => match x.as_sequence() {
                Some(x) => x
                    .into_iter()
                    .map(|x| serde_yaml::from_value(x.clone()))
                    .collect::<Result<Vec<_>, _>>()?,
                None => Err("expected post_create to be a list of strings")?,
            },
            None => vec![],
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
            _ => Err("expected commands to be a list of strings")?,
        })
    }
}

impl From<&str> for Pane {
    fn from(command: &str) -> Self {
        Self {
            working_dir: None,
            split: None,
            split_from: None,
            split_size: None,
            post_create: vec![],
            commands: vec![command.into()],
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

#[derive(Serialize, Deserialize, Debug)]
pub enum PaneSplit {
    #[serde(rename = "horizontal")]
    Horizontal,
    #[serde(rename = "vertical")]
    Vertical,
}
