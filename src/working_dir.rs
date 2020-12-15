use serde::de;
use shellexpand::tilde;

use std::path::PathBuf;

pub fn de_working_dir<'de, D>(deserializer: D) -> Result<Option<PathBuf>, D::Error>
where
    D: de::Deserializer<'de>,
{
    let opt: Option<PathBuf> = de::Deserialize::deserialize(deserializer)?;
    Ok(Some(opt.map_or_else(
        || home_working_dir(),
        |path| process_working_dir(&path.to_string_lossy()),
    )))
}

pub fn process_working_dir(str_path: &str) -> PathBuf {
    PathBuf::from(tilde(str_path).to_string())
}

pub fn home_working_dir() -> PathBuf {
    PathBuf::from(tilde("~").to_string())
}
