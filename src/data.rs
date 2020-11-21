use serde::de;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum ProjectTemplate {
    Raw(String),
    File(PathBuf),
    Default,
}

impl Default for ProjectTemplate {
    fn default() -> Self {
        ProjectTemplate::Default
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Project {
    #[serde(default)]
    pub session_name: Option<String>,
    #[serde(default = "defaults::window_base_index")]
    pub window_base_index: u32,
    #[serde(default = "defaults::pane_base_index")]
    pub pane_base_index: u32,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize::template_field")]
    pub template: ProjectTemplate,
    pub windows: Vec<Window>,
}

impl Project {
    pub fn ensure_name(&mut self, project_name: &str) -> () {
        if self.session_name.is_none() {
            self.session_name = Some(project_name.into())
        }
    }
}

#[derive(Serialize, Debug)]
pub struct Window {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub working_dir: Option<PathBuf>,
    #[serde(default)]
    pub panes: Vec<Pane>,
}

impl<'de> Deserialize<'de> for Window {
    fn deserialize<D>(deserializer: D) -> Result<Window, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let val: Value = de::Deserialize::deserialize(deserializer)?;
        if !val.is_mapping() {
            return Err(de::Error::custom(
                "expected window definition to be a single-value hashmap",
            ));
        }

        let map = val.as_mapping().unwrap();
        if map.len() != 1 {
            return Err(de::Error::custom(
                "expected window definition to be a single-value hashmap",
            ));
        }

        let (window_name, window_definition) = map.iter().next().unwrap();

        let name = match window_name {
            n if n.is_string() => Ok(n.as_str().map(|x| x.into())),
            n if n.is_null() => Ok(None),
            _ => Err(de::Error::custom("expected window name to be a string")),
        }?;

        let working_dir = match window_definition.as_mapping() {
            Some(content) => Ok(content
                .get(&"working_dir".into())
                .map_or_else(|| content.get(&"root".into()), |x| Some(x))
                .map_or(Ok(None), |working_dir| match working_dir {
                    w if w.is_string() => Ok(w.as_str().map(|x| x.into())),
                    w if w.is_null() => Ok(None),
                    _ => Err(de::Error::custom("expected working_dir to be a string")),
                })?),
            _ => Ok(None),
        }?;

        let panes = match window_definition {
            d if d.is_mapping() => match d.as_mapping().unwrap().get(&"panes".into()) {
                None => Ok(vec![]),
                Some(panes) => panes
                    .as_sequence()
                    .ok_or(de::Error::custom("expected panel list"))?
                    .into_iter()
                    .map(|x| serde_yaml::from_value(x.clone()))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(de::Error::custom),
            },
            d if d.is_sequence() => d
                .as_sequence()
                .unwrap()
                .into_iter()
                .map(|pane| serde_yaml::from_value(pane.clone()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(de::Error::custom),
            d if d.is_null() => Ok(vec![]),
            _ => Err(de::Error::custom("expected pane list")),
        }?;

        return Ok(Window {
            name,
            working_dir,
            panes,
        });
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum PaneSplit {
    Horizontal,
    Vertical,
}

#[derive(Serialize, Debug)]
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
    pub commands: Vec<String>,
    #[serde(default)]
    pub post_create: Vec<String>,
}

impl<'de> Deserialize<'de> for Pane {
    fn deserialize<D>(deserializer: D) -> Result<Pane, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let val: Value = de::Deserialize::deserialize(deserializer)?;

        match val {
            v if v.is_string() => Ok(Pane {
                working_dir: None,
                split: None,
                split_from: None,
                split_size: None,
                commands: vec![v.as_str().unwrap().into()],
                post_create: vec![],
            }),
            v if v.is_mapping() => {
                Ok({
                    let map = v.as_mapping().unwrap();

                    Pane {
                        working_dir: match map.get(&"working_dir".into()) {
                            None => None,
                            Some(x) => Some(match x.as_str() {
                                None => Err(de::Error::custom("expected working_dir to be a string")),
                                Some(path) => Ok(path.into()),
                            }?),
                        },
                        split: match map.get(&"split".into()) {
                            None => None,
                            Some(x) => Some(match x.as_str() {
                                Some(x) if ["v", "vertical"].contains(&x.to_lowercase().as_str()) => {
                                    Ok(PaneSplit::Vertical)
                                }
                                Some(x) if ["h", "horizontal"].contains(&x.to_lowercase().as_str()) => {
                                    Ok(PaneSplit::Horizontal)
                                }
                                _ => Err(de::Error::custom(
                                    "expected split value to match v|h|vertical|horizontal",
                                )),
                            }?),
                        },
                        split_from: match map.get(&"split_from".into()) {
                            None => None,
                            Some(x) => Some(
                                match x.as_u64() {
                                    Some(x) => Ok(x),
                                    _ => Err(de::Error::custom(
                                        "expected split_from to be a positive integer",
                                    )),
                                }?
                                .into(),
                            ),
                        },
                        split_size: match map.get(&"split_size".into()) {
                            None => None,
                            Some(x) => Some(match x {
                                x if x.is_string() => Ok(x.as_str().unwrap().into()),
                                x if x.is_u64() => Ok(format!("{}", x.as_u64().unwrap())),
                                _ => Err(de::Error::custom(
                                    "expected split_size to be either a positive integer or a string"
                                )),
                            }?),
                        },
                        commands: match map.get(&"commands".into()) {
                            None => vec![],
                            Some(x) => match x.as_sequence() {
                                Some(x) => x.into_iter()
                                    .map(|x| serde_yaml::from_value(x.clone()))
                                    .collect::<Result<Vec<_>,_>>()
                                    .map_err(de::Error::custom),
                                _ => Err(de::Error::custom("expected commands to be a list of strings")),
                            }?,
                        },
                        post_create: match map.get(&"post_create".into()) {
                            None => vec![],
                            Some(x) => match x.as_sequence() {
                                Some(x) => x.into_iter()
                                    .map(|x| serde_yaml::from_value(x.clone()))
                                    .collect::<Result<Vec<_>,_>>()
                                    .map_err(de::Error::custom),
                                _ => Err(de::Error::custom("expected post_create to be a list of strings")),
                            }?,
                        },
                    }
                })
            }
            v if v.is_sequence() => Ok(Pane {
                working_dir: None,
                split: None,
                split_from: None,
                split_size: None,
                commands: v
                    .as_sequence()
                    .unwrap()
                    .into_iter()
                    .map(|x| serde_yaml::from_value(x.clone()))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(de::Error::custom)?,
                post_create: vec![],
            }),
            _ => Err(de::Error::custom(
                "expected pane definition to be a hashmap or a command list",
            )),
        }
    }
}

mod defaults {
    pub fn window_base_index() -> u32 {
        1
    }

    pub fn pane_base_index() -> u32 {
        1
    }
}

mod deserialize {
    use super::*;

    pub fn template_field<'de, D>(deserializer: D) -> Result<ProjectTemplate, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let val: Value = de::Deserialize::deserialize(deserializer)?;
        match val {
            v if v.is_null() => Ok(ProjectTemplate::Default),
            v if v.is_string() => Ok(ProjectTemplate::Raw(v.as_str().unwrap().into())),
            v if v.is_mapping() => match v.as_mapping().unwrap().get(&"file".into()) {
                None => Err(de::Error::custom("missing 'file' field")),
                Some(filename) => filename.as_str().map_or(
                    Err(de::Error::custom("expected file to be a string")),
                    |path| Ok(ProjectTemplate::File(path.into())),
                ),
            },
            v => Err(de::Error::custom(format!(
                "invalid value for field 'template': {:?}",
                v
            ))),
        }
    }
}
