use super::*;

#[test]
fn process_command_escapes_pounds() {
    let result = process_command(String::from("#hello #world##"));

    assert_eq!(result, "##hello ##world####")
}

#[test]
fn process_command_removes_line_carriages() {
    let result = process_command(String::from("hello\r\n\nworld\n\r\n"));

    assert_eq!(result, "hello  world  ")
}
