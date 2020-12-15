use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
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
