use serde::{Deserialize, Serialize};

use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct Project {
    pub name: String,
    pub template: Option<String>,
    pub session_name: Option<String>,
    pub windows: Vec<Window>,
    pub window_base_index: u32,
    pub pane_base_index: u32,
}

impl Project {
    pub fn new(name: &str) -> Project {
        Project {
            name: name.into(),
            template: None,
            session_name: None,
            windows: vec![],
            window_base_index: 1,
            pane_base_index: 1,
        }
    }

    pub fn get_session_name(&self) -> &str {
        self.session_name.as_deref().unwrap_or(&self.name)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Window {
    pub name: Option<String>,
    pub working_dir: Option<PathBuf>,
    pub panes: Vec<Pane>,
}

impl Window {
    pub fn new() -> Window {
        Window {
            name: None,
            working_dir: None,
            panes: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum PaneSplit {
    Horizontal,
    Vertical,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Pane {
    pub working_dir: Option<PathBuf>,
    pub commands: Vec<String>,
    pub post_create: Vec<String>,
    pub split: Option<PaneSplit>,
    pub split_from: Option<u32>,
    pub split_size: Option<String>,
}

impl Pane {
    pub fn new() -> Pane {
        Pane {
            working_dir: None,
            commands: vec![],
            post_create: vec![],
            split: None,
            split_from: None,
            split_size: None,
        }
    }
}
