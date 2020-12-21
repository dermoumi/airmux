use crate::command::de_command_list;
use crate::config::Config;
use crate::pane::Pane;
use crate::pane_split::PaneSplit;
use crate::startup_window::StartupWindow;
use crate::utils::{is_default, parse_command, valid_tmux_identifier};
use crate::window::Window;
use crate::working_dir::{de_working_dir, ser_working_dir};

use serde::ser::{SerializeSeq, Serializer};
use serde::{de, Deserialize, Serialize};
use shell_words::{join, split};

use std::error::Error;
use std::iter;
use std::path::PathBuf;

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct Project {
    pub session_name: Option<String>,
    pub tmux_command: Option<String>,
    pub tmux_options: Option<String>,
    pub tmux_socket: Option<String>,
    pub working_dir: Option<PathBuf>,
    pub window_base_index: usize,
    pub pane_base_index: usize,
    pub startup_window: StartupWindow,
    pub startup_pane: Option<usize>,
    pub on_start: Vec<String>,
    pub on_first_start: Vec<String>,
    pub on_restart: Vec<String>,
    pub on_exit: Vec<String>,
    pub on_stop: Vec<String>,
    pub post_create: Vec<String>,
    pub on_pane_create: Vec<String>,
    pub post_pane_create: Vec<String>,
    pub pane_commands: Vec<String>,
    pub attach: bool,
    pub windows: Vec<Window>,
}

impl Project {
    pub fn prepare(self, config: &Config, project_name: &str, force_attach: Option<bool>) -> Self {
        let mut project = Self {
            session_name: self.session_name.or_else(|| Some(project_name.to_string())),
            ..self
        };

        if let Some(attach) = force_attach {
            project.attach = attach;
        }

        if let Some(tmux_command) = &config.tmux_command {
            project.tmux_command = Some(tmux_command.to_owned());
        } else if project.tmux_command.is_none() {
            project.tmux_command = Some(String::from("tmux"));
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
                    return Err(
                        format!("startup_window: there is no window with index {}", index).into(),
                    );
                }
            }
            StartupWindow::Name(name) => {
                if self
                    .windows
                    .iter()
                    .find(|window| match &window.name {
                        Some(window_name) => window_name == name,
                        _ => false,
                    })
                    .is_none()
                {
                    return Err(
                        format!("startup_window: there is no window with name {:?}", name).into(),
                    );
                }
            }
            _ => {}
        }

        // Make sure working_dir exists and is a directory
        if let Some(path) = &self.working_dir {
            if !path.is_dir() {
                return Err(format!(
                    "project working_dir {:?} is not a directory or does not exist",
                    path
                )
                .into());
            }
        }

        // Run checks for each window
        self.windows
            .iter()
            .map(|w| w.check(self.pane_base_index))
            .collect::<Result<_, _>>()
    }

    // Separates tmux_command into the command itself + an array of arguments
    // The arguments are then merged with the passed arguments
    // Also appends tmux_socket and tmux_options as arguments while at it
    pub fn tmux_command(&self, args: &[&str]) -> Result<(String, Vec<String>), Box<dyn Error>> {
        let command = self.tmux_command.as_ref().ok_or("tmux command not set")?;

        let mut full_args = vec![];

        // Build tmux_socket arguments
        if let Some(tmux_socket) = &self.tmux_socket {
            full_args.extend_from_slice(&["-L", tmux_socket]);
        }

        // Convert tmux_options ot OsString
        let tmux_options_split;
        if let Some(tmux_options) = &self.tmux_options {
            tmux_options_split = split(tmux_options)?;
            let mut tmux_options_split: Vec<&str> =
                tmux_options_split.iter().map(|x| x.as_str()).collect();
            full_args.append(&mut tmux_options_split);
        }

        full_args.extend_from_slice(args);

        // Use utiliy to split command and append args to the split arguments
        parse_command(&command, &full_args)
    }

    // Sanitizes tmux_command for use in the template file
    pub fn tmux<'a, I, S>(&self, args: I) -> Result<String, Box<dyn Error>>
    where
        I: IntoIterator<Item = &'a S>,
        S: AsRef<str> + 'a,
    {
        let args: Vec<&str> = args.into_iter().map(AsRef::as_ref).collect();
        let (command, args) = self.tmux_command(&args)?;

        Ok(join(iter::once(command).chain(args.into_iter())))
    }

    fn default_window_base_index() -> usize {
        1
    }

    fn is_default_window_base_index(value: &usize) -> bool {
        value == &Self::default_window_base_index()
    }

    fn default_pane_base_index() -> usize {
        1
    }

    fn is_default_pane_base_index(value: &usize) -> bool {
        value == &Self::default_pane_base_index()
    }

    fn default_windows() -> Vec<Window> {
        vec![Window::default()]
    }

    fn default_attach() -> bool {
        true
    }

    fn is_default_attach(attach: &bool) -> bool {
        attach == &Self::default_attach()
    }

    fn de_window_base_index<'de, D>(deserializer: D) -> Result<usize, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let opt: Option<usize> = de::Deserialize::deserialize(deserializer)?;
        Ok(opt.unwrap_or_else(Self::default_window_base_index))
    }

    fn de_pane_base_index<'de, D>(deserializer: D) -> Result<usize, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let opt: Option<usize> = de::Deserialize::deserialize(deserializer)?;
        Ok(opt.unwrap_or_else(Self::default_pane_base_index))
    }

    fn de_windows<'de, D>(deserializer: D) -> Result<Vec<Window>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        #[derive(Deserialize, Debug)]
        #[serde(untagged)]
        enum WindowList {
            Empty,
            List(Vec<Window>),
            Single(Window),
        };

        let window_list: WindowList = de::Deserialize::deserialize(deserializer)?;

        Ok(match window_list {
            WindowList::List(windows) => windows,
            WindowList::Single(window) => vec![window],
            WindowList::Empty => Self::default_windows(),
        })
    }

    pub fn serialize_compact(&self, json: bool) -> Result<String, Box<dyn Error>> {
        fn is_default_windows(windows: &[CompactWindow]) -> bool {
            Project::default_windows()
                .into_iter()
                .map(CompactWindow::from)
                .eq(windows.to_owned())
        }

        pub fn is_default_panes(panes: &[CompactPane]) -> bool {
            Window::default_panes()
                .into_iter()
                .map(CompactPane::from)
                .eq(panes.to_owned())
        }

        #[derive(Serialize, PartialEq, Clone)]
        pub struct CompactProject {
            #[serde(skip_serializing_if = "is_default")]
            pub session_name: Option<String>,
            #[serde(skip_serializing_if = "is_default")]
            pub tmux_command: Option<String>,
            #[serde(skip_serializing_if = "is_default")]
            pub tmux_options: Option<String>,
            #[serde(skip_serializing_if = "is_default")]
            pub tmux_socket: Option<String>,
            #[serde(skip_serializing_if = "is_default", serialize_with = "ser_working_dir")]
            pub working_dir: Option<PathBuf>,
            #[serde(skip_serializing_if = "Project::is_default_window_base_index")]
            pub window_base_index: usize,
            #[serde(skip_serializing_if = "Project::is_default_pane_base_index")]
            pub pane_base_index: usize,
            #[serde(skip_serializing_if = "is_default")]
            pub startup_window: StartupWindow,
            #[serde(skip_serializing_if = "is_default")]
            pub startup_pane: Option<usize>,
            #[serde(skip_serializing_if = "is_default")]
            pub on_start: Vec<String>,
            #[serde(skip_serializing_if = "is_default")]
            pub on_first_start: Vec<String>,
            #[serde(skip_serializing_if = "is_default")]
            pub on_restart: Vec<String>,
            #[serde(skip_serializing_if = "is_default")]
            pub on_exit: Vec<String>,
            #[serde(skip_serializing_if = "is_default")]
            pub on_stop: Vec<String>,
            #[serde(skip_serializing_if = "is_default")]
            pub post_create: Vec<String>,
            #[serde(skip_serializing_if = "is_default")]
            pub on_pane_create: Vec<String>,
            #[serde(skip_serializing_if = "is_default")]
            pub post_pane_create: Vec<String>,
            #[serde(skip_serializing_if = "is_default")]
            pub pane_commands: Vec<String>,
            #[serde(skip_serializing_if = "Project::is_default_attach")]
            pub attach: bool,
            #[serde(skip_serializing_if = "is_default_windows")]
            pub windows: Vec<CompactWindow>,
        }

        impl From<Project> for CompactProject {
            fn from(copy: Project) -> Self {
                Self {
                    session_name: copy.session_name,
                    tmux_command: copy.tmux_command,
                    tmux_options: copy.tmux_options,
                    tmux_socket: copy.tmux_socket,
                    working_dir: copy.working_dir,
                    window_base_index: copy.window_base_index,
                    pane_base_index: copy.pane_base_index,
                    startup_window: copy.startup_window,
                    startup_pane: copy.startup_pane,
                    on_start: copy.on_start,
                    on_first_start: copy.on_first_start,
                    on_restart: copy.on_restart,
                    on_exit: copy.on_exit,
                    on_stop: copy.on_stop,
                    post_create: copy.post_create,
                    on_pane_create: copy.on_pane_create,
                    post_pane_create: copy.post_pane_create,
                    pane_commands: copy.pane_commands,
                    attach: copy.attach,
                    windows: copy.windows.into_iter().map(CompactWindow::from).collect(),
                }
            }
        }

        #[derive(Serialize, PartialEq, Clone)]
        pub struct CompactWindow {
            pub name: Option<String>,
            #[serde(skip_serializing_if = "is_default", serialize_with = "ser_working_dir")]
            pub working_dir: Option<PathBuf>,
            #[serde(skip_serializing_if = "is_default")]
            pub layout: Option<String>,
            #[serde(skip_serializing_if = "is_default")]
            pub on_create: Vec<String>,
            #[serde(skip_serializing_if = "is_default")]
            pub post_create: Vec<String>,
            #[serde(skip_serializing_if = "is_default")]
            pub on_pane_create: Vec<String>,
            #[serde(skip_serializing_if = "is_default")]
            pub post_pane_create: Vec<String>,
            #[serde(skip_serializing_if = "is_default")]
            pub pane_commands: Vec<String>,
            #[serde(skip_serializing_if = "is_default_panes", serialize_with = "ser_panes")]
            pub panes: Vec<CompactPane>,
        }

        impl From<Window> for CompactWindow {
            fn from(copy: Window) -> Self {
                Self {
                    name: copy.name,
                    working_dir: copy.working_dir,
                    layout: copy.layout,
                    on_create: copy.on_create,
                    post_create: copy.post_create,
                    on_pane_create: copy.on_pane_create,
                    post_pane_create: copy.post_pane_create,
                    pane_commands: copy.pane_commands,
                    panes: copy.panes.into_iter().map(CompactPane::from).collect(),
                }
            }
        }

        #[derive(Serialize, PartialEq, Clone)]
        pub struct CompactPane {
            #[serde(skip_serializing_if = "is_default")]
            pub name: Option<String>,
            #[serde(skip_serializing_if = "is_default", serialize_with = "ser_working_dir")]
            pub working_dir: Option<PathBuf>,
            #[serde(skip_serializing_if = "is_default")]
            pub split: Option<PaneSplit>,
            #[serde(skip_serializing_if = "is_default")]
            pub split_from: Option<usize>,
            #[serde(skip_serializing_if = "is_default")]
            pub split_size: Option<String>,
            #[serde(skip_serializing_if = "is_default")]
            pub clear: bool,
            #[serde(skip_serializing_if = "is_default")]
            pub on_create: Vec<String>,
            #[serde(skip_serializing_if = "is_default")]
            pub post_create: Vec<String>,
            #[serde(skip_serializing_if = "is_default")]
            pub commands: Vec<String>,
        }

        impl From<Pane> for CompactPane {
            fn from(copy: Pane) -> Self {
                Self {
                    name: copy.name,
                    working_dir: copy.working_dir,
                    split: copy.split,
                    split_from: copy.split_from,
                    split_size: copy.split_size,
                    clear: copy.clear,
                    on_create: copy.on_create,
                    post_create: copy.post_create,
                    commands: copy.commands,
                }
            }
        }

        pub fn ser_panes<S>(panes: &[CompactPane], serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut seq = serializer.serialize_seq(Some(panes.len()))?;
            for pane in panes {
                if pane.commands.len() <= 1
                    && is_default(&pane.name)
                    && is_default(&pane.working_dir)
                    && is_default(&pane.split)
                    && is_default(&pane.split_from)
                    && is_default(&pane.split_size)
                    && is_default(&pane.clear)
                    && is_default(&pane.on_create)
                    && is_default(&pane.post_create)
                {
                    if pane.commands.is_empty() {
                        seq.serialize_element(&None as &Option<&str>)?;
                    } else {
                        seq.serialize_element(&pane.commands[0])?;
                    }
                } else {
                    seq.serialize_element(pane)?;
                }
            }
            seq.end()
        }

        let project = CompactProject::from(self.to_owned());

        Ok(if json {
            serde_json::to_string_pretty(&project)?
        } else {
            serde_yaml::to_string(&project)?
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
            startup_pane: None,
            on_start: vec![],
            on_first_start: vec![],
            on_restart: vec![],
            on_exit: vec![],
            on_stop: vec![],
            post_create: vec![],
            on_pane_create: vec![],
            post_pane_create: vec![],
            pane_commands: vec![],
            attach: true,
            windows: Self::default_windows(),
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
            session_name: Option<String>,
            #[serde(default)]
            tmux_command: Option<String>,
            #[serde(default)]
            tmux_options: Option<String>,
            #[serde(default, alias = "socket_name")]
            tmux_socket: Option<String>,
            #[serde(default, alias = "root", deserialize_with = "de_working_dir")]
            working_dir: Option<PathBuf>,
            #[serde(
                default = "Project::default_window_base_index",
                deserialize_with = "Project::de_window_base_index"
            )]
            window_base_index: usize,
            #[serde(
                default = "Project::default_pane_base_index",
                deserialize_with = "Project::de_pane_base_index"
            )]
            pane_base_index: usize,
            #[serde(default)]
            startup_window: StartupWindow,
            #[serde(default)]
            startup_pane: Option<usize>,
            #[serde(
                default,
                alias = "on_project_start",
                deserialize_with = "de_command_list"
            )]
            on_start: Vec<String>,
            #[serde(
                default,
                alias = "on_project_first_start",
                alias = "on_create",
                deserialize_with = "de_command_list"
            )]
            on_first_start: Vec<String>,
            #[serde(
                default,
                alias = "on_project_restart",
                deserialize_with = "de_command_list"
            )]
            on_restart: Vec<String>,
            #[serde(
                default,
                alias = "on_project_exit",
                deserialize_with = "de_command_list"
            )]
            on_exit: Vec<String>,
            #[serde(
                default,
                alias = "on_project_stop",
                deserialize_with = "de_command_list"
            )]
            on_stop: Vec<String>,
            #[serde(default, deserialize_with = "de_command_list")]
            post_create: Vec<String>,
            #[serde(default, deserialize_with = "de_command_list")]
            on_pane_create: Vec<String>,
            #[serde(default, deserialize_with = "de_command_list")]
            post_pane_create: Vec<String>,
            #[serde(
                default,
                alias = "pre_window",
                alias = "pane_command",
                deserialize_with = "de_command_list"
            )]
            pane_commands: Vec<String>,
            #[serde(default, alias = "tmux_attached")]
            attach: Option<bool>,
            #[serde(default, alias = "tmux_detached")]
            detached: Option<bool>,
            #[serde(
                default = "Project::default_windows",
                alias = "window",
                deserialize_with = "Project::de_windows"
            )]
            windows: Vec<Window>,
        }

        let opt: Option<ProjectProxy> = de::Deserialize::deserialize(deserializer)?;

        Ok(match opt {
            None => Self::default(),
            Some(project) => {
                let attach = match project.attach {
                    Some(attach) => match project.detached {
                        None => attach,
                        Some(_) => {
                            return Err(de::Error::custom(
                                "cannot set both 'attach' and 'detached' fields",
                            ))
                        }
                    },
                    None => match project.detached {
                        Some(detached) => !detached,
                        None => Self::default_attach(),
                    },
                };

                Self {
                    session_name: project.session_name,
                    tmux_command: project.tmux_command,
                    tmux_options: project.tmux_options,
                    tmux_socket: project.tmux_socket,
                    working_dir: project.working_dir,
                    window_base_index: project.window_base_index,
                    pane_base_index: project.pane_base_index,
                    startup_window: project.startup_window,
                    startup_pane: project.startup_pane,
                    on_start: project.on_start,
                    on_first_start: project.on_first_start,
                    on_restart: project.on_restart,
                    on_exit: project.on_exit,
                    on_stop: project.on_stop,
                    post_create: project.post_create,
                    on_pane_create: project.on_pane_create,
                    post_pane_create: project.post_pane_create,
                    pane_commands: project.pane_commands,
                    attach,
                    windows: project.windows,
                }
            }
        })
    }
}

#[cfg(test)]
#[path = "test/project.rs"]
mod tests;
