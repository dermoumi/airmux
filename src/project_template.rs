use serde::{de, Deserialize, Serialize};

use std::path::PathBuf;

#[derive(Serialize, Debug, PartialEq, Clone)]
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

#[cfg(test)]
#[path = "test/project_template.rs"]
mod tests;
