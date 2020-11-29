use super::*;

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
        on_create: echo on_create
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
            on_create: vec![String::from("echo on_create")],
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
        on_create:
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
    println!("{:?}", result);
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
fn project_template_coerces_from_str() {
    let template = "template_content";

    let project_template = ProjectTemplate::from(template);
    assert_eq!(
        project_template,
        ProjectTemplate::Raw(String::from(template))
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
    assert_eq!(
        project_template,
        ProjectTemplate::Raw(String::from("my_template"))
    );
}

#[test]
fn project_template_deserializes_from_file_mapping() {
    let yaml = r#"
        file: template.tera
    "#;

    let project_template: ProjectTemplate = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(
        project_template,
        ProjectTemplate::File(PathBuf::from("template.tera"))
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
        .contains("data did not match any variant of untagged enum TemplateProxy"));

    let yaml = r#"
        not_file: test.file
    "#;

    let result = serde_yaml::from_str::<ProjectTemplate>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("data did not match any variant of untagged enum TemplateProxy"));

    let yaml = r#"
        file: 42
    "#;

    let result = serde_yaml::from_str::<ProjectTemplate>(yaml);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("data did not match any variant of untagged enum TemplateProxy"));
}

#[test]
fn window_check_succeeds_on_valid_window() {
    let window = Window {
        name: Some(String::from("window")),
        ..Window::default()
    };

    let result = window.check();
    assert!(result.is_ok());
}

#[test]
fn window_check_fails_on_invalid_name() {
    let window = Window {
        name: Some(String::from("window:1")),
        ..Window::default()
    };

    let result = window.check();
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "name \"window:1\" cannot contain the following characters: .: "
    );

    let window = Window {
        name: Some(String::from("window.1")),
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
fn window_check_fails_when_pane_split_from_is_out_of_bounds() {
    let window = Window {
        panes: vec![Pane {
            split_from: Some(2),
            ..Pane::default()
        }],
        ..Window::default()
    };
    assert_eq!(window.panes.len(), 1);

    let result = window.check();
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap().to_string(),
        "split_from: there is no pane with index 2 (pane indexes always start at 0)"
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
    let result = window.check();
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
    let result = window.check();
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
    let result = window.check();
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
        panes: echo pane
    "#;

    let result = serde_yaml::from_str::<Window>(yaml);
    println!("{:?}", result);
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
        .contains("data did not match any variant of untagged enum WindowOption"));
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
    println!("{:?}", result);
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
    println!("{:?}", result);
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
fn pane_deserializes_split_h() {
    let yaml = r#"
        split: h
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.split, Some(PaneSplit::Horizontal));
}

#[test]
fn pane_deserializes_split_horizontal() {
    let yaml = r#"
        split: horizontal
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.split, Some(PaneSplit::Horizontal));
}

#[test]
fn pane_deserializes_split_v() {
    let yaml = r#"
        split: v
    "#;

    let pane: Pane = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(pane.split, Some(PaneSplit::Vertical));
}

#[test]
fn pane_deserializes_split_vertical() {
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
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("expected split value \"o\" to match v|h|vertical|horizontal"));
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
