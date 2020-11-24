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
#[serde(deny_unknown_fields)]
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
            #[serde(default)]
            pub name: Option<String>,
            #[serde(default, alias = "root", deserialize_with = "de_working_dir")]
            pub working_dir: Option<PathBuf>,
            #[serde(default)]
            pub layout: Option<String>,
            #[serde(default, deserialize_with = "de_command_list")]
            pub on_create: Vec<String>,
            #[serde(default, deserialize_with = "de_command_list")]
            pub post_create: Vec<String>,
            #[serde(default)]
            pub panes: Vec<Pane>,
        }

        #[derive(Deserialize, Debug)]
        #[serde(untagged)]
        enum WindowOption {
            None,
            String(String),
            CommandList(Vec<String>),
            PaneList(Vec<Pane>),
            Definition(WindowDef),
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
                        WindowOption::String(string) => {
                            window.panes = vec![Pane {
                                commands: vec![string],
                                ..Pane::default()
                            }]
                        }
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
                        "name" => window.name = None,
                        "working_dir" | "root" => window.working_dir = None,
                        "layout" => window.layout = None,
                        "on_create" => window.on_create = vec![],
                        "post_create" => window.post_create = vec![],
                        "panes" => window.panes = vec![Pane::default()],
                        _ => {
                            if first_entry {
                                window.name = Some(key);
                            } else {
                                Err(de::Error::custom(format!(
                                    "window field {:?} cannot be null",
                                    key
                                )))?;
                            }
                        }
                    },
                    WindowOption::String(string) => match key.as_str() {
                        "name" => window.name = Some(string),
                        "working_dir" | "root" => window.working_dir = Some(PathBuf::from(string)),
                        "layout" => window.layout = Some(string),
                        "on_create" => window.on_create = vec![process_command(string)],
                        "post_create" => window.post_create = vec![process_command(string)],
                        "panes" => window.panes = vec![Pane::from(string)],
                        _ => {
                            if !first_entry {
                                Err(de::Error::custom(format!(
                                    "window field {:?} cannot be a string",
                                    key
                                )))?
                            }

                            window.name = Some(key);
                            window.panes = vec![Pane::from(string)]
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

impl<'de> Deserialize<'de> for Pane {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        #[derive(Deserialize, Debug)]
        struct PaneDefinition {
            #[serde(default, alias = "root", deserialize_with = "de_working_dir")]
            pub working_dir: Option<PathBuf>,
            #[serde(default)]
            pub split: Option<PaneSplit>,
            #[serde(default)]
            pub split_from: Option<usize>,
            #[serde(default, deserialize_with = "Pane::de_split_size")]
            pub split_size: Option<String>,
            #[serde(default)]
            pub clear: bool,
            #[serde(default, deserialize_with = "de_command_list")]
            pub on_create: Vec<String>,
            #[serde(default, deserialize_with = "de_command_list")]
            pub post_create: Vec<String>,
            #[serde(default, alias = "command", deserialize_with = "de_command_list")]
            pub commands: Vec<String>,
        }

        #[derive(Deserialize, Debug)]
        #[serde(untagged)]
        enum PaneProxy {
            Definition(PaneDefinition),
            CommandList(Vec<String>),
            Command(String),
            None,
        }

        let proxy: PaneProxy = de::Deserialize::deserialize(deserializer)?;
        Ok(match proxy {
            PaneProxy::None => Self::default(),
            PaneProxy::CommandList(list) => Self::from(list),
            PaneProxy::Command(cmd) => Self::from(cmd),
            PaneProxy::Definition(proxy) => Self {
                working_dir: proxy.working_dir,
                split: proxy.split,
                split_from: proxy.split_from,
                split_size: proxy.split_size,
                clear: proxy.clear,
                on_create: proxy.on_create,
                post_create: proxy.post_create,
                commands: proxy.commands,
            },
        })
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
    Ok(Some(PathBuf::from(opt.map_or_else(
        || tilde("~").to_string(),
        |path| tilde(&path.to_string_lossy()).to_string(),
    ))))
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

fn process_command(command: String) -> String {
    command.replace("#", "##")
}

fn process_command_list(commands: Vec<String>) -> Vec<String> {
    commands.into_iter().map(process_command).collect()
}

#[cfg(test)]
#[path = "test/data.rs"]
mod tests;
