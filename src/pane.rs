use crate::command::{de_command_list, process_command, process_command_list};
use crate::pane_split::PaneSplit;
use crate::working_dir::{de_working_dir, home_working_dir, process_working_dir};

use de::Visitor;
use serde::{de, Deserialize, Serialize};

use std::error::Error;
use std::fmt;
use std::path::PathBuf;

#[derive(Serialize, Default, Debug, PartialEq, Clone)]
pub struct Pane {
    pub name: Option<String>,
    pub working_dir: Option<PathBuf>,
    pub split: Option<PaneSplit>,
    pub split_from: Option<usize>,
    pub split_size: Option<String>,
    pub clear: bool,
    pub on_create: Vec<String>,
    pub post_create: Vec<String>,
    pub commands: Vec<String>,
}

impl Pane {
    pub fn check(&self) -> Result<(), Box<dyn Error>> {
        // Make sure working_dir exists and is a directory
        if let Some(path) = &self.working_dir {
            if !path.is_dir() {
                Err(format!(
                    "pane working_dir {:?} is not a directory or does not exist",
                    path
                ))?;
            }
        }

        Ok(())
    }

    fn de_split_size<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        #[derive(Deserialize, Debug)]
        #[serde(untagged)]
        enum SplitSize {
            Cells(usize),
            Percent(String),
            None,
        };

        let size: SplitSize = de::Deserialize::deserialize(deserializer)?;
        Ok(match size {
            SplitSize::Cells(size) => Some(size.to_string()),
            SplitSize::Percent(percent) => Some(percent),
            SplitSize::None => None,
        })
    }
}

impl From<&str> for Pane {
    fn from(command: &str) -> Self {
        Self::from(command.to_string())
    }
}

impl From<String> for Pane {
    fn from(command: String) -> Self {
        Self {
            commands: vec![process_command(command)],
            ..Self::default()
        }
    }
}

impl From<Vec<String>> for Pane {
    fn from(commands: Vec<String>) -> Self {
        Self {
            commands: commands.into_iter().map(process_command).collect(),
            ..Self::default()
        }
    }
}

struct PaneVisitor;
impl<'de> Visitor<'de> for PaneVisitor {
    type Value = Pane;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a pane definition")
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Pane::default())
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Pane::default())
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Pane::from(v))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut commands: Vec<String> = Vec::with_capacity(seq.size_hint().unwrap_or(0));

        while let Some(command) = seq.next_element::<String>()? {
            commands.push(command);
        }

        Ok(Pane::from(commands))
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: de::MapAccess<'de>,
    {
        type PaneKeyType = Option<String>;

        #[derive(Deserialize, Debug)]
        #[serde(deny_unknown_fields)]
        struct PaneDef {
            #[serde(default, alias = "root", deserialize_with = "de_working_dir")]
            working_dir: Option<PathBuf>,
            #[serde(default)]
            split: Option<PaneSplit>,
            #[serde(default)]
            split_from: Option<usize>,
            #[serde(default, deserialize_with = "Pane::de_split_size")]
            split_size: Option<String>,
            #[serde(default)]
            clear: bool,
            #[serde(default, deserialize_with = "de_command_list")]
            on_create: Vec<String>,
            #[serde(default, deserialize_with = "de_command_list")]
            post_create: Vec<String>,
            #[serde(default, alias = "command", deserialize_with = "de_command_list")]
            commands: Vec<String>,
        }

        #[derive(Deserialize, Debug)]
        #[serde(deny_unknown_fields)]
        struct PaneDefWithName {
            #[serde(alias = "title")]
            name: Option<String>,
            #[serde(default, alias = "root", deserialize_with = "de_working_dir")]
            working_dir: Option<PathBuf>,
            #[serde(default)]
            split: Option<PaneSplit>,
            #[serde(default)]
            split_from: Option<usize>,
            #[serde(default, deserialize_with = "Pane::de_split_size")]
            split_size: Option<String>,
            #[serde(default)]
            clear: bool,
            #[serde(default, deserialize_with = "de_command_list")]
            on_create: Vec<String>,
            #[serde(default, deserialize_with = "de_command_list")]
            post_create: Vec<String>,
            #[serde(default, alias = "command", deserialize_with = "de_command_list")]
            commands: Vec<String>,
        }

        #[derive(Deserialize, Debug)]
        #[serde(untagged)]
        enum PaneOption {
            None,
            Bool(bool),
            Number(usize),
            String(String),
            CommandList(Vec<String>),
            Definition(PaneDef),
            DefinitionWithName(PaneDefWithName),
        }

        let mut first_entry = true;
        let mut pane = Self::Value::default();
        while let Some((key, val)) = map.next_entry::<PaneKeyType, PaneOption>()? {
            match key {
                None => {
                    if !first_entry {
                        Err(de::Error::custom(
                            "null name can only be set as first element of the map",
                        ))?;
                    }

                    match val {
                        PaneOption::None => {}
                        PaneOption::Bool(val) => {
                            Err(de::Error::custom(format!(
                                "invalid value for pane: {:?}",
                                val
                            )))?;
                        }
                        PaneOption::Number(val) => {
                            Err(de::Error::custom(format!(
                                "invalid value for pane: {:?}",
                                val
                            )))?;
                        }
                        PaneOption::String(string) => pane.commands = vec![process_command(string)],
                        PaneOption::CommandList(commands) => {
                            pane.commands = process_command_list(commands)
                        }
                        PaneOption::Definition(def) => {
                            pane.working_dir = def.working_dir;
                            pane.split = def.split;
                            pane.split_from = def.split_from;
                            pane.split_size = def.split_size;
                            pane.clear = def.clear;
                            pane.on_create = def.on_create;
                            pane.post_create = def.post_create;
                            pane.commands = def.commands;
                        }
                        PaneOption::DefinitionWithName(def) => {
                            pane.name = def.name;
                            pane.working_dir = def.working_dir;
                            pane.split = def.split;
                            pane.split_from = def.split_from;
                            pane.split_size = def.split_size;
                            pane.clear = def.clear;
                            pane.on_create = def.on_create;
                            pane.post_create = def.post_create;
                            pane.commands = def.commands;
                        }
                    }
                }
                Some(key) => match val {
                    PaneOption::None => match key.as_str() {
                        "name" | "title" => pane.name = None,
                        "working_dir" | "root" => pane.working_dir = Some(home_working_dir()),
                        "split" => pane.split = None,
                        "split_from" => pane.split_from = None,
                        "split_size" => pane.split_size = None,
                        "clear" => pane.clear = false,
                        "on_create" => pane.on_create = vec![],
                        "post_create" => pane.post_create = vec![],
                        "commands" | "command" => pane.commands = vec![],
                        _ => {
                            if !first_entry {
                                Err(de::Error::custom(format!(
                                    "pane field {:?} cannot be null",
                                    key
                                )))?;
                            }

                            pane.name = Some(key);
                        }
                    },
                    PaneOption::Bool(val) => match key.as_str() {
                        "clear" => pane.clear = val,
                        _ => {
                            Err(de::Error::custom(format!(
                                "pane field {:?} cannot be a boolean",
                                key
                            )))?;
                        }
                    },
                    PaneOption::Number(val) => match key.as_str() {
                        "name" | "title" => pane.name = Some(val.to_string()),
                        "working_dir" | "root" => {
                            pane.working_dir = Some(process_working_dir(val.to_string().as_str()))
                        }
                        "split_from" => pane.split_from = Some(val),
                        "split_size" => pane.split_size = Some(val.to_string()),
                        "clear" => pane.clear = val != 0,
                        _ => {
                            Err(de::Error::custom(format!(
                                "pane field {:?} cannot be a number",
                                key
                            )))?;
                        }
                    },
                    PaneOption::String(val) => match key.as_str() {
                        "name" | "title" => pane.name = Some(val),
                        "working_dir" | "root" => {
                            pane.working_dir = Some(process_working_dir(val.as_str()))
                        }
                        "split" => {
                            pane.split = Some(match val {
                                s if ["v", "vertical"].contains(&s.to_lowercase().as_str()) => {
                                    PaneSplit::Vertical
                                }
                                s if ["h", "horizontal"].contains(&s.to_lowercase().as_str()) => {
                                    PaneSplit::Horizontal
                                }
                                _ => Err(de::Error::custom(format!(
                                    "expected split value {:?} to match v|h|vertical|horizontal",
                                    val
                                )))?,
                            })
                        }
                        "split_size" => pane.split_size = Some(val),
                        "on_create" => pane.on_create = vec![process_command(val)],
                        "post_create" => pane.post_create = vec![process_command(val)],
                        "commands" | "command" => pane.commands = vec![process_command(val)],
                        _ => {
                            if !first_entry {
                                Err(de::Error::custom(format!(
                                    "pane field {:?} cannot be a string",
                                    key
                                )))?;
                            }

                            pane.name = Some(key);
                            pane.commands = vec![process_command(val)];
                        }
                    },
                    PaneOption::CommandList(commands) => match key.as_str() {
                        "on_create" => pane.on_create = process_command_list(commands),
                        "post_create" => pane.post_create = process_command_list(commands),
                        "commands" | "command" => pane.commands = process_command_list(commands),
                        _ => {
                            if !first_entry {
                                Err(de::Error::custom(format!(
                                    "pane field {:?} cannot be a command list",
                                    key
                                )))?;
                            }

                            pane.name = Some(key);
                            pane.commands = process_command_list(commands);
                        }
                    },
                    PaneOption::Definition(def) => {
                        if !first_entry {
                            Err(de::Error::custom(format!(
                                "pane field {:?} cannot be a pane definition",
                                key
                            )))?
                        }

                        pane.name = Some(key);
                        pane.working_dir = def.working_dir;
                        pane.split = def.split;
                        pane.split_from = def.split_from;
                        pane.split_size = def.split_size;
                        pane.clear = def.clear;
                        pane.on_create = def.on_create;
                        pane.post_create = def.post_create;
                        pane.commands = def.commands;
                    }
                    PaneOption::DefinitionWithName(def) => {
                        if !first_entry {
                            Err(de::Error::custom(format!(
                                "pane field {:?} cannot be a pane definition",
                                key
                            )))?
                        }

                        pane.name = def.name;
                        pane.working_dir = def.working_dir;
                        pane.split = def.split;
                        pane.split_from = def.split_from;
                        pane.split_size = def.split_size;
                        pane.clear = def.clear;
                        pane.on_create = def.on_create;
                        pane.post_create = def.post_create;
                        pane.commands = def.commands;
                    }
                },
            }

            first_entry = false;
        }

        Ok(pane)
    }
}

impl<'de> Deserialize<'de> for Pane {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(PaneVisitor)
    }
}

#[cfg(test)]
#[path = "test/pane.rs"]
mod tests;
