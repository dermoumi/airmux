use crate::config::Config;
use crate::utils::{parse_command, valid_tmux_identifier};

use de::Visitor;
use serde::{de, Deserialize, Serialize};
use shell_words::{quote, split};
use shellexpand::tilde;

use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;

#[derive(Serialize, Debug, PartialEq)]
pub struct Project {
    pub session_name: Option<String>,
    pub tmux_command: Option<String>,
    pub tmux_options: Option<String>,
    pub tmux_socket: Option<String>,
    pub working_dir: Option<PathBuf>,
    pub window_base_index: usize,
    pub pane_base_index: usize,
    pub startup_window: StartupWindow,
    pub on_create: Vec<String>,
    pub post_create: Vec<String>,
    pub attach: bool,
    pub template: ProjectTemplate,
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
        // Make sure session name is valid
        if let Some(session_name) = &self.session_name {
            valid_tmux_identifier(session_name)?;
        }

        // Make sure start up window exists
        match &self.startup_window {
            StartupWindow::Index(index) => {
                if *index >= self.window_base_index + self.windows.len()
                    || *index < self.window_base_index
                {
                    Err(format!(
                        "startup_window: there is no window with index {}",
                        index
                    ))?;
                }
            }
            StartupWindow::Name(name) => {
                if self
                    .windows
                    .iter()
                    .find(|w| match &w.name {
                        Some(window_name) => window_name == name,
                        _ => false,
                    })
                    .is_none()
                {
                    Err(format!(
                        "startup_window: there is no window with name {}",
                        name
                    ))?;
                }
            }
            _ => {}
        }

        // Make sure working_dir exists and is a directory
        if let Some(path) = &self.working_dir {
            if !path.is_dir() {
                Err(format!("session working_dir {:?} does not exist", path))?;
            }
        }

        // Run checks for each window
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

    fn default_window_base_index() -> usize {
        1
    }

    fn default_pane_base_index() -> usize {
        1
    }

    fn default_attach() -> bool {
        true
    }

    fn default_windows() -> Vec<Window> {
        vec![Window::default()]
    }

    fn de_window_base_index<'de, D>(deserializer: D) -> Result<usize, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let opt: Option<usize> = de::Deserialize::deserialize(deserializer)?;
        Ok(opt.unwrap_or(Self::default_window_base_index()))
    }

    fn de_pane_base_index<'de, D>(deserializer: D) -> Result<usize, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let opt: Option<usize> = de::Deserialize::deserialize(deserializer)?;
        Ok(opt.unwrap_or(Self::default_pane_base_index()))
    }

    fn de_windows<'de, D>(deserializer: D) -> Result<Vec<Window>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        #[derive(Deserialize, Debug)]
        #[serde(untagged)]
        enum WindowList {
            List(Vec<Window>),
            Single(Window),
            Empty,
        };

        let window_list: WindowList = de::Deserialize::deserialize(deserializer)?;

        Ok(match window_list {
            WindowList::List(windows) => windows,
            WindowList::Single(window) => vec![window],
            WindowList::Empty => vec![Window::default()],
        })
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
            startup_window: StartupWindow::default(),
            on_create: vec![],
            post_create: vec![],
            attach: true,
            template: ProjectTemplate::default(),
            windows: vec![Window::default()],
        }
    }
}

impl From<Option<Project>> for Project {
    fn from(project: Option<Project>) -> Self {
        project.unwrap_or_default()
    }
}

impl<'de> Deserialize<'de> for Project {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        #[derive(Deserialize, Debug)]
        #[serde(deny_unknown_fields)]
        struct ProjectProxy {
            #[serde(default, alias = "name")]
            pub session_name: Option<String>,
            #[serde(default)]
            pub tmux_command: Option<String>,
            #[serde(default)]
            pub tmux_options: Option<String>,
            #[serde(default, alias = "socket_name")]
            pub tmux_socket: Option<String>,
            #[serde(default, alias = "root", deserialize_with = "de_working_dir")]
            pub working_dir: Option<PathBuf>,
            #[serde(
                default = "Project::default_window_base_index",
                deserialize_with = "Project::de_window_base_index"
            )]
            pub window_base_index: usize,
            #[serde(
                default = "Project::default_pane_base_index",
                deserialize_with = "Project::de_pane_base_index"
            )]
            pub pane_base_index: usize,
            #[serde(default)]
            pub startup_window: StartupWindow,
            #[serde(default, deserialize_with = "de_command_list")]
            pub on_create: Vec<String>,
            #[serde(default, deserialize_with = "de_command_list")]
            pub post_create: Vec<String>,
            #[serde(default = "Project::default_attach")]
            pub attach: bool,
            #[serde(default)]
            pub template: ProjectTemplate,
            #[serde(
                default = "Project::default_windows",
                deserialize_with = "Project::de_windows",
                alias = "window"
            )]
            pub windows: Vec<Window>,
        }

        let opt: Option<ProjectProxy> = de::Deserialize::deserialize(deserializer)?;

        Ok(match opt {
            None => Self::default(),
            Some(project) => Self {
                session_name: project.session_name,
                tmux_command: project.tmux_command,
                tmux_options: project.tmux_options,
                tmux_socket: project.tmux_socket,
                working_dir: project.working_dir,
                window_base_index: project.window_base_index,
                pane_base_index: project.pane_base_index,
                startup_window: project.startup_window,
                on_create: project.on_create,
                post_create: project.post_create,
                attach: project.attach,
                template: project.template,
                windows: project.windows,
            },
        })
    }
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
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
        #[derive(Deserialize, Debug)]
        #[serde(untagged)]
        enum TemplateProxy {
            File { file: PathBuf },
            Raw(String),
            Default,
        }

        let proxy: TemplateProxy = de::Deserialize::deserialize(deserializer)?;
        Ok(match proxy {
            TemplateProxy::File { file } => Self::File(file),
            TemplateProxy::Raw(content) => Self::Raw(content),
            TemplateProxy::Default => Self::Default,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum StartupWindow {
    Default,
    Name(String),
    Index(usize),
}

impl Default for StartupWindow {
    fn default() -> Self {
        StartupWindow::Default
    }
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Window {
    pub name: Option<String>,
    pub working_dir: Option<PathBuf>,
    pub layout: Option<String>,
    pub on_create: Vec<String>,
    pub post_create: Vec<String>,
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
                Err(format!("window working_dir {:?} does not exist", path))?;
            }
        }

        // Run check for each pane
        self.panes
            .iter()
            .map(|p| p.check())
            .collect::<Result<_, _>>()
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
            #[serde(default)]
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
            #[serde(default)]
            panes: Vec<Pane>,
        }

        #[derive(Deserialize, Debug)]
        #[serde(untagged)]
        enum WindowOption {
            None,
            String(String),
            CommandList(Vec<String>),
            PaneList(Vec<Pane>),
            Definition(WindowDef),
            DefinitionWithName(WindowDefWithName),
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
                        WindowOption::PaneList(panes) => window.panes = panes,
                        WindowOption::DefinitionWithName(def) => {
                            window.name = def.name;
                            window.working_dir = def.working_dir;
                            window.layout = def.layout;
                            window.on_create = def.on_create;
                            window.post_create = def.post_create;
                            window.panes = def.panes;
                        }
                        WindowOption::Definition(def) => {
                            window.working_dir = def.working_dir;
                            window.layout = def.layout;
                            window.on_create = def.on_create;
                            window.post_create = def.post_create;
                            window.panes = def.panes;
                        }
                    }
                }
                Some(key) => match value {
                    WindowOption::None => match key.as_str() {
                        "name" | "title" => window.name = None,
                        "working_dir" | "root" => window.working_dir = Some(home_working_dir()),
                        "layout" => window.layout = None,
                        "on_create" => window.on_create = vec![],
                        "post_create" => window.post_create = vec![],
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
                        window.panes = def.panes;
                    }
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

impl Default for Window {
    fn default() -> Self {
        Self {
            name: None,
            working_dir: None,
            layout: None,
            on_create: vec![],
            post_create: vec![],
            panes: vec![Pane::default()],
        }
    }
}

#[derive(Serialize, Default, Debug, PartialEq)]
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
                Err(format!("pane working_dir {:?} does not exist", path))?;
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
            Percent(String),
            Cells(usize),
            None,
        };

        let size: SplitSize = de::Deserialize::deserialize(deserializer)?;
        Ok(match size {
            SplitSize::Percent(percent) => Some(percent),
            SplitSize::Cells(size) => Some(size.to_string()),
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
                            if !first_entry {
                                Err(de::Error::custom(format!(
                                    "pane field {:?} cannot be a boolean",
                                    key
                                )))?;
                            }
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
                            if !first_entry {
                                Err(de::Error::custom(format!(
                                    "pane field {:?} cannot be a number",
                                    key
                                )))?;
                            }

                            pane.name = Some(key);
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
                        }
                    },
                    PaneOption::Definition(def) => {
                        if !first_entry {
                            Err(de::Error::custom(format!(
                                "pane field {:?} cannot be a window definition",
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
                                "pane field {:?} cannot be a window definition",
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

#[derive(Serialize, Debug, PartialEq)]
pub enum PaneSplit {
    #[serde(rename = "horizontal")]
    Horizontal,
    #[serde(rename = "vertical")]
    Vertical,
}

impl<'de> Deserialize<'de> for PaneSplit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let value: String = de::Deserialize::deserialize(deserializer)?;
        Ok(match value {
            s if ["v", "vertical"].contains(&s.to_lowercase().as_str()) => PaneSplit::Vertical,
            s if ["h", "horizontal"].contains(&s.to_lowercase().as_str()) => PaneSplit::Horizontal,
            _ => Err(de::Error::custom(format!(
                "expected split value {:?} to match v|h|vertical|horizontal",
                value
            )))?,
        })
    }
}

fn de_working_dir<'de, D>(deserializer: D) -> Result<Option<PathBuf>, D::Error>
where
    D: de::Deserializer<'de>,
{
    let opt: Option<PathBuf> = de::Deserialize::deserialize(deserializer)?;
    Ok(Some(opt.map_or_else(
        || home_working_dir(),
        |path| process_working_dir(&path.to_string_lossy()),
    )))
}

fn de_command_list<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: de::Deserializer<'de>,
{
    #[derive(Deserialize, Debug)]
    #[serde(untagged)]
    enum CommandList {
        List(Vec<String>),
        Single(String),
        Empty,
    };

    let command_list: CommandList = de::Deserialize::deserialize(deserializer)?;
    Ok(match command_list {
        CommandList::List(commands) => process_command_list(commands),
        CommandList::Single(command) => vec![process_command(command)],
        CommandList::Empty => vec![],
    })
}

fn process_working_dir(str_path: &str) -> PathBuf {
    PathBuf::from(tilde(str_path).to_string())
}

fn home_working_dir() -> PathBuf {
    PathBuf::from(tilde("~").to_string())
}

fn process_command(command: String) -> String {
    command.replace("#", "##")
}

fn process_command_list(commands: Vec<String>) -> Vec<String> {
    commands.into_iter().map(process_command).collect()
}

#[cfg(test)]
#[path = "test/data.rs"]
mod tests;
