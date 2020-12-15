use super::*;

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
