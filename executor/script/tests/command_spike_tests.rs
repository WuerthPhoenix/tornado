use std::process::Command;

#[test]
fn spike_command_script() {
    let output = Command::new("./test_resources/echo.sh")
        .arg("hello_world")
        .output()
        .expect("failed to execute process");

    println!("status: {}", output.status);
    println!("stdout: {}", String::from_utf8_lossy(&output.stdout).trim());
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr).trim());

    assert_eq!("hello_world", String::from_utf8_lossy(&output.stdout).trim());
    assert!(output.status.success());
}

#[test]
fn spike_command_failing_script() {
    let output =
        Command::new("./test_resources/fail.sh").output().expect("failed to execute process");

    println!("status: {}", output.status);

    assert!(!output.status.success());
}

#[test]
fn spike_command_script_with_inline_args() {
    let shell: [&str; 2] = ["sh", "-c"];
    let output = Command::new(shell[0])
        .args(&shell[1..])
        .arg("./test_resources/echo.sh hello_world")
        .output()
        .expect("failed to execute process");

    println!("status: {}", output.status);
    println!("stdout: {}", String::from_utf8_lossy(&output.stdout).trim());
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr).trim());

    assert_eq!("hello_world", String::from_utf8_lossy(&output.stdout).trim());
    assert!(output.status.success());
}

#[test]
fn spike_execute_script_write_file() {
    // Arrange
    let tempdir = tempfile::tempdir().unwrap();
    let filename = format!("{}/output.txt", tempdir.path().to_str().unwrap().to_owned());
    let content = "HelloRustyWorld!";

    // Act
    let output = Command::new("./test_resources/write_file.sh")
        .arg(&filename)
        .arg(&content)
        .output()
        .expect("failed to execute process");

    // Assert
    assert!(output.status.success());

    let file_content = std::fs::read_to_string(&filename).unwrap();
    assert_eq!(content, file_content.trim())
}
