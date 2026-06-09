use std::process::{Command, Stdio};

/// Build a `busca` command with stdin detached, so the binary deterministically
/// sees a non-TTY stdin (no interactive picker) regardless of the test runner.
fn busca() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_busca"));
    command.stdin(Stdio::null());
    command
}

#[test]
fn json_format_emits_array_without_content_by_default() {
    let output = busca()
        .args([
            "-r",
            "sample_dir_hello_world/file_1.py",
            "-s",
            "sample_dir_hello_world",
            "--include-glob",
            "*.py",
            "--format",
            "json",
        ])
        .output()
        .expect("run busca");
    assert!(output.status.success(), "status: {:?}", output.status);

    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("valid json array");
    let array = value.as_array().expect("top-level array");
    assert!(!array.is_empty(), "expected at least one comparison");

    let first = &array[0];
    assert!(first.get("path").is_some(), "path field present");
    assert!(
        first.get("similarity_ratio").is_some(),
        "similarity_ratio field present"
    );
    assert!(
        first.get("content").is_none(),
        "content omitted without --with-content"
    );
}

#[test]
fn json_format_includes_content_with_flag() {
    let output = busca()
        .args([
            "-r",
            "sample_dir_hello_world/file_1.py",
            "-s",
            "sample_dir_hello_world",
            "--include-glob",
            "*.py",
            "--format",
            "json",
            "--with-content",
        ])
        .output()
        .expect("run busca");
    assert!(output.status.success(), "status: {:?}", output.status);

    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("valid json array");
    let first = &value.as_array().expect("array")[0];
    assert!(
        first.get("content").is_some(),
        "content present with --with-content"
    );
}

#[test]
fn json_count_limits_array_length() {
    let output = busca()
        .args([
            "-r",
            "sample_dir_hello_world/file_1.py",
            "-s",
            "sample_dir_hello_world",
            "--include-glob",
            "*.py",
            "--format",
            "json",
            "--count",
            "1",
        ])
        .output()
        .expect("run busca");
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("valid json array");
    assert_eq!(value.as_array().expect("array").len(), 1);
}

#[test]
fn no_interactive_prints_table_without_picker_note() {
    let output = busca()
        .args([
            "-r",
            "sample_dir_hello_world/file_1.py",
            "-s",
            "sample_dir_hello_world",
            "--include-glob",
            "*.py",
            "--no-interactive",
        ])
        .output()
        .expect("run busca");
    assert!(output.status.success(), "status: {:?}", output.status);

    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("utf-8 stderr");
    assert!(stdout.contains("file_1.py"), "table on stdout");
    // The user asked for non-interactive, so the explanatory note is suppressed.
    assert!(
        !stderr.contains("interactive prompt is not supported"),
        "no fallback note when --no-interactive is explicit"
    );
}

#[test]
fn auto_fallback_note_goes_to_stderr_not_stdout() {
    let output = busca()
        .args([
            "-r",
            "sample_dir_hello_world/file_1.py",
            "-s",
            "sample_dir_hello_world",
            "--include-glob",
            "*.py",
        ])
        .output()
        .expect("run busca");
    assert!(output.status.success(), "status: {:?}", output.status);

    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("utf-8 stderr");
    assert!(
        !stdout.contains("interactive prompt is not supported"),
        "note must not pollute stdout"
    );
    assert!(
        stderr.contains("interactive prompt is not supported"),
        "auto fallback explains itself on stderr"
    );
}

#[test]
fn exit_zero_when_results_found() {
    let status = busca()
        .args([
            "-r",
            "sample_dir_hello_world/file_1.py",
            "-s",
            "sample_dir_hello_world",
            "--include-glob",
            "*.py",
            "--format",
            "json",
        ])
        .status()
        .expect("run busca");
    assert_eq!(status.code(), Some(0));
}

#[test]
fn exit_one_when_no_results() {
    // No candidate matches this glob, so the result set is empty.
    let output = busca()
        .args([
            "-r",
            "sample_dir_hello_world/file_1.py",
            "-s",
            "sample_dir_hello_world",
            "--include-glob",
            "*.no_such_ext",
        ])
        .output()
        .expect("run busca");
    assert_eq!(output.status.code(), Some(1));
    assert!(
        output.stdout.is_empty(),
        "stdout must be clean on empty: {:?}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8(output.stderr).expect("utf-8 stderr");
    assert!(stderr.contains("No files found"), "stderr: {stderr}");
}

#[test]
fn exit_two_on_error() {
    // A missing search path is an error, distinct from an empty result.
    let status = busca()
        .args([
            "-r",
            "sample_dir_hello_world/file_1.py",
            "-s",
            "this_path_does_not_exist_xyz",
            "--format",
            "json",
        ])
        .status()
        .expect("run busca");
    assert_eq!(status.code(), Some(2));
}
