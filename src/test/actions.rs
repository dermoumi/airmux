use super::*;
use std::os;
use std::path;
use std::path::PathBuf;
use tempfile::tempdir;

#[cfg(windows)]
const TEST_EDITOR_BIN: &str = "cmd /c echo";
#[cfg(unix)]
const TEST_EDITOR_BIN: &str = "true";

fn make_config(tmux_command: Option<&str>, config_dir: Option<PathBuf>) -> Config {
    Config {
        app_name: "test_app_name",
        app_author: "test_app_author",
        tmux_command: Some(String::from(tmux_command.unwrap_or("tmux"))),
        config_dir,
    }
}

#[test]
fn edit_project_fails_when_editor_is_empty() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();
    let test_config = make_config(None, Some(temp_dir));
    let project_name = "project";

    assert!(matches!(
        edit_project(
            &test_config,
            Some(project_name),
            None,
            Some("yml"),
            "",
            false,
            &[]
        )
        .err()
        .unwrap()
        .downcast_ref::<Error>()
        .unwrap(),
        Error::EditorEmpty {}
    ))
}

#[test]
fn edit_project_succeeds_when_project_file_does_not_exist() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();
    let test_config = make_config(None, Some(temp_dir));
    let project_name = "project";
    let project_path = test_config
        .get_projects_dir(&project_name)
        .unwrap()
        .with_extension("yml");

    let result = edit_project(
        &test_config,
        Some(project_name),
        None,
        Some("yml"),
        TEST_EDITOR_BIN,
        true,
        &[],
    );

    assert!(project_path.is_file());
    assert!(result.is_ok());
}

#[test]
fn edit_project_succeeds_when_project_file_exists() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();
    let test_config = make_config(None, Some(temp_dir));
    let project_name = "project";

    // Make sure the file exists
    let projects_dir = test_config.get_projects_dir("").unwrap();
    let project_path = projects_dir.join(&project_name).with_extension("yml");
    mkdirp(projects_dir).unwrap();
    edit::create_project(&project_name, &project_path, "yml", None).unwrap();
    assert!(project_path.is_file());

    // Run edit_project
    let result = edit_project(
        &test_config,
        Some(project_name),
        None,
        Some("yml"),
        TEST_EDITOR_BIN,
        true,
        &[],
    );

    assert!(project_path.is_file());
    assert!(result.is_ok());
}

#[test]
fn edit_project_creates_sub_directories_as_needed() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();
    let test_config = make_config(None, Some(temp_dir));
    let project_name = "subdir1/subdir2/project";
    let subdir_path = test_config.get_projects_dir("subdir1/subdir2").unwrap();
    let project_path = test_config
        .get_projects_dir(&project_name)
        .unwrap()
        .with_extension("yml");

    edit_project(
        &test_config,
        Some(project_name),
        None,
        Some("yml"),
        TEST_EDITOR_BIN,
        true,
        &[],
    )
    .unwrap();

    assert!(subdir_path.is_dir());
    assert!(project_path.is_file());
}

#[test]
fn edit_project_fails_when_project_path_is_directory() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();
    let test_config = make_config(None, Some(temp_dir));
    let project_name = "project";
    let project_path = test_config
        .get_projects_dir(&project_name)
        .unwrap()
        .with_extension("yml");

    mkdirp(&project_path).unwrap();
    assert!(&project_path.is_dir());

    let result = edit_project(
        &test_config,
        Some(project_name),
        None,
        Some("yml"),
        TEST_EDITOR_BIN,
        false,
        &[],
    );
    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap().downcast_ref::<Error>().unwrap(),
        Error::ProjectFileIsADirectory { path } if path == &project_path
    ));
}

#[test]
fn edit_project_project_name_cannot_be_empty() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();
    let test_config = make_config(None, Some(temp_dir));
    let project_name = "";

    let result = edit_project(
        &test_config,
        Some(project_name),
        None,
        Some("yml"),
        TEST_EDITOR_BIN,
        false,
        &[],
    );
    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap().downcast_ref::<Error>().unwrap(),
        Error::ProjectNameEmpty {}
    ));
}

#[test]
fn edit_project_fails_if_extension_is_not_supported() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();
    let test_config = make_config(None, Some(temp_dir));
    let project_name = "project";
    let unsupported_extension = "exe";

    let result = edit_project(
        &test_config,
        Some(project_name),
        None,
        Some(unsupported_extension),
        TEST_EDITOR_BIN,
        false,
        &[],
    );
    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap().downcast_ref::<Error>().unwrap(),
        Error::UnsupportedFileExtension { extension } if extension == unsupported_extension
    ));
}

#[test]
fn edit_project_creates_file_locally() {
    let temp_config_dir = tempdir().unwrap();
    let temp_config_dir = temp_config_dir.path().to_path_buf();
    let test_config = make_config(None, Some(temp_config_dir));

    let temp_local_dir = tempdir().unwrap();
    let temp_local_dir = temp_local_dir.path();
    std::env::set_current_dir(temp_local_dir).unwrap();

    let extension = "json";
    let project_file = temp_local_dir.join(".airmux").with_extension(extension);
    assert!(!project_file.exists());

    edit_project(
        &test_config,
        None,
        None,
        Some(extension),
        TEST_EDITOR_BIN,
        true,
        &[],
    )
    .unwrap();
    assert!(project_file.exists());
}

#[test]
fn remove_project_removes_existing_project() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();
    let test_config = make_config(None, Some(temp_dir));
    let project_name = "project";

    // Make sure the file exists
    let projects_dir = test_config.get_projects_dir("").unwrap();
    let project_path = projects_dir.join(&project_name).with_extension("yml");
    mkdirp(projects_dir).unwrap();
    edit::create_project(&project_name, &project_path, "yml", None).unwrap();
    assert!(project_path.is_file());

    let result = remove_project(&test_config, Some(project_name), true);
    assert!(result.is_ok());
    assert!(!project_path.exists());
}

#[test]
fn remove_project_removes_parent_subdirectories_if_empty() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();
    let test_config = make_config(None, Some(temp_dir));
    let project_name = "subdir1/subdir2/project";

    // Make sure the project's parent directory exists
    let namespace = utils::get_project_namespace(&project_name).unwrap();
    let data_dir = test_config.get_projects_dir("").unwrap();
    mkdirp(data_dir.join(&namespace)).unwrap();

    // Make sure the file exists
    let projects_dir = test_config.get_projects_dir("").unwrap();
    let project_path = projects_dir.join(&project_name).with_extension("yml");
    edit::create_project(&project_name, &project_path, "yml", None).unwrap();
    assert!(project_path.is_file());

    let result = remove_project(&test_config, Some(project_name), true);
    assert!(result.is_ok());
    assert!(!project_path.exists());
    assert!(!project_path.parent().unwrap().exists());
    assert!(!project_path.parent().unwrap().parent().unwrap().exists());
}

#[test]
fn remove_project_does_not_remove_parent_subdirs_if_not_empty() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();
    let test_config = make_config(None, Some(temp_dir));
    let project1_name = "subdir1/subdir2/project1";
    let project2_name = "subdir1/project2";

    // Make sure the project's parent directory exists
    let namespace = utils::get_project_namespace(&project1_name).unwrap();
    let data_dir = test_config.get_projects_dir("").unwrap();
    mkdirp(data_dir.join(&namespace)).unwrap();

    // Make sure the file exists
    let projects_dir = test_config.get_projects_dir("").unwrap();

    let project1_path = projects_dir.join(&project1_name).with_extension("yml");
    edit::create_project(&project1_name, &project1_path, "yml", None).unwrap();
    assert!(project1_path.is_file());

    let project2_path = projects_dir.join(&project2_name).with_extension("yml");
    edit::create_project(&project2_name, &project2_path, "yml", None).unwrap();
    assert!(project2_path.is_file());

    let result = remove_project(&test_config, Some(project1_name), true);
    assert!(result.is_ok());
    assert!(!project1_path.exists());
    assert!(!project1_path.parent().unwrap().exists());
    assert!(project1_path.parent().unwrap().parent().unwrap().exists());
}

#[test]
fn remove_project_fails_if_project_does_not_exist() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();
    let test_config = make_config(None, Some(temp_dir));
    let project1_name = "project";

    let result = remove_project(&test_config, Some(project1_name), true);
    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap().downcast_ref::<Error>().unwrap(),
        Error::ProjectDoesNotExist { project_name } if project_name == project1_name
    ));
}

#[test]
fn remove_project_project_name_cannot_be_empty() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();
    let test_config = make_config(None, Some(temp_dir));
    let project_name = "";

    let result = remove_project(&test_config, Some(project_name), true);
    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap().downcast_ref::<Error>().unwrap(),
        Error::ProjectNameEmpty {}
    ));
}

#[test]
fn remove_project_removes_local_project() {
    let temp_config_dir = tempdir().unwrap();
    let temp_config_dir = temp_config_dir.path().to_path_buf();
    let test_config = make_config(None, Some(temp_config_dir));

    let temp_local_dir = tempdir().unwrap();
    let temp_local_dir = temp_local_dir.path();
    std::env::set_current_dir(temp_local_dir).unwrap();

    let project_file = temp_local_dir.join(".airmux");

    for extension in FILE_EXTENSIONS {
        let project_file = project_file.with_extension(extension);

        let file = fs::File::create(&project_file).unwrap();
        file.sync_all().unwrap();
        assert!(project_file.exists());

        remove_project(&test_config, None, true).unwrap();
        assert!(!project_file.exists());
    }
}

#[test]
fn list_project_does_not_fail() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();
    let test_config = make_config(None, Some(temp_dir));
    let projects_dir = test_config.get_projects_dir("").unwrap();

    for n in 0..5 {
        let project_name = format!("project{}", n);

        edit::create_project(&project_name, projects_dir.join(&project_name), "yml", None).unwrap();
    }

    list_projects(&test_config).unwrap();
}

#[test]
fn get_project_list_returns_projects_without_extensions() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();

    let mut expected_project_list = Vec::with_capacity(5);
    for n in 0..5 {
        let project_name = format!("project{}", n);

        let project_path = temp_dir.join(&project_name);
        let project_path = project::test_for_file_extensions(project_path).unwrap();

        edit::create_project(&project_name, &project_path, "yml", None).unwrap();
        expected_project_list.push(project_name);
    }
    expected_project_list.sort();

    let mut project_list = list::get_projects(&temp_dir).unwrap();
    project_list.sort();

    assert_eq!(project_list, expected_project_list);
}

#[test]
fn list_shows_projects_in_subdirectories() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();

    let mut expected_project_list = Vec::with_capacity(4);
    for n in 0..2 {
        let project_name = format!("project{}", n);

        let project_path = temp_dir.join(&project_name);
        let project_path = project::test_for_file_extensions(project_path).unwrap();

        edit::create_project(&project_name, &project_path, "yml", None).unwrap();
        expected_project_list.push(project_name);
    }
    for n in 2..4 {
        let project_name = format!("subdir1{}project{}", path::MAIN_SEPARATOR, n);
        mkdirp(temp_dir.join("subdir1")).unwrap();

        let project_path = temp_dir.join(&project_name);
        let project_path = project::test_for_file_extensions(project_path).unwrap();

        edit::create_project(&project_name, &project_path, "yml", None).unwrap();
        expected_project_list.push(project_name);
    }
    for n in 4..6 {
        let project_name = format!("subdir2{}project{}", path::MAIN_SEPARATOR, n);
        mkdirp(temp_dir.join("subdir2")).unwrap();

        let project_path = temp_dir.join(&project_name);
        let project_path = project::test_for_file_extensions(project_path).unwrap();

        edit::create_project(&project_name, &project_path, "yml", None).unwrap();
        expected_project_list.push(project_name);
    }
    expected_project_list.sort();

    let mut project_list = list::get_projects(&temp_dir).unwrap();
    project_list.sort();

    assert_eq!(project_list, expected_project_list);
}

#[test]
fn list_follows_symlinks() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();

    let mut expected_project_list = Vec::with_capacity(4);
    for n in 0..2 {
        let project_name = format!("project{}", n);

        let project_path = temp_dir.join(&project_name);
        let project_path = project::test_for_file_extensions(project_path).unwrap();

        edit::create_project(&project_name, &project_path, "yml", None).unwrap();
        expected_project_list.push(project_name);
    }
    for n in 2..4 {
        let project_name = format!("subdir1{}project{}", path::MAIN_SEPARATOR, n);
        mkdirp(temp_dir.join("subdir1")).unwrap();

        let project_path = temp_dir.join(&project_name);
        let project_path = project::test_for_file_extensions(project_path).unwrap();

        edit::create_project(&project_name, &project_path, "yml", None).unwrap();
        expected_project_list.push(project_name);
    }
    for n in 2..4 {
        let project_name = format!("subdir2{}project{}", path::MAIN_SEPARATOR, n);

        expected_project_list.push(project_name);
    }
    expected_project_list.sort();

    #[cfg(windows)]
    os::windows::fs::symlink_dir(temp_dir.join("subdir1"), temp_dir.join("subdir2")).unwrap();
    #[cfg(unix)]
    os::unix::fs::symlink(temp_dir.join("subdir1"), temp_dir.join("subdir2")).unwrap();
    assert!(temp_dir.join("subdir2").is_dir());

    let mut project_list = list::get_projects(&temp_dir).unwrap();
    project_list.sort();

    assert_eq!(project_list, expected_project_list);
}

#[test]
fn list_detects_symlink_loops() {
    let temp_dir = tempdir().unwrap();
    let temp_dir = temp_dir.path().to_path_buf();

    let mut expected_project_list = Vec::with_capacity(4);
    for n in 0..2 {
        let project_name = format!("project{}", n);

        let project_path = temp_dir.join(&project_name);
        let project_path = project::test_for_file_extensions(project_path).unwrap();

        edit::create_project(&project_name, &project_path, "yml", None).unwrap();
        expected_project_list.push(project_name);
    }
    for n in 2..4 {
        let project_name = format!("subdir1{}project{}", path::MAIN_SEPARATOR, n);
        mkdirp(temp_dir.join("subdir1")).unwrap();

        let project_path = temp_dir.join(&project_name);
        let project_path = project::test_for_file_extensions(project_path).unwrap();

        edit::create_project(&project_name, &project_path, "yml", None).unwrap();
        expected_project_list.push(project_name);
    }
    expected_project_list.sort();

    #[cfg(windows)]
    os::windows::fs::symlink_dir(&temp_dir, temp_dir.join("subdir2")).unwrap();
    #[cfg(unix)]
    os::unix::fs::symlink(&temp_dir, temp_dir.join("subdir2")).unwrap();
    assert!(temp_dir.join("subdir2").is_dir());

    let mut project_list = list::get_projects(&temp_dir).unwrap();
    project_list.sort();

    assert_eq!(project_list, expected_project_list);
}

#[test]
fn env_context_returns_positional_vars_if_in_bounds() {
    let result = project::env_context("2", &["var1", "var2", "var3"]).unwrap();

    assert_eq!(result, Some(String::from("var2")));
}

#[test]
fn env_context_returns_none_if_out_of_bounds() {
    let result = project::env_context("0", &["var1", "var2", "var3"]).unwrap();

    assert_eq!(result, None);
}

#[test]
fn get_filename_extracts_project_name_from_project_file() {
    let test_config = make_config(None, None);
    let test_project_file = "/some/path/myfile.yml";

    let (project_name, project_path) =
        project::get_filename(&test_config, None, Some(test_project_file)).unwrap();

    assert_eq!(project_name, "myfile.yml");
    assert_eq!(project_path, PathBuf::from("/some/path/myfile.yml"));
}

#[test]
fn get_filename_returns_empty_filepath_if_project_file_is_single_dash() {
    let test_config = make_config(None, None);
    let test_project_name = "my_project";
    let test_project_file = "-";

    let (project_name, project_path) = project::get_filename(
        &test_config,
        Some(test_project_name),
        Some(test_project_file),
    )
    .unwrap();

    assert_eq!(project_name, "my_project");
    assert_eq!(project_path, PathBuf::new());
}

#[test]
fn get_filename_fails_if_project_file_is_single_dash_and_project_name_is_none() {
    let test_config = make_config(None, None);
    let test_project_file = "-";

    let result = project::get_filename(&test_config, None, Some(test_project_file));

    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap().downcast_ref::<Error>().unwrap(),
        Error::ProjectNameEmpty
    ));
}

#[test]
fn get_filename_fails_if_project_file_is_single_dash_and_project_name_is_empty() {
    let test_config = make_config(None, None);
    let test_project_name = "";
    let test_project_file = "-";

    let result = project::get_filename(
        &test_config,
        Some(test_project_name),
        Some(test_project_file),
    );

    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap().downcast_ref::<Error>().unwrap(),
        Error::ProjectNameEmpty
    ));
}

#[test]
fn get_filename_fails_if_path_does_not_contain_a_filename() {
    let test_config = make_config(None, None);
    let test_project_file = "";

    let result = project::get_filename(&test_config, None, Some(test_project_file));

    assert!(result.is_err());
    assert!(matches!(
        result.err().unwrap().downcast_ref::<Error>().unwrap(),
        Error::CannotExtractProjectName { project_file } if project_file == &PathBuf::from(test_project_file)
    ));
}
