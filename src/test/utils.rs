use super::*;

#[test]
fn parses_empty_namespace() {
    let expected_result = PathBuf::new();
    let project_name = "project";

    let result = get_project_namespace(&project_name).unwrap();
    assert_eq!(result, expected_result);
}

#[test]
fn parses_namespace() {
    let expected_result = PathBuf::from("namespace");
    let project_name = "namespace/project";

    let result = get_project_namespace(&project_name).unwrap();
    assert_eq!(result, expected_result);
}

#[test]
fn parses_multilevel_namspaces() {
    let expected_result = PathBuf::from(format!(
        "my{}name{}space",
        path::MAIN_SEPARATOR,
        path::MAIN_SEPARATOR,
    ));

    let project_name = format!(
        "my{}name{}space{}project",
        path::MAIN_SEPARATOR,
        path::MAIN_SEPARATOR,
        path::MAIN_SEPARATOR,
    );

    let result = get_project_namespace(&project_name).unwrap();
    assert_eq!(result, expected_result);
}

#[test]
fn fails_when_project_name_has_a_trailing_slash() {
    let name = format!("project{}", path::MAIN_SEPARATOR);

    let result = get_project_namespace(&name);
    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap().downcast_ref::<Error>().unwrap(),
        Error::ProjectNameTrailingSlash { project_name } if *project_name == name
    ));
}

#[test]
#[cfg(unix)]
fn fails_when_project_name_is_an_absolute_path() {
    let name = "/project";

    let result = get_project_namespace(&name);
    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap().downcast_ref::<Error>().unwrap(),
        Error::ProjectNameAbsolutePath { project_name } if *project_name == name
    ));
}

#[test]
#[cfg(windows)]
fn fails_when_project_name_is_an_absolute_path_windows() {
    let name = "c:/project";

    let result = get_project_namespace(&name);
    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap().downcast_ref::<Error>().unwrap(),
        Error::ProjectNameAbsolutePath { project_name } if *project_name == name
    ));
}

#[test]
fn correct_command_parses_single_command() {
    let expected_result = (String::from("cmd"), vec![]);
    let command = "cmd";
    let args = &[];

    let result = parse_command(command, args).unwrap();
    assert_eq!(result, expected_result);
}

#[test]
fn correct_command_parses_command_with_flags_command() {
    let expected_result = (String::from("cmd"), vec![String::from("-flag")]);
    let command = "cmd -flag";
    let args = &[];

    let result = parse_command(command, args).unwrap();
    assert_eq!(result, expected_result);
}

#[test]
fn correct_command_parses_returns_correct_arguments() {
    let expected_result = (
        String::from("cmd"),
        vec![String::from("-flag"), String::from("file")],
    );
    let command = "cmd -flag";
    let args = &["file"];

    let result = parse_command(command, args).unwrap();
    assert_eq!(result, expected_result);
}

#[test]
fn correct_command_fails_on_empty_command() {
    let command = "";
    let args = &[];

    let result = parse_command(command, args);
    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap().downcast_ref::<Error>().unwrap(),
        Error::EmptyCommand {}
    ));
}
