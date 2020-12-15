use crate::command::{de_command_list, process_command, process_command_list};
use crate::pane::Pane;
use crate::utils::valid_tmux_identifier;
use crate::working_dir::{de_working_dir, home_working_dir, process_working_dir};

use de::Visitor;
use serde::{de, Deserialize, Serialize};

use std::error::Error;
use std::fmt;
use std::path::PathBuf;

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct Window {
    pub name: Option<String>,
    pub working_dir: Option<PathBuf>,
    pub layout: Option<String>,
    pub on_create: Vec<String>,
    pub post_create: Vec<String>,
    pub on_pane_create: Vec<String>,
    pub post_pane_create: Vec<String>,
    pub pane_commands: Vec<String>,
    pub panes: Vec<Pane>,
}

impl Window {
    pub fn check(&self) -> Result<(), Box<dyn Error>> {
        // Make sure the pane's
        if let Some(name) = &self.name {
            valid_tmux_identifier(name)?;
        }

        // Check that split_from for each pane points to an existing pane
        for pane in &self.panes {
            pane.check()?;

            if let Some(split_from) = pane.split_from {
                if split_from >= self.panes.len() {
                    Err(format!(
                        "split_from: there is no pane with index {} (pane indexes always start at 0)",
                        split_from
                    ))?;
                }
            }
        }

        // Make sure working_dir exists and is a directory
        if let Some(path) = &self.working_dir {
            if !path.is_dir() {
                Err(format!(
                    "window working_dir {:?} is not a directory or does not exist",
                    path
                ))?;
            }
        }

        // Run check for each pane
        self.panes
            .iter()
            .map(|p| p.check())
            .collect::<Result<_, _>>()
    }

    fn default_panes() -> Vec<Pane> {
        vec![Pane::default()]
    }

    fn de_panes<'de, D>(deserializer: D) -> Result<Vec<Pane>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        #[derive(Deserialize, Debug)]
        #[serde(untagged)]
        enum PaneList {
            Empty,
            List(Vec<Pane>),
            Single(Pane),
        };

        let pane_list: PaneList = de::Deserialize::deserialize(deserializer)?;

        Ok(match pane_list {
            PaneList::List(panes) => panes,
            PaneList::Single(pane) => vec![pane],
            PaneList::Empty => Self::default_panes(),
        })
    }
}

impl From<&str> for Window {
    fn from(command: &str) -> Self {
        Self::from(command.to_string())
    }
}

impl From<String> for Window {
    fn from(command: String) -> Self {
        Self {
            panes: vec![Pane::from(command)],
            ..Self::default()
        }
    }
}

impl From<Vec<String>> for Window {
    fn from(commands: Vec<String>) -> Self {
        Self {
            panes: commands
                .into_iter()
                .map(|command| Pane::from(command))
                .collect(),
            ..Self::default()
        }
    }
}

impl Default for Window {
    fn default() -> Self {
        Self {
            name: None,
            working_dir: None,
            layout: None,
            on_create: vec![],
            post_create: vec![],
            on_pane_create: vec![],
            post_pane_create: vec![],
            pane_commands: vec![],
            panes: Self::default_panes(),
        }
    }
}

struct WindowVisitor;
impl<'de> Visitor<'de> for WindowVisitor {
    type Value = Window;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a window definition")
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Window::default())
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Window::default())
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Window::from(v))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut commands: Vec<String> = Vec::with_capacity(seq.size_hint().unwrap_or(0));

        while let Some(command) = seq.next_element::<String>()? {
            commands.push(command);
        }

        Ok(Window::from(commands))
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: de::MapAccess<'de>,
    {
        type WindowKeyType = Option<String>;

        #[derive(Deserialize, Debug)]
        #[serde(deny_unknown_fields)]
        struct WindowDef {
            #[serde(default, alias = "root", deserialize_with = "de_working_dir")]
            working_dir: Option<PathBuf>,
            #[serde(default)]
            layout: Option<String>,
            #[serde(default, deserialize_with = "de_command_list")]
            on_create: Vec<String>,
            #[serde(default, deserialize_with = "de_command_list")]
            post_create: Vec<String>,
            #[serde(default, deserialize_with = "de_command_list")]
            on_pane_create: Vec<String>,
            #[serde(default, deserialize_with = "de_command_list")]
            post_pane_create: Vec<String>,
            #[serde(
                default,
                alias = "pre",
                alias = "pane_command",
                deserialize_with = "de_command_list"
            )]
            pane_commands: Vec<String>,
            #[serde(
                default = "Window::default_panes",
                deserialize_with = "Window::de_panes"
            )]
            panes: Vec<Pane>,
        }

        #[derive(Deserialize, Debug)]
        #[serde(deny_unknown_fields)]
        struct WindowDefWithName {
            #[serde(alias = "title")]
            name: Option<String>,
            #[serde(default, alias = "root", deserialize_with = "de_working_dir")]
            working_dir: Option<PathBuf>,
            #[serde(default)]
            layout: Option<String>,
            #[serde(default, deserialize_with = "de_command_list")]
            on_create: Vec<String>,
            #[serde(default, deserialize_with = "de_command_list")]
            post_create: Vec<String>,
            #[serde(default, deserialize_with = "de_command_list")]
            on_pane_create: Vec<String>,
            #[serde(default, deserialize_with = "de_command_list")]
            post_pane_create: Vec<String>,
            #[serde(
                default,
                alias = "pre",
                alias = "pane_command",
                deserialize_with = "de_command_list"
            )]
            pane_commands: Vec<String>,
            #[serde(
                default = "Window::default_panes",
                deserialize_with = "Window::de_panes"
            )]
            panes: Vec<Pane>,
        }

        #[derive(Deserialize, Debug)]
        #[serde(untagged)]
        enum WindowOption {
            None,
            String(String),
            CommandList(Vec<String>),
            Definition(WindowDef),
            DefinitionWithName(WindowDefWithName),
            PaneList(Vec<Pane>),
        }

        let mut first_entry = true;
        let mut window = Self::Value::default();
        while let Some((key, value)) = map.next_entry::<WindowKeyType, WindowOption>()? {
            match key {
                None => {
                    if !first_entry {
                        Err(de::Error::custom(
                            "null name can only be set as first element of the map",
                        ))?;
                    }

                    match value {
                        WindowOption::None => {}
                        WindowOption::String(string) => window.panes = vec![Pane::from(string)],
                        WindowOption::CommandList(commands) => {
                            window.panes = commands
                                .into_iter()
                                .map(|command| Pane {
                                    commands: vec![command],
                                    ..Pane::default()
                                })
                                .collect()
                        }
                        WindowOption::DefinitionWithName(def) => {
                            window.name = def.name;
                            window.working_dir = def.working_dir;
                            window.layout = def.layout;
                            window.on_create = def.on_create;
                            window.post_create = def.post_create;
                            window.on_pane_create = def.on_pane_create;
                            window.post_pane_create = def.post_pane_create;
                            window.pane_commands = def.pane_commands;
                            window.panes = def.panes;
                        }
                        WindowOption::Definition(def) => {
                            window.working_dir = def.working_dir;
                            window.layout = def.layout;
                            window.on_create = def.on_create;
                            window.post_create = def.post_create;
                            window.on_pane_create = def.on_pane_create;
                            window.post_pane_create = def.post_pane_create;
                            window.pane_commands = def.pane_commands;
                            window.panes = def.panes;
                        }
                        WindowOption::PaneList(panes) => window.panes = panes,
                    }
                }
                Some(key) => match value {
                    WindowOption::None => match key.as_str() {
                        "name" | "title" => window.name = None,
                        "working_dir" | "root" => window.working_dir = Some(home_working_dir()),
                        "layout" => window.layout = None,
                        "on_create" => window.on_create = vec![],
                        "post_create" => window.post_create = vec![],
                        "on_pane_create" => window.on_pane_create = vec![],
                        "post_pane_create" => window.post_pane_create = vec![],
                        "pane_commands" | "pane_command" | "pre" => window.pane_commands = vec![],
                        "panes" => window.panes = vec![Pane::default()],
                        _ => {
                            if !first_entry {
                                Err(de::Error::custom(format!(
                                    "window field {:?} cannot be null",
                                    key
                                )))?;
                            }

                            window.name = Some(key);
                        }
                    },
                    WindowOption::String(val) => match key.as_str() {
                        "name" | "title" => window.name = Some(val),
                        "working_dir" | "root" => {
                            window.working_dir = Some(process_working_dir(val.as_str()))
                        }
                        "layout" => window.layout = Some(val),
                        "on_create" => window.on_create = vec![process_command(val)],
                        "post_create" => window.post_create = vec![process_command(val)],
                        "on_pane_create" => window.on_pane_create = vec![process_command(val)],
                        "post_pane_create" => window.post_pane_create = vec![process_command(val)],
                        "pane_commands" | "pane_command" | "pre" => {
                            window.pane_commands = vec![process_command(val)]
                        }
                        "panes" => window.panes = vec![Pane::from(val)],
                        _ => {
                            if !first_entry {
                                Err(de::Error::custom(format!(
                                    "window field {:?} cannot be a string",
                                    key
                                )))?
                            }

                            window.name = Some(key);
                            window.panes = vec![Pane::from(val)]
                        }
                    },
                    WindowOption::CommandList(commands) => match key.as_str() {
                        "on_create" => window.on_create = process_command_list(commands),
                        "post_create" => window.post_create = process_command_list(commands),
                        "on_pane_create" => window.on_pane_create = process_command_list(commands),
                        "post_pane_create" => {
                            window.post_pane_create = process_command_list(commands)
                        }
                        "pane_commands" | "pane_command" | "pre" => {
                            window.pane_commands = process_command_list(commands)
                        }
                        "panes" => {
                            window.panes = commands
                                .into_iter()
                                .map(|command| Pane::from(command))
                                .collect()
                        }
                        _ => {
                            if !first_entry {
                                Err(de::Error::custom(format!(
                                    "window field {:?} cannot be a command list",
                                    key
                                )))?
                            }

                            window.name = Some(key);
                            window.panes = commands
                                .into_iter()
                                .map(|command| Pane::from(command))
                                .collect()
                        }
                    },
                    WindowOption::Definition(def) => {
                        if !first_entry {
                            Err(de::Error::custom(format!(
                                "window field {:?} cannot be a window definition",
                                key
                            )))?
                        }

                        window.name = Some(key);
                        window.working_dir = def.working_dir;
                        window.layout = def.layout;
                        window.on_create = def.on_create;
                        window.post_create = def.post_create;
                        window.on_pane_create = def.on_pane_create;
                        window.post_pane_create = def.post_pane_create;
                        window.pane_commands = def.pane_commands;
                        window.panes = def.panes;
                    }
                    WindowOption::DefinitionWithName(def) => {
                        if !first_entry {
                            Err(de::Error::custom(format!(
                                "window field {:?} cannot be a window definition",
                                key
                            )))?
                        }

                        window.name = def.name;
                        window.working_dir = def.working_dir;
                        window.layout = def.layout;
                        window.on_create = def.on_create;
                        window.post_create = def.post_create;
                        window.on_pane_create = def.on_pane_create;
                        window.post_pane_create = def.post_pane_create;
                        window.pane_commands = def.pane_commands;
                        window.panes = def.panes;
                    }
                    WindowOption::PaneList(panes) => match key.as_str() {
                        "panes" => window.panes = panes,
                        _ => {
                            if !first_entry {
                                Err(de::Error::custom(format!(
                                    "window field {:?} cannot be a pane list",
                                    key
                                )))?
                            }

                            window.name = Some(key);
                            window.panes = panes
                        }
                    },
                },
            }

            first_entry = false;
        }

        Ok(window)
    }
}

impl<'de> Deserialize<'de> for Window {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(WindowVisitor)
    }
}

#[cfg(test)]
#[path = "test/window.rs"]
mod tests;
