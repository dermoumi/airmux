use super::*;

use tempfile::tempdir;

use std::fs;

#[test]
fn pane_check_succeeds_on_valid_pane() {
    let pane = Pane::default();

    let result = pane.check();
    assert!(result.is_ok());
}

#[test]
fn pane_check_succeeds_when_working_dir_is_a_existing_dir() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();

    let pane = Pane {
        working_dir: Some(temp_dir),
        ..Pane::default()
    };
    let result = pane.check();
    assert!(result.is_ok());
}

#[test]
fn pane_check_fails_when_working_dir_is_missing() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();

    // Does not exist
    let working_dir = temp_dir.join("random_dirname");
    let pane = Pane {
        working_dir: Some(working_dir.to_owned()),
        ..Pane::default()
    };
    let result = pane.check();
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        format!(
            "pane working_dir {:?} is not a directory or does not exist",
            working_dir
        ),
    );
}

#[test]
fn pane_check_fails_when_working_dir_is_not_a_directory() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();

    // Exists but not a directory
    let working_dir = temp_dir.join("some_filename");
    let file = fs::File::create(&working_dir).unwrap();
    file.sync_all().unwrap();
    drop(file);
    assert!(working_dir.is_file());

    let pane = Pane {
        working_dir: Some(working_dir.to_owned()),
        ..Pane::default()
    };
    let result = pane.check();
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        format!(
            "pane working_dir {:?} is not a directory or does not exist",
            working_dir,
        ),
    );
}

#[test]
fn pane_1st_form_deserializes_from_null() {
    let yaml = r#"
        ~
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane, Pane::default());
}

#[test]
fn pane_1st_form_deserializes_correctly() {
    let yaml = r#"
        name: pane name
        working_dir: /home
        split: v
        split_from: 1
        split_size: 42%
        clear: true
        on_create: echo on_create
        post_create: echo post_create
        command: echo command
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        pane,
        Pane {
            name: Some(String::from("pane name")),
            working_dir: Some(PathBuf::from("/home")),
            split: Some(PaneSplit::Vertical),
            split_from: Some(1),
            split_size: Some(String::from("42%")),
            clear: true,
            on_create: vec![String::from("echo on_create")],
            post_create: vec![String::from("echo post_create")],
            commands: vec![String::from("echo command")],
        }
    )
}

#[test]
fn pane_1st_form_deserializes_split_size_null() {
    let yaml = r#"
        split_size:
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.split_size, None);
}

#[test]
fn pane_1st_form_deserializes_split_size_string() {
    let yaml = r#"
        split_size: "75%"
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.split_size, Some(String::from("75%")));
}

#[test]
fn pane_1st_form_deserializes_split_size_number() {
    let yaml = r#"
        split_size: 42
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.split_size, Some(String::from("42")));
}

#[test]
fn pane_1st_form_raises_error_on_invalid_split_size_value() {
    let yaml = r#"
        split_size:
          - 42
    "#;

    let result = serde_yaml::from_str::<Pane>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("data did not match any variant of untagged enum PaneOption"));
}

#[test]
fn pane_1st_form_deserializes_correctly_with_key_name() {
    let yaml = r#"
        pane name:
        working_dir: /home
        split: v
        split_from: 1
        split_size: 42%
        clear: true
        on_create: echo on_create
        post_create: echo post_create
        command: echo command
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        pane,
        Pane {
            name: Some(String::from("pane name")),
            working_dir: Some(PathBuf::from("/home")),
            split: Some(PaneSplit::Vertical),
            split_from: Some(1),
            split_size: Some(String::from("42%")),
            clear: true,
            on_create: vec![String::from("echo on_create")],
            post_create: vec![String::from("echo post_create")],
            commands: vec![String::from("echo command")],
        }
    )
}

#[test]
fn pane_1st_form_deserializes_correctly_with_null_key_name() {
    let yaml = r#"
        ~:
        working_dir: /home
        split: h
        split_from: 1
        split_size: 42%
        clear: true
        on_create: echo on_create
        post_create: echo post_create
        command: echo command
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        pane,
        Pane {
            name: None,
            working_dir: Some(PathBuf::from("/home")),
            split: Some(PaneSplit::Horizontal),
            split_from: Some(1),
            split_size: Some(String::from("42%")),
            clear: true,
            on_create: vec![String::from("echo on_create")],
            post_create: vec![String::from("echo post_create")],
            commands: vec![String::from("echo command")],
        }
    )
}

#[test]
fn pane_1st_form_fails_when_key_name_is_not_first_line() {
    let yaml = r#"
        working_dir: /home
        some name:
        split: v
        split_from: 1
        split_size: 42%
        clear: true
        on_create: echo on_create
        post_create: echo post_create
        command: echo command
    "#;

    let result = serde_yaml::from_str::<Pane>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("pane field \"some name\" cannot be null"));
}

#[test]
fn pane_1st_form_fails_when_null_key_name_is_not_first_line() {
    let yaml = r#"
        working_dir: /home
        ~:
        split: v
        split_from: 1
        split_size: 42%
        clear: true
        on_create: echo on_create
        post_create: echo post_create
        command: echo command
    "#;

    let result = serde_yaml::from_str::<Pane>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("null name can only be set as first element of the map"));
}

#[test]
fn pane_1st_form_deserializes_working_dir_from_number() {
    let yaml = r#"
        pane:
        working_dir: 0
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.working_dir, Some(PathBuf::from("0")));
}

#[test]
fn pane_1st_form_fails_when_a_field_does_not_accept_a_string() {
    let yaml = r#"
        pane:
        clear: a string
    "#;

    let result = serde_yaml::from_str::<Pane>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("pane field \"clear\" cannot be a string"));
}

#[test]
fn pane_1st_form_fails_when_a_field_does_not_accept_a_command_list() {
    let yaml = r#"
        pane:
        clear:
            - command1
            - command2
    "#;

    let result = serde_yaml::from_str::<Pane>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("pane field \"clear\" cannot be a command list"));
}

#[test]
fn pane_1st_form_fails_when_a_field_does_not_accept_a_pane_definition_with_name() {
    let yaml = r#"
        pane:
        clear:
            name: pane
            command: echo hi
    "#;

    let result = serde_yaml::from_str::<Pane>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("pane field \"clear\" cannot be a pane definition"));
}

#[test]
fn pane_1st_form_fails_when_a_field_does_not_accept_a_pane_definition_without_name() {
    let yaml = r#"
        pane:
        clear:
            command: echo hi
    "#;

    let result = serde_yaml::from_str::<Pane>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("pane field \"clear\" cannot be a pane definition"));
}

#[test]
fn pane_1st_form_deserializes_split_h() {
    let yaml = r#"
        split: h
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.split, Some(PaneSplit::Horizontal));
}

#[test]
fn pane_1st_form_deserializes_split_horizontal() {
    let yaml = r#"
        split: horizontal
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.split, Some(PaneSplit::Horizontal));
}

#[test]
fn pane_1st_form_deserializes_split_v() {
    let yaml = r#"
        split: v
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.split, Some(PaneSplit::Vertical));
}

#[test]
fn pane_1st_form_deserializes_split_vertical() {
    let yaml = r#"
        split: vertical
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.split, Some(PaneSplit::Vertical));
}

#[test]
fn pane_1st_form_raises_error_on_invalid_split_value() {
    let yaml = r#"
        split: o
    "#;

    let result = serde_yaml::from_str::<Pane>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("expected split value \"o\" to match v|h|vertical|horizontal"));
}

#[test]
fn pane_2nd_form_deserializes_correctly_with_name() {
    let yaml = r#"
        pane name:
            working_dir: /home
            split: v
            split_from: 1
            split_size: 42%
            clear: true
            on_create: echo on_create
            post_create: echo post_create
            command: echo command
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        pane,
        Pane {
            name: Some(String::from("pane name")),
            working_dir: Some(PathBuf::from("/home")),
            split: Some(PaneSplit::Vertical),
            split_from: Some(1),
            split_size: Some(String::from("42%")),
            clear: true,
            on_create: vec![String::from("echo on_create")],
            post_create: vec![String::from("echo post_create")],
            commands: vec![String::from("echo command")],
        }
    )
}

#[test]
fn pane_2nd_form_deserializes_correctly_with_null_name() {
    let yaml = r#"
        ~:
            working_dir: /home
            split: v
            split_from: 1
            split_size: 42%
            clear: true
            on_create: echo on_create
            post_create: echo post_create
            command: echo command
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        pane,
        Pane {
            name: None,
            working_dir: Some(PathBuf::from("/home")),
            split: Some(PaneSplit::Vertical),
            split_from: Some(1),
            split_size: Some(String::from("42%")),
            clear: true,
            on_create: vec![String::from("echo on_create")],
            post_create: vec![String::from("echo post_create")],
            commands: vec![String::from("echo command")],
        }
    )
}

#[test]
fn pane_2nd_form_deserializes_correctly_from_string() {
    let yaml = r#"
        pane name: command
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        pane,
        Pane {
            name: Some(String::from("pane name")),
            commands: vec![String::from("command")],
            ..Pane::default()
        }
    )
}

#[test]
fn pane_2nd_form_fails_to_deserialize_from_boolean_with_name() {
    let yaml = r#"
        pane name: true
    "#;

    let result = serde_yaml::from_str::<Pane>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("pane field \"pane name\" cannot be a boolean"));
}

#[test]
fn pane_2nd_form_fails_to_deserialize_from_boolean_with_null_name() {
    let yaml = r#"
        ~: true
    "#;

    let result = serde_yaml::from_str::<Pane>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("invalid value for pane: true"));
}

#[test]
fn pane_2nd_form_fails_to_deserialize_from_number_with_name() {
    let yaml = r#"
        pane name: 42
    "#;

    let result = serde_yaml::from_str::<Pane>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("pane field \"pane name\" cannot be a number"));
}

#[test]
fn pane_2nd_form_fails_to_deserialize_from_number_with_null_name() {
    let yaml = r#"
        ~: 42
    "#;

    let result = serde_yaml::from_str::<Pane>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("invalid value for pane: 42"));
}

#[test]
fn pane_2nd_form_deserializes_correctly_from_command_list_with_name() {
    let yaml = r#"
        pane name:
            - command1
            - command2
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        pane,
        Pane {
            name: Some(String::from("pane name")),
            commands: vec![String::from("command1"), String::from("command2")],
            ..Pane::default()
        }
    )
}

#[test]
fn pane_2nd_form_deserializes_correctly_from_command_list_with_null_name() {
    let yaml = r#"
        ~:
            - command1
            - command2
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        pane,
        Pane {
            name: None,
            commands: vec![String::from("command1"), String::from("command2")],
            ..Pane::default()
        }
    )
}

#[test]
fn pane_2nd_form_deserializes_correctly_from_single_command_with_null_name() {
    let yaml = r#"
        ~: command
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        pane,
        Pane {
            name: None,
            commands: vec![String::from("command")],
            ..Pane::default()
        }
    )
}

#[test]
fn pane_2nd_form_deserializes_split_size_null() {
    let yaml = r#"
        pane:
            split_size:
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.split_size, None);
}

#[test]
fn pane_2nd_form_deserializes_split_size_string() {
    let yaml = r#"
        pane:
            split_size: "75%"
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.split_size, Some(String::from("75%")));
}

#[test]
fn pane_2nd_form_deserializes_split_size_number() {
    let yaml = r#"
        pane:
            split_size: 42
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.split_size, Some(String::from("42")));
}

#[test]
fn pane_2nd_form_raises_error_on_invalid_split_size_value() {
    let yaml = r#"
        pane:
            split_size:
                - 42
    "#;

    let result = serde_yaml::from_str::<Pane>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("data did not match any variant of untagged enum PaneOption"));
}

#[test]
fn pane_2nd_form_deserializes_split_h() {
    let yaml = r#"
        pane:
            split: h
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.split, Some(PaneSplit::Horizontal));
}

#[test]
fn pane_2nd_form_deserializes_split_horizontal() {
    let yaml = r#"
        pane:
            split: horizontal
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.split, Some(PaneSplit::Horizontal));
}

#[test]
fn pane_2nd_form_deserializes_split_v() {
    let yaml = r#"
        pane:
            split: v
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.split, Some(PaneSplit::Vertical));
}

#[test]
fn pane_2nd_form_deserializes_split_vertical() {
    let yaml = r#"
        pane:
            split: vertical
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.split, Some(PaneSplit::Vertical));
}

#[test]
fn pane_2nd_form_raises_error_on_invalid_split_value() {
    let yaml = r#"
        pane:
            split: o
    "#;

    let result = serde_yaml::from_str::<Pane>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("data did not match any variant of untagged enum PaneOption"));
}

#[test]
fn pane_3rd_form_deserializes_correctly_with_name() {
    let yaml = r#"
        some name:
            name: pane name
            working_dir: /home
            split: v
            split_from: 1
            split_size: 42%
            clear: true
            on_create: echo on_create
            post_create: echo post_create
            command: echo command
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        pane,
        Pane {
            name: Some(String::from("pane name")),
            working_dir: Some(PathBuf::from("/home")),
            split: Some(PaneSplit::Vertical),
            split_from: Some(1),
            split_size: Some(String::from("42%")),
            clear: true,
            on_create: vec![String::from("echo on_create")],
            post_create: vec![String::from("echo post_create")],
            commands: vec![String::from("echo command")],
        }
    )
}

#[test]
fn pane_3rd_form_deserializes_correctly_with_null_name() {
    let yaml = r#"
        some name:
            name: ~
            working_dir: /home
            split: h
            split_from: 1
            split_size: 42%
            clear: true
            on_create: echo on_create
            post_create: echo post_create
            command: echo command
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        pane,
        Pane {
            name: None,
            working_dir: Some(PathBuf::from("/home")),
            split: Some(PaneSplit::Horizontal),
            split_from: Some(1),
            split_size: Some(String::from("42%")),
            clear: true,
            on_create: vec![String::from("echo on_create")],
            post_create: vec![String::from("echo post_create")],
            commands: vec![String::from("echo command")],
        }
    )
}

#[test]
fn pane_3rd_form_deserializes_correctly_with_id() {
    let yaml = r#"
        some name:
            name: pane name
            working_dir: /home
            split: v
            split_from: 1
            split_size: 42%
            clear: true
            on_create: echo on_create
            post_create: echo post_create
            command: echo command
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        pane,
        Pane {
            name: Some(String::from("pane name")),
            working_dir: Some(PathBuf::from("/home")),
            split: Some(PaneSplit::Vertical),
            split_from: Some(1),
            split_size: Some(String::from("42%")),
            clear: true,
            on_create: vec![String::from("echo on_create")],
            post_create: vec![String::from("echo post_create")],
            commands: vec![String::from("echo command")],
        }
    )
}

#[test]
fn pane_3rd_form_deserializes_correctly_with_null_id() {
    let yaml = r#"
        ~:
            name: pane name
            working_dir: /home
            split: h
            split_from: 1
            split_size: 42%
            clear: true
            on_create: echo on_create
            post_create: echo post_create
            command: echo command
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        pane,
        Pane {
            name: Some(String::from("pane name")),
            working_dir: Some(PathBuf::from("/home")),
            split: Some(PaneSplit::Horizontal),
            split_from: Some(1),
            split_size: Some(String::from("42%")),
            clear: true,
            on_create: vec![String::from("echo on_create")],
            post_create: vec![String::from("echo post_create")],
            commands: vec![String::from("echo command")],
        }
    )
}

#[test]
fn pane_raises_error_on_invalid_split_from_value() {
    let yaml = r#"
        split_from:
          - 42
    "#;

    let result = serde_yaml::from_str::<Pane>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("data did not match any variant of untagged enum PaneOption"));
}

#[test]
fn pane_deserializes_post_create() {
    let yaml = r#"
        post_create:
          - display cmd1
          - display cmd2
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.post_create, vec!["display cmd1", "display cmd2"])
}

#[test]
fn pane_deserializes_sequence_as_command() {
    let yaml = r#"
        - echo cmd1
        - echo cmd2
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.commands, vec!["echo cmd1", "echo cmd2"]);
}

#[test]
fn pane_deserializes_null_command() {
    let yaml = r#"
        command: ~
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.commands.len(), 0);
}

#[test]
fn pane_deserializes_string_command() {
    let yaml = r#"
        command: "echo cmd1"
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.commands, vec!["echo cmd1"]);
}

#[test]
fn pane_deserializes_sequence_commands() {
    let yaml = r#"
        commands:
          - echo cmd1
          - echo cmd2
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.commands, vec!["echo cmd1", "echo cmd2"]);
}

#[test]
fn pane_raises_error_on_invalid_commands_value() {
    let yaml = r#"
        command:
          map: as_a_command
    "#;

    let result = serde_yaml::from_str::<Pane>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("data did not match any variant of untagged enum PaneOption"));
}

#[test]
fn pane_from_string_translates_to_single_command_pane() {
    let command = "echo hello";

    let pane = Pane::from(command);

    assert_eq!(pane.commands.len(), 1);
    assert_eq!(pane.commands[0], command);
}
