use serde::{de, ser};
use shellexpand::tilde;

use std::path::PathBuf;

pub fn de_working_dir<'de, D>(deserializer: D) -> Result<Option<PathBuf>, D::Error>
where
    D: de::Deserializer<'de>,
{
    let opt: Option<PathBuf> = de::Deserialize::deserialize(deserializer)?;
    Ok(Some(opt.map_or_else(home_working_dir, |path| {
        process_working_dir(&path.to_string_lossy())
    })))
}

pub fn ser_working_dir<S>(path: &Option<PathBuf>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: ser::Serializer,
{
    match path {
        None => serializer.serialize_unit(), // This one's ignored decause it's the default value
        Some(path) => {
            let home_path = tilde("~").to_string();
            let processed_path = match path.strip_prefix(home_path) {
                Err(_) => path.to_owned(),
                Ok(rel_path) => match rel_path.parent() {
                    None => PathBuf::from("~"),
                    Some(_) => PathBuf::from("~").join(rel_path),
                },
            };

            serializer.serialize_str(processed_path.to_string_lossy().to_string().as_str())
        }
    }
}

pub fn process_working_dir(str_path: &str) -> PathBuf {
    PathBuf::from(tilde(str_path).to_string())
}

pub fn home_working_dir() -> PathBuf {
    PathBuf::from(tilde("~").to_string())
}
