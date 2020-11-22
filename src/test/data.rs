use super::*;

fn make_config(tmux_command: Option<OsString>, config_dir: Option<PathBuf>) -> Config {
    Config {
        app_name: "test_app_name",
        app_author: "test_app_author",
        tmux_command: Some(tmux_command.unwrap_or(OsString::from("tmux"))),
        config_dir,
    }
}

#[test]
fn project_prepare_replaces_session_name_when_none() {
    let config = make_config(None, None);

    let project = Project {
        working_dir: Some("/".into()),
        ..Project::default()
    };
    assert_eq!(project.working_dir, Some("/".into()));
    assert_eq!(project.session_name, None);

    let project = project.prepare(&config, "project", None);
    assert_eq!(project.working_dir, Some("/".into()));
    assert_eq!(project.session_name, Some("project".into()));
}

#[test]
fn project_prepare_replaces_attach_when_force_attach_is_set() {
    let config = make_config(None, None);

    let project = Project {
        working_dir: Some("/".into()),
        attach: false,
        ..Project::default()
    };
    assert_eq!(project.working_dir, Some("/".into()));
    assert_eq!(project.attach, false);

    let project = project.prepare(&config, "project", Some(true));
    assert_eq!(project.working_dir, Some("/".into()));
    assert_eq!(project.attach, true);

    let project = Project {
        working_dir: Some("/".into()),
        attach: true,
        ..Project::default()
    };
    assert_eq!(project.working_dir, Some("/".into()));
    assert_eq!(project.attach, true);

    let project = project.prepare(&config, "project", Some(false));
    assert_eq!(project.working_dir, Some("/".into()));
    assert_eq!(project.attach, false);
}

#[test]
fn project_check_succeeds_on_valid_project() {
    let project = Project {
        session_name: Some("project".into()),
        ..Project::default()
    };

    let result = project.check();
    assert!(result.is_ok());
}

#[test]
fn project_check_fails_on_invalid_session_name() {
    let project = Project {
        session_name: Some("project:1".into()),
        ..Project::default()
    };

    let result = project.check();
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "name \"project:1\" cannot contain the following characters: .: "
    );

    let project = Project {
        session_name: Some("project.1".into()),
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
fn project_deserializes_from_minimal_yaml() {
    let yaml = r#"
        name: ~
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(project, Project::default());
}

#[test]
fn window_check_succeeds_on_valid_window() {
    let window = Window {
        name: Some("window".into()),
        ..Window::default()
    };

    let result = window.check();
    assert!(result.is_ok());
}

#[test]
fn window_check_fails_on_invalid_name() {
    let window = Window {
        name: Some("window:1".into()),
        ..Window::default()
    };

    let result = window.check();
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "name \"window:1\" cannot contain the following characters: .: "
    );

    let window = Window {
        name: Some("window.1".into()),
        ..Window::default()
    };

    let result = window.check();
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "name \"window.1\" cannot contain the following characters: .: "
    );
}

#[test]
fn window_deserializes_session_name() {
    let yaml = r#"
        session_name: project1
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(project.session_name, Some("project1".into()));

    let yaml = r#"
        name: project2
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(project.session_name, Some("project2".into()));
}

#[test]
fn window_deserializes_working_dir() {
    let yaml = r#"
        working_dir: /path1
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(project.working_dir, Some("/path1".into()));

    let yaml = r#"
        root: /path2
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(project.working_dir, Some("/path2".into()));
}

#[test]
fn window_deserializes_working_dir_null_as_home() {
    let yaml = r#"
        working_dir: ~
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(project.working_dir, Some("~".into()));

    let yaml = r#"
        root: ~
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(project.working_dir, Some("~".into()));
}

#[test]
fn project_raises_error_on_invalid_working_dir_value() {
    let yaml = r#"
        working_dir: 42
    "#;

    let result = serde_yaml::from_str::<Project>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .starts_with("expected working_dir to be a string or null"));
}

#[test]
fn window_deserializes_windows_from_sequence() {
    let yaml = r#"
        windows:
          - ~
          - win2: echo hello
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        project.windows,
        vec![
            serde_yaml::from_str::<Window>("~").unwrap(),
            serde_yaml::from_str::<Window>("win2: echo hello").unwrap(),
        ]
    );
}

#[test]
fn window_deserializes_windows_from_other() {
    let yaml = r#"
        window: echo hello
    "#;

    let project: Project = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        project.windows,
        vec![serde_yaml::from_str::<Window>("echo hello").unwrap(),]
    );
}

#[test]
fn project_template_deserializes_from_null() {
    let yaml = r#"
        ~
    "#;

    let project_template: ProjectTemplate = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(project_template, ProjectTemplate::Default);
}

#[test]
fn project_template_deserializes_from_string() {
    let yaml = r#"
        my_template
    "#;

    let project_template: ProjectTemplate = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(project_template, ProjectTemplate::Raw("my_template".into()));
}

#[test]
fn project_template_deserializes_from_file_mapping() {
    let yaml = r#"
        file: template.tera
    "#;

    let project_template: ProjectTemplate = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        project_template,
        ProjectTemplate::File("template.tera".into())
    );
}

#[test]
fn project_template_raises_error_on_invalid_value() {
    let yaml = r#"
        42
    "#;

    let result = serde_yaml::from_str::<ProjectTemplate>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .starts_with("invalid value for field 'template':"));

    let yaml = r#"
        not_file: test.file
    "#;

    let result = serde_yaml::from_str::<ProjectTemplate>(yaml);
    assert!(result.is_err());
    assert_eq!(result.err().unwrap().to_string(), "missing 'file' field");

    let yaml = r#"
        file: 42
    "#;

    let result = serde_yaml::from_str::<ProjectTemplate>(yaml);
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "expected file to be a string"
    );
}

#[test]
fn window_raises_error_on_multiple_hashmap_keys() {
    let yaml = r#"
        win1:
        win2:
    "#;

    let result = serde_yaml::from_str::<Window>(yaml);
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "expected window definition to be a single-value hashmap"
    );
}

#[test]
fn window_def_deserializes_name_string() {
    let yaml = r#"
        win1:
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(window.name, Some("win1".into()));
}

#[test]
fn window_def_deserializes_name_null() {
    let yaml = r#"
        ~:
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(window.name, None);
}

#[test]
fn window_def_raises_error_on_invalid_name_value() {
    let yaml = r#"
        42:
    "#;

    let result = serde_yaml::from_str::<Window>(yaml);
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "expected window name to be a string"
    );
}

#[test]
fn window_def_deserializes_working_dir() {
    let yaml = r#"
        win1:
          working_dir: /path1
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(window.name, Some("win1".into()));
    assert_eq!(window.working_dir, Some("/path1".into()));

    let yaml = r#"
        win1:
          root: /path2
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(window.name, Some("win1".into()));
    assert_eq!(window.working_dir, Some("/path2".into()));
}

#[test]
fn window_def_deserializes_working_dir_null_as_home() {
    let yaml = r#"
        win1:
          working_dir: ~
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(window.name, Some("win1".into()));
    assert_eq!(window.working_dir, Some("~".into()));

    let yaml = r#"
        win1:
          root: ~
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(window.name, Some("win1".into()));
    assert_eq!(window.working_dir, Some("~".into()));
}

#[test]
fn window_def_raises_error_on_invalid_working_dir_value() {
    let yaml = r#"
        win1:
            working_dir: 42
    "#;

    let result = serde_yaml::from_str::<Window>(yaml);
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "expected working_dir to be a string or null"
    );
}

#[test]
fn window_def_deserializes_layout() {
    let yaml = r#"
        win1:
          layout: layout
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(window.name, Some("win1".into()));
    assert_eq!(window.layout, Some("layout".into()));
}

#[test]
fn window_def_raises_error_on_invalid_layout_value() {
    let yaml = r#"
        win1:
            layout: 42
    "#;

    let result = serde_yaml::from_str::<Window>(yaml);
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "expected layout to be a string"
    )
}

#[test]
fn window_def_deserializes_panes() {
    let yaml = r#"
        win1:
          panes:
            - echo cmd1
            - echo cmd2
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(window.name, Some("win1".into()));
    assert_eq!(
        window.panes,
        vec![
            serde_yaml::from_str("echo cmd1").unwrap(),
            serde_yaml::from_str("echo cmd2").unwrap(),
        ]
    );
}

#[test]
fn window_def_deserializes_from_null() {
    let yaml = r#"
        win1:
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(window.name, Some("win1".into()));
    assert_eq!(window.panes, vec![serde_yaml::from_str("~").unwrap()]);
}

#[test]
fn window_def_deserializes_from_str() {
    let yaml = r#"
        win1: echo cmd
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(window.name, Some("win1".into()));
    assert_eq!(
        window.panes,
        vec![serde_yaml::from_str("echo cmd").unwrap()]
    );
}

#[test]
fn window_def_deserializes_from_sequence() {
    let yaml = r#"
        win1:
          - echo cmd1
          - echo cmd2
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(window.name, Some("win1".into()));
    assert_eq!(
        window.panes,
        vec![
            serde_yaml::from_str("echo cmd1").unwrap(),
            serde_yaml::from_str("echo cmd2").unwrap(),
        ]
    );
}

#[test]
fn window_deserializes_from_null() {
    let yaml = r#"
        ~
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(window, Window::default());
    assert_eq!(window.panes, vec![serde_yaml::from_str("~").unwrap()]);
}

#[test]
fn window_deserializes_from_str() {
    let yaml = r#"
        echo cmd
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        window.panes,
        vec![serde_yaml::from_str("echo cmd").unwrap()]
    );
}

#[test]
fn window_deserializes_from_sequence() {
    let yaml = r#"
        - echo cmd1
        - echo cmd2
    "#;

    let window: Window = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        window.panes,
        vec![
            serde_yaml::from_str("echo cmd1").unwrap(),
            serde_yaml::from_str("echo cmd2").unwrap(),
        ]
    );
}

#[test]
fn pane_check_succeeds_on_valid_pane() {
    let pane = Pane::default();

    let result = pane.check();
    assert!(result.is_ok());
}

#[test]
fn pane_deserializes_from_null() {
    let yaml = r#"
        ~
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane, Pane::default());
}

#[test]
fn pane_deserializes_working_dir() {
    let yaml = r#"
        working_dir: /path1
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.working_dir, Some("/path1".into()));

    let yaml = r#"
        root: /path2
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.working_dir, Some("/path2".into()));
}

#[test]
fn pane_deserializes_working_dir_null_as_home() {
    let yaml = r#"
        working_dir: ~
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.working_dir, Some("~".into()));

    let yaml = r#"
        root: ~
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.working_dir, Some("~".into()));
}

#[test]
fn pane_raises_error_on_invalid_working_dir_value() {
    let yaml = r#"
        working_dir: 42
    "#;

    let result = serde_yaml::from_str::<Pane>(yaml);
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "expected working_dir to be a string or null"
    );
}

#[test]
fn pane_deserializes_split() {
    let yaml = r#"
        split: h
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.split, Some(PaneSplit::Horizontal));

    let yaml = r#"
        split: horizontal
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.split, Some(PaneSplit::Horizontal));

    let yaml = r#"
        split: v
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.split, Some(PaneSplit::Vertical));

    let yaml = r#"
        split: vertical
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.split, Some(PaneSplit::Vertical));
}

#[test]
fn pane_raises_error_on_invalid_split_value() {
    let yaml = r#"
        split: o
    "#;

    let result = serde_yaml::from_str::<Pane>(yaml);
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "expected split value to match v|h|vertical|horizontal"
    );
}

#[test]
fn pane_deserializes_split_from() {
    let yaml = r#"
        split_from: 0
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.split_from, Some(0));
}

#[test]
fn pane_raises_error_on_invalid_split_from_value() {
    let yaml = r#"
        split_from:
          - 42
    "#;

    let result = serde_yaml::from_str::<Pane>(yaml);
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "expected split_from to be a positive integer"
    );
}

#[test]
fn pane_deserializes_split_size_string() {
    let yaml = r#"
        split_size: "75%"
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.split_size, Some("75%".into()));
}

#[test]
fn pane_deserializes_split_size_number() {
    let yaml = r#"
        split_size: 42
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.split_size, Some("42".into()));
}

#[test]
fn pane_raises_error_on_invalid_split_size_value() {
    let yaml = r#"
        split_size:
          - 42
    "#;

    let result = serde_yaml::from_str::<Pane>(yaml);
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "expected split_size to be either a positive integer or a string"
    );
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
        command: 0
    "#;

    let result = serde_yaml::from_str::<Pane>(yaml);
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "expected commands to be null, a string or a list of strings"
    );
}

#[test]
fn pane_from_string_translates_to_single_command_pane() {
    let command = "echo hello";

    let pane = Pane::from(command);

    assert_eq!(pane.commands.len(), 1);
    assert_eq!(pane.commands[0], command);
}
