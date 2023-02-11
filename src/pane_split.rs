use serde::{de, Deserialize, Serialize};

#[derive(Serialize, Debug, PartialEq, Clone)]
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
        let pane_split = match &value.to_lowercase().as_str() {
            s if ["v", "vertical"].contains(s) => PaneSplit::Vertical,
            s if ["h", "horizontal"].contains(s) => PaneSplit::Horizontal,
            _ => {
                return Err(de::Error::custom(format!(
                    "expected split value {value:?} to match v|h|vertical|horizontal"
                )))
            }
        };

        Ok(pane_split)
    }
}
