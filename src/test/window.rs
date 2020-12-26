use super::*;

use crate::pane_split::PaneSplit;
use tempfile::tempdir;

use std::fs;

#[test]
fn window_check_succeeds_on_valid_window() {
    let window = Window {
        name: Some(String::from("window")),
        ..Window::default()
    };

    let result = window.check(1);
    assert!(result.is_ok());
}

#[test]
fn window_check_fails_on_invalid_name() {
    let window = Window {
        name: Some(String::from("window:1")),
        ..Window::default()
    };

    let result = window.check(1);
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "name \"window:1\" cannot contain the following characters: .: "
    );

    let window = Window {
        name: Some(String::from("window.1")),
        ..Window::default()
    };

    let result = window.check(1);
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "name \"window.1\" cannot contain the following characters: .: "
    );
}

#[test]
fn window_check_fails_when_pane_split_from_is_out_of_bounds() {
    let window = Window {
        panes: vec![Pane {
            split_from: Some(2),
            ..Pane::default()
        }],
        ..Window::default()
    };
    assert_eq!(window.panes.len(), 1);

    let result = window.check(1);
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "split_from: there is no pane with index 2 (pane indexes always start at pane_base_index)"
    )
}

#[test]
fn window_check_succeeds_when_working_dir_is_a_existing_dir() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();

    let window = Window {
        working_dir: Some(temp_dir),
        ..Window::default()
    };
    let result = window.check(1);
    assert!(result.is_ok());
}

#[test]
fn window_check_fails_when_working_dir_is_missing() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();

    // Does not exist
    let working_dir = temp_dir.join("random_dirname");
    let window = Window {
        working_dir: Some(working_dir.to_owned()),
        ..Window::default()
    };
    let result = window.check(1);
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        format!(
            "window working_dir {:?} is not a directory or does not exist",
            working_dir
        ),
    );
}

#[test]
fn window_check_fails_when_working_dir_is_not_a_directory() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();

    // Exists but not a directory
    let working_dir = temp_dir.join("some_filename");
    let file = fs::File::create(&working_dir).unwrap();
    file.sync_all().unwrap();
    drop(file);
    assert!(working_dir.is_file());

    let window = Window {
        working_dir: Some(working_dir.to_owned()),
        ..Window::default()
    };
    let result = window.check(1);
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        format!(
            "window working_dir {:?} is not a directory or does not exist",
            working_dir,
        ),
    );
}

#[test]
fn window_check_fails_when_layout_and_split_are_both_used() {
    let window = Window {
        layout: Some(String::from("main-vertical")),
        panes: vec![Pane {
            split: Some(PaneSplit::Vertical),
            ..Pane::default()
        }],
        ..Window::default()
    };

    let result = window.check(1);
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "layout: cannot use layout when sub-panes use split or split_size",
    )
}

#[test]
fn window_check_fails_when_layout_and_split_size_are_both_used() {
    let window = Window {
        layout: Some(String::from("main-vertical")),
        panes: vec![Pane {
            split_size: Some(String::from("50%")),
            ..Pane::default()
        }],
        ..Window::default()
    };

    let result = window.check(1);
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "layout: cannot use layout when sub-panes use split or split_size",
    )
}

#[test]
fn window_1st_form_deserializes_from_null() {
    let yaml = r#"
        ~
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(window, Window::default());
}

#[test]
fn window_1st_form_deserializes_from_single_command() {
    let yaml = r#"
        echo hello
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        window,
        Window {
            panes: vec![Pane::from("echo hello")],
            ..Window::default()
        }
    );
}

#[test]
fn window_1st_form_deserializes_from_multiple_commands() {
    let yaml = r#"
        - echo pane1
        - echo pane2
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        window,
        Window {
            panes: vec![Pane::from("echo pane1"), Pane::from("echo pane2")],
            ..Window::default()
        }
    );
}

#[test]
fn window_1st_form_deserializes_correctly_with_key_name() {
    let yaml = r#"
        my name:
        working_dir: /home
        layout: main-vertical
        on_create: echo on_create
        post_create: echo post_create
        on_pane_create: echo on_pane_create
        post_pane_create: echo post_pane_create
        pane_command: echo pane_command
        clear_panes: true
        panes: echo pane
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        window,
        Window {
            name: Some(String::from("my name")),
            working_dir: Some(PathBuf::from("/home")),
            layout: Some(String::from("main-vertical")),
            on_create: vec![String::from("echo on_create")],
            post_create: vec![String::from("echo post_create")],
            on_pane_create: vec![String::from("echo on_pane_create")],
            post_pane_create: vec![String::from("echo post_pane_create")],
            pane_commands: vec![String::from("echo pane_command")],
            clear_panes: true,
            panes: vec![Pane::from("echo pane")],
        }
    );
}

#[test]
fn window_1st_form_deserializes_correctly_with_null_key_name() {
    let yaml = r#"
        ~:
        working_dir: /home
        layout: main-vertical
        on_create: echo on_create
        post_create: echo post_create
        on_pane_create: echo on_pane_create
        post_pane_create: echo post_pane_create
        pane_command: echo pane_command
        clear_panes: true
        panes: echo pane
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        window,
        Window {
            name: None,
            working_dir: Some(PathBuf::from("/home")),
            layout: Some(String::from("main-vertical")),
            on_create: vec![String::from("echo on_create")],
            post_create: vec![String::from("echo post_create")],
            on_pane_create: vec![String::from("echo on_pane_create")],
            post_pane_create: vec![String::from("echo post_pane_create")],
            pane_commands: vec![String::from("echo pane_command")],
            clear_panes: true,
            panes: vec![Pane::from("echo pane")],
        }
    );
}

#[test]
fn window_1st_form_fails_when_key_name_is_not_first_line() {
    let yaml = r#"
        working_dir: /home
        some name:
        layout: main-vertical
        on_create: echo on_create
        post_create: echo post_create
        on_pane_create: echo on_pane_create
        post_pane_create: echo post_pane_create
        pane_command: echo pane_command
        clear_panes: true,
        panes: echo pane
    "#;

    let result = serde_yaml::from_str::<Window>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("window field \"some name\" cannot be null"));
}

#[test]
fn window_1st_form_fails_when_null_key_name_is_not_first_line() {
    let yaml = r#"
        working_dir: /home
        ~:
        layout: main-vertical
        on_create: echo on_create
        post_create: echo post_create
        on_pane_create: echo on_pane_create
        post_pane_create: echo post_pane_create
        pane_command: echo pane_command
        clear_panes: true,
        panes: echo pane
    "#;

    let result = serde_yaml::from_str::<Window>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("null name can only be set as first element of the map"));
}

#[test]
fn window_1st_form_deserializes_correctly_with_explicit_name() {
    let yaml = r#"
        working_dir: /home
        layout: main-vertical
        on_create: echo on_create
        post_create: echo post_create
        on_pane_create: echo on_pane_create
        post_pane_create: echo post_pane_create
        pane_command: echo pane_command
        clear_panes: true
        panes: echo pane
        name: my name
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        window,
        Window {
            name: Some(String::from("my name")),
            working_dir: Some(PathBuf::from("/home")),
            layout: Some(String::from("main-vertical")),
            on_create: vec![String::from("echo on_create")],
            post_create: vec![String::from("echo post_create")],
            on_pane_create: vec![String::from("echo on_pane_create")],
            post_pane_create: vec![String::from("echo post_pane_create")],
            pane_commands: vec![String::from("echo pane_command")],
            clear_panes: true,
            panes: vec![Pane::from("echo pane")],
        }
    );
}

#[test]
fn window_1st_form_deserializes_correctly_deserializes_command_lists() {
    let yaml = r#"
        on_create:
            - echo on_create1
            - echo on_create2
        post_create:
            - echo post_create1
            - echo post_create2
        on_pane_create:
            - echo on_pane_create1
            - echo on_pane_create2
        post_pane_create:
            - echo post_pane_create1
            - echo post_pane_create2
        pane_commands:
            - echo pane_command1
            - echo pane_command2
        panes:
            - echo pane1
            - echo pane2
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        window,
        Window {
            on_create: vec![
                String::from("echo on_create1"),
                String::from("echo on_create2")
            ],
            post_create: vec![
                String::from("echo post_create1"),
                String::from("echo post_create2")
            ],
            on_pane_create: vec![
                String::from("echo on_pane_create1"),
                String::from("echo on_pane_create2")
            ],
            post_pane_create: vec![
                String::from("echo post_pane_create1"),
                String::from("echo post_pane_create2")
            ],
            pane_commands: vec![
                String::from("echo pane_command1"),
                String::from("echo pane_command2")
            ],
            panes: vec![Pane::from("echo pane1"), Pane::from("echo pane2")],
            ..Window::default()
        }
    );
}

#[test]
fn window_1st_form_does_not_accept_unknown_fields() {
    let yaml = r#"
        window:
        unknown_field: value
    "#;

    let result = serde_yaml::from_str::<Window>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("window field \"unknown_field\" cannot be a string"));
}

#[test]
fn window_1st_form_fails_when_a_field_does_not_accept_command_list() {
    let yaml = r#"
        window:
        layout:
            - command1
            - command2
    "#;

    let result = serde_yaml::from_str::<Window>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("window field \"layout\" cannot be a command list"));
}

#[test]
fn window_1st_form_fails_when_a_field_does_not_accept_window_definiton() {
    let yaml = r#"
        window:
        layout:
            layout: vertical
            panes:
                - command1
                - command2
    "#;

    let result = serde_yaml::from_str::<Window>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("window field \"layout\" cannot be a window definition"));
}

#[test]
fn window_1st_form_fails_when_a_field_does_not_accept_window_definiton_with_name() {
    let yaml = r#"
        window:
        layout:
            name: window name
            layout: vertical
            panes:
                - command1
                - command2
    "#;

    let result = serde_yaml::from_str::<Window>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("window field \"layout\" cannot be a window definition"));
}

#[test]
fn window_1st_form_fails_when_a_field_does_not_accept_a_pane_list() {
    let yaml = r#"
        window:
        layout:
            - commands:
                - command1
                - command2
            - commands:
                - command3
                - command4
    "#;

    let result = serde_yaml::from_str::<Window>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("window field \"layout\" cannot be a pane list"));
}

#[test]
fn window_1st_form_fails_when_clear_panes_has_an_invalid_value() {
    let yaml = r#"
        window:
        clear_panes: hello
    "#;

    let result: Result<Window, _> = serde_yaml::from_str(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("window field \"clear_panes\" cannot be a string"));
}

#[test]
fn window_2nd_form_deserializes_from_null() {
    let yaml = r#"
        win1:
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(window.name, Some(String::from("win1")));
    assert_eq!(window.panes, vec![serde_yaml::from_str("~").unwrap()]);
}

#[test]
fn window_2nd_form_deserializes_from_str_with_name() {
    let yaml = r#"
        win1: echo cmd
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(window.name, Some(String::from("win1")));
    assert_eq!(
        window.panes,
        vec![serde_yaml::from_str("echo cmd").unwrap()]
    );
}

#[test]
fn window_2nd_form_deserializes_from_str_with_null_name() {
    let yaml = r#"
        ~: echo cmd
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(window.name, None);
    assert_eq!(
        window.panes,
        vec![serde_yaml::from_str("echo cmd").unwrap()]
    );
}

#[test]
fn window_2nd_form_deserializes_from_sequence_with_name() {
    let yaml = r#"
        win1:
          - echo cmd1
          - echo cmd2
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(window.name, Some(String::from("win1")));
    assert_eq!(
        window.panes,
        vec![
            serde_yaml::from_str("echo cmd1").unwrap(),
            serde_yaml::from_str("echo cmd2").unwrap(),
        ]
    );
}

#[test]
fn window_2nd_form_deserializes_from_sequence_with_null_name() {
    let yaml = r#"
        ~:
          - echo cmd1
          - echo cmd2
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(window.name, None);
    assert_eq!(
        window.panes,
        vec![
            serde_yaml::from_str("echo cmd1").unwrap(),
            serde_yaml::from_str("echo cmd2").unwrap(),
        ]
    );
}

#[test]
fn window_2nd_form_deserializes_correctly_with_name() {
    let yaml = r#"
        my name:
            working_dir: /home
            layout: main-vertical
            on_create: echo on_create
            post_create: echo post_create
            on_pane_create: echo on_pane_create
            post_pane_create: echo post_pane_create
            pane_command: echo pane_command
            clear_panes: true
            panes: echo pane
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        window,
        Window {
            name: Some(String::from("my name")),
            working_dir: Some(PathBuf::from("/home")),
            layout: Some(String::from("main-vertical")),
            on_create: vec![String::from("echo on_create")],
            post_create: vec![String::from("echo post_create")],
            on_pane_create: vec![String::from("echo on_pane_create")],
            post_pane_create: vec![String::from("echo post_pane_create")],
            pane_commands: vec![String::from("echo pane_command")],
            clear_panes: true,
            panes: vec![Pane::from("echo pane")],
        }
    );
}

#[test]
fn window_2nd_form_deserializes_correctly_with_null_name() {
    let yaml = r#"
        ~:
            working_dir: /home
            layout: main-vertical
            on_create: echo on_create
            post_create: echo post_create
            on_pane_create: echo on_pane_create
            post_pane_create: echo post_pane_create
            pane_command: echo pane_command
            clear_panes: true
            panes: echo pane
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        window,
        Window {
            name: None,
            working_dir: Some(PathBuf::from("/home")),
            layout: Some(String::from("main-vertical")),
            on_create: vec![String::from("echo on_create")],
            post_create: vec![String::from("echo post_create")],
            on_pane_create: vec![String::from("echo on_pane_create")],
            post_pane_create: vec![String::from("echo post_pane_create")],
            pane_commands: vec![String::from("echo pane_command")],
            clear_panes: true,
            panes: vec![Pane::from("echo pane")],
        }
    );
}

#[test]
fn window_2nd_form_deserializes_correctly_from_pane_listwith_null_name() {
    let yaml = r#"
        ~:
            - commands:
                - command1
                - command2
            - commands:
                - command3
                - command4
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        window,
        Window {
            name: None,
            panes: vec![
                Pane::from(vec![String::from("command1"), String::from("command2")]),
                Pane::from(vec![String::from("command3"), String::from("command4")])
            ],
            ..Window::default()
        }
    );
}

#[test]
fn window_2nd_form_raises_error_on_multiple_hashmap_keys() {
    let yaml = r#"
        win1:
        win2:
    "#;

    let result = serde_yaml::from_str::<Window>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("window field \"win2\" cannot be null"));
}

#[test]
fn window_2nd_form_deserializes_name_string() {
    let yaml = r#"
        win1:
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(window.name, Some(String::from("win1")));
}

#[test]
fn window_2nd_form_deserializes_null_name() {
    let yaml = r#"
        ~:
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(window.name, None);
}

#[test]
fn window_2nd_form_raises_error_on_invalid_name_value() {
    let yaml = r#"
        []:
    "#;

    let result = serde_yaml::from_str::<Window>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("invalid type: sequence, expected a string"));
}

#[test]
fn window_2nd_form_raises_error_on_invalid_layout_value() {
    let yaml = r#"
        win1:
            layout: 42
    "#;

    let result = serde_yaml::from_str::<Window>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("data did not match any variant of untagged enum WindowOption"));
}

#[test]
fn window_2nd_form_raises_error_on_invalid_window_definition() {
    let yaml = r#"
        win1: false
    "#;

    let result = serde_yaml::from_str::<Window>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("window field \"win1\" cannot be a boolean"));
}

#[test]
fn window_3rd_form_deserializes_correctly_with_string_key() {
    let yaml = r#"
        some name:
            name: my name
            working_dir: /home
            layout: main-vertical
            on_create: echo on_create
            post_create: echo post_create
            on_pane_create: echo on_pane_create
            post_pane_create: echo post_pane_create
            pane_command: echo pane_command
            clear_panes: true
            panes: echo pane
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        window,
        Window {
            name: Some(String::from("my name")),
            working_dir: Some(PathBuf::from("/home")),
            layout: Some(String::from("main-vertical")),
            on_create: vec![String::from("echo on_create")],
            post_create: vec![String::from("echo post_create")],
            on_pane_create: vec![String::from("echo on_pane_create")],
            post_pane_create: vec![String::from("echo post_pane_create")],
            pane_commands: vec![String::from("echo pane_command")],
            clear_panes: true,
            panes: vec![Pane::from("echo pane")],
        }
    );
}

#[test]
fn window_3rd_form_deserializes_correctly_with_null_key() {
    let yaml = r#"
        ~:
            name: my name
            working_dir: /home
            layout: main-vertical
            on_create: echo on_create
            post_create: echo post_create
            on_pane_create: echo on_pane_create
            post_pane_create: echo post_pane_create
            pane_command: echo pane_command
            clear_panes: true
            panes: echo pane
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        window,
        Window {
            name: Some(String::from("my name")),
            working_dir: Some(PathBuf::from("/home")),
            layout: Some(String::from("main-vertical")),
            on_create: vec![String::from("echo on_create")],
            post_create: vec![String::from("echo post_create")],
            on_pane_create: vec![String::from("echo on_pane_create")],
            post_pane_create: vec![String::from("echo post_pane_create")],
            pane_commands: vec![String::from("echo pane_command")],
            clear_panes: true,
            panes: vec![Pane::from("echo pane")],
        }
    );
}

#[test]
fn window_3rd_form_overrides_null_name() {
    let yaml = r#"
        some name:
            name: ~
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(window.name, None);
}

#[test]
fn window_deserializes_empty_pane_list() {
    let yaml = r#"
        win1:
            panes: ~
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(window.panes, Window::default_panes());
}

#[test]
fn window_deserializes_pane_keyword_as_panes() {
    let yaml = r#"
        name: win1
        pane:
            - commands:
                - command1
                - command2
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        window,
        Window {
            name: Some(String::from("win1")),
            panes: vec![Pane::from(vec![
                String::from("command1"),
                String::from("command2"),
            ])],
            ..Window::default()
        }
    );
}
