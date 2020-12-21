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
        Ok(match value {
            s if ["v", "vertical"].contains(&s.to_lowercase().as_str()) => PaneSplit::Vertical,
            s if ["h", "horizontal"].contains(&s.to_lowercase().as_str()) => PaneSplit::Horizontal,
            _ => {
                return Err(de::Error::custom(format!(
                    "expected split value {:?} to match v|h|vertical|horizontal",
                    value
                )))
            }
        })
    }
}
