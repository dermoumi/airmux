use serde::{de, Deserialize};

pub fn de_command_list<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: de::Deserializer<'de>,
{
    #[derive(Deserialize, Debug)]
    #[serde(untagged)]
    enum CommandList {
        List(Vec<String>),
        Single(String),
        Empty,
    }

    let command_list: CommandList = de::Deserialize::deserialize(deserializer)?;
    Ok(match command_list {
        CommandList::List(commands) => process_command_list(commands),
        CommandList::Single(command) => vec![process_command(command)],
        CommandList::Empty => vec![],
    })
}

pub fn process_command(command: String) -> String {
    command
        .replace('#', "##")
        .replace('\n', " ")
        .replace('\r', "")
}

pub fn process_command_list(commands: Vec<String>) -> Vec<String> {
    commands.into_iter().map(process_command).collect()
}

#[cfg(test)]
#[path = "test/command.rs"]
mod tests;
