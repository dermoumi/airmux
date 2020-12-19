use super::*;

use shellexpand::tilde;
use tempfile::tempdir;

use std::fs;

fn make_config(tmux_command: Option<OsString>, config_dir: Option<PathBuf>) -> Config {
    Config {
        app_name: "test_app_name",
        app_author: "test_app_author",
        tmux_command,
        config_dir,
    }
}

#[test]
fn project_coerces_from_none() {
    let project = Project::from(None);
    assert_eq!(project, Project::default());
}

#[test]
fn project_coerces_from_some() {
    let dummy_project = Project {
        session_name: Some(String::from("my session name")),
        ..Project::default()
    };

    let project = Project::from(Some(dummy_project.to_owned()));
    assert_eq!(project, dummy_project);
}

#[test]
fn project_prepare_replaces_session_name_when_none() {
    let config = make_config(None, None);

    let project = Project {
        working_dir: Some(PathBuf::from("/")),
        tmux_command: None,
        ..Project::default()
    };
    assert_eq!(project.working_dir, Some(PathBuf::from("/")));
    assert_eq!(project.session_name, None);

    let project = project.prepare(&config, "project", None);
    assert_eq!(project.working_dir, Some(PathBuf::from("/")));
    assert_eq!(project.session_name, Some(String::from("project")));
}

#[test]
fn project_prepare_replaces_attach_when_force_attach_is_set() {
    let config = make_config(None, None);

    let project = Project {
        working_dir: Some(PathBuf::from("/")),
        attach: false,
        ..Project::default()
    };
    assert_eq!(project.working_dir, Some(PathBuf::from("/")));
    assert_eq!(project.attach, false);

    let project = project.prepare(&config, "project", Some(true));
    assert_eq!(project.working_dir, Some(PathBuf::from("/")));
    assert_eq!(project.attach, true);

    // --

    let project = Project {
        working_dir: Some(PathBuf::from("/")),
        attach: true,
        ..Project::default()
    };
    assert_eq!(project.working_dir, Some(PathBuf::from("/")));
    assert_eq!(project.attach, true);

    let project = project.prepare(&config, "project", Some(false));
    assert_eq!(project.working_dir, Some(PathBuf::from("/")));
    assert_eq!(project.attach, false);
}

#[test]
fn project_prepare_replaces_tmux_command_if_set_in_config() {
    let tmux_command = OsString::from("other_tmux");
    let config = make_config(Some(tmux_command.to_owned()), None);

    // When it's not definied in project file
    let project = Project::default().prepare(&config, "project_name", None);
    assert_eq!(project.tmux_command.unwrap().as_str(), tmux_command);

    // When it's not defined at all
    let project = Project {
        tmux_command: Some(String::from("dummy_tmux_command")),
        ..Project::default()
    }
    .prepare(&config, "project_name", None);
    assert_eq!(project.tmux_command.unwrap().as_str(), tmux_command);
}

#[test]
fn project_prepare_sets_tmux_default_command_when_empty() {
    let config = make_config(None, None);

    let project = Project::default().prepare(&config, "project_name", None);
    assert_eq!(project.tmux_command.unwrap().as_str(), "tmux");
}

#[test]
fn project_check_succeeds_on_valid_project() {
    let project = Project {
        session_name: Some(String::from("project")),
        ..Project::default()
    };

    let result = project.check();
    assert!(result.is_ok());
}

#[test]
fn project_check_fails_on_invalid_session_name() {
    let project = Project {
        session_name: Some(String::from("project:1")),
        ..Project::default()
    };

    let result = project.check();
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "name \"project:1\" cannot contain the following characters: .: "
    );

    let project = Project {
        session_name: Some(String::from("project.1")),
        ..Project::default()
    };

    let result = project.check();
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "name \"project.1\" cannot contain the following characters: .: "
    );
}

#[test]
fn project_check_fails_on_invalid_startup_window() {
    // With window index (too hight)
    let project = Project {
        startup_window: StartupWindow::Index(2),
        window_base_index: 1,
        windows: vec![Window::default()],
        ..Project::default()
    };
    let result = project.check();
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "startup_window: there is no window with index 2",
    );

    // With window index (too low)
    let project = Project {
        startup_window: StartupWindow::Index(0),
        window_base_index: 1,
        windows: vec![Window::default()],
        ..Project::default()
    };
    let result = project.check();
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "startup_window: there is no window with index 0",
    );

    // With window name
    let project = Project {
        startup_window: StartupWindow::Name(String::from("window51")),
        ..Project::default()
    };
    let result = project.check();
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "startup_window: there is no window with name \"window51\"",
    );
}

#[test]
fn project_check_succeeds_when_working_dir_is_a_existing_dir() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();

    let project = Project {
        working_dir: Some(temp_dir),
        ..Project::default()
    };
    let result = project.check();
    assert!(result.is_ok());
}

#[test]
fn project_check_fails_when_working_dir_is_missing() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();

    // Does not exist
    let working_dir = temp_dir.join("random_dirname");
    let project = Project {
        working_dir: Some(working_dir.to_owned()),
        ..Project::default()
    };
    let result = project.check();
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        format!(
            "project working_dir {:?} is not a directory or does not exist",
            working_dir
        ),
    );
}

#[test]
fn project_check_fails_when_working_dir_is_not_a_directory() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();

    // Exists but not a directory
    let working_dir = temp_dir.join("some_filename");
    let file = fs::File::create(&working_dir).unwrap();
    file.sync_all().unwrap();
    drop(file);
    assert!(working_dir.is_file());

    let project = Project {
        working_dir: Some(working_dir.to_owned()),
        ..Project::default()
    };
    let result = project.check();
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        format!(
            "project working_dir {:?} is not a directory or does not exist",
            working_dir,
        ),
    );
}

#[test]
fn project_get_tmux_command_splits_command_and_appends_options() {
    let project = Project {
        tmux_command: Some(String::from("tmux -o1 option1")),
        tmux_socket: Some(String::from("socket")),
        tmux_options: Some(String::from("-o2 option2")),
        ..Project::default()
    };

    let (command, args) = project
        .get_tmux_command(vec![OsString::from("-o3"), OsString::from("option3")])
        .unwrap();

    assert_eq!(command, "tmux");
    assert_eq!(
        args,
        vec![
            OsString::from("-o1"),
            OsString::from("option1"),
            OsString::from("-L"),
            OsString::from("socket"),
            OsString::from("-o2"),
            OsString::from("option2"),
            OsString::from("-o3"),
            OsString::from("option3"),
        ],
    );
}

#[test]
fn project_get_tmux_command_for_template_returns_joined_quoted_params() {
    let project = Project {
        tmux_command: Some(String::from("tmux -o1 'op tion1'")),
        tmux_socket: Some(String::from("socket")),
        tmux_options: Some(String::from("-o2 option2")),
        ..Project::default()
    };

    let command = project.get_tmux_command_for_template().unwrap();
    assert_eq!(command, "tmux -o1 'op tion1' -L socket -o2 option2");
}

#[test]
fn project_get_tmux_command_for_template_returns_single_command() {
    let project = Project {
        tmux_command: Some(String::from("tmux")),
        ..Project::default()
    };

    let command = project.get_tmux_command_for_template().unwrap();
    assert_eq!(command, "tmux");
}

#[test]
fn project_deserializes_correctly() {
    let yaml = r#"
        name: project
        tmux_command: teemux
        tmux_options: -d option-d
        tmux_socket: soquette
        root: /database
        window_base_index: 101
        pane_base_index: 102
        startup_window: 103
        startup_pane: 104
        on_start: echo on_start
        on_first_start: echo on_first_start
        on_restart: echo on_restart
        on_exit: echo on_exit
        on_stop: echo on_stop
        post_create: echo post_create
        on_pane_create: echo on_pane_create
        post_pane_create: echo post_pane_create
        pane_command: echo pane_command
        attach: false
        template: tis but a scratch
        window: echo not_a_portal
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        project,
        Project {
            session_name: Some(String::from("project")),
            tmux_command: Some(String::from("teemux")),
            tmux_options: Some(String::from("-d option-d")),
            tmux_socket: Some(String::from("soquette")),
            working_dir: Some(PathBuf::from("/database")),
            window_base_index: 101,
            pane_base_index: 102,
            startup_window: StartupWindow::Index(103),
            startup_pane: Some(104),
            on_start: vec![String::from("echo on_start")],
            on_first_start: vec![String::from("echo on_first_start")],
            on_restart: vec![String::from("echo on_restart")],
            on_exit: vec![String::from("echo on_exit")],
            on_stop: vec![String::from("echo on_stop")],
            post_create: vec![String::from("echo post_create")],
            on_pane_create: vec![String::from("echo on_pane_create")],
            post_pane_create: vec![String::from("echo post_pane_create")],
            pane_commands: vec![String::from("echo pane_command")],
            attach: false,
            template: ProjectTemplate::Raw(String::from("tis but a scratch")),
            windows: vec![Window::from("echo not_a_portal")],
        }
    );
}

#[test]
fn project_deserializes_from_null() {
    let yaml = r#"
        ~
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(project, Project::default());
}

#[test]
fn project_deserializer_accepts_empty_values() {
    // working_dir treats empty value as home, so we exclude it from this test

    let yaml = r#"
        name:
        tmux_command:
        tmux_options:
        tmux_socket:
        window_base_index:
        pane_base_index:
        startup_window:
        startup_pane:
        on_start:
        on_first_start:
        on_restart:
        on_exit:
        on_stop:
        post_create:
        on_pane_create:
        post_pane_create:
        pane_command:
        attach:
        template:
        window:
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(project, Project::default());
}

#[test]
fn project_deserializer_accepts_single_window_command() {
    let yaml = r#"
        window: echo hello
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(project.windows, vec![Window::from("echo hello")]);
}

#[test]
fn project_deserializer_accepts_multiple_window_commands() {
    let yaml = r#"
        windows:
          - echo hello
          - echo world
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        project.windows,
        vec![Window::from("echo hello"), Window::from("echo world")]
    );
}

#[test]
fn project_deserializer_rejects_unknown_fields() {
    let yaml = r#"
        unknown_field_51: hello
    "#;

    let result = serde_yaml::from_str::<Project>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("unknown field `unknown_field_51`"));
}

#[test]
fn project_deserializer_raises_error_when_both_attach_and_detached_are_set() {
    let yaml = r#"
        attach: true
        detached: false
    "#;

    let result = serde_yaml::from_str::<Project>(yaml);
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "cannot set both 'attach' and 'detached' fields",
    );
}

#[test]
fn project_deserializer_attach_default_when_neither_attach_or_detach_are_set() {
    let yaml = r#"
        ~
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(project.attach, Project::default_attach());
}

#[test]
fn project_deserializer_attach_value_is_set_correctly_when_attach_is_set() {
    let yaml = r#"
        attach: true
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(project.attach, true);

    let yaml = r#"
        attach: false
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(project.attach, false);
}

#[test]
fn project_deserializer_attach_value_is_set_correctly_when_detached_is_set() {
    let yaml = r#"
        detached: true
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(project.attach, false);

    let yaml = r#"
        detached: false
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(project.attach, true);
}

#[test]
fn project_deserializes_working_dir() {
    let yaml = r#"
        working_dir: /path1
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(project.working_dir, Some(PathBuf::from("/path1")));

    let yaml = r#"
        root: /path2
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(project.working_dir, Some(PathBuf::from("/path2")));
}

#[test]
fn project_deserializes_working_dir_null_as_home() {
    let yaml = r#"
        working_dir:
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        project.working_dir,
        Some(PathBuf::from(tilde("~").to_string()))
    );

    let yaml = r#"
        root: ~
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        project.working_dir,
        Some(PathBuf::from(tilde("~").to_string()))
    );
}

#[test]
fn project_startup_window_by_index() {
    let yaml = r#"
        startup_window: 42
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(project.startup_window, StartupWindow::Index(42));
}

#[test]
fn project_startup_window_by_name() {
    let yaml = r#"
        startup_window: my_window
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        project.startup_window,
        StartupWindow::Name(String::from("my_window"))
    );
}

#[test]
fn project_raises_error_on_invalid_working_dir_value() {
    let yaml = r#"
        working_dir:
          - path_in_a_list
    "#;

    let result = serde_yaml::from_str::<Project>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("invalid type: sequence, expected path string"));
}

#[test]
fn project_on_create_deserializes_as_on_first_start() {
    let yaml = r#"
        on_create: echo on_create
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(project.on_first_start, vec![String::from("echo on_create")]);
}

#[test]
fn project_pane_no_command_serializes_to_an_empty_string() {
    let mut project = Project::default();
    project.windows[0].panes[0] = Pane {
        commands: vec![],
        ..Pane::default()
    };

    let output = project.serialize_compact(false).unwrap();
    let expected_output = r#"---
{}"#;

    assert_eq!(output, expected_output);
}

#[test]
fn project_pane_single_command_serializes_to_a_single_string() {
    let mut project = Project::default();
    project.windows[0].panes[0] = Pane {
        commands: vec![String::from("echo cmd1")],
        ..Pane::default()
    };

    let output = project.serialize_compact(false).unwrap();
    let expected_output = r#"---
windows:
  - name: ~
    panes:
      - echo cmd1"#;

    assert_eq!(output, expected_output);
}

#[test]
fn project_pane_two_or_more_commands_serializes_to_a_full_object() {
    let mut project = Project::default();
    project.windows[0].panes[0] = Pane {
        commands: vec![String::from("echo cmd1"), String::from("echo cmd2")],
        ..Pane::default()
    };

    let output = project.serialize_compact(false).unwrap();
    let expected_output = r#"---
windows:
  - name: ~
    panes:
      - commands:
          - echo cmd1
          - echo cmd2"#;

    assert_eq!(output, expected_output);
}
