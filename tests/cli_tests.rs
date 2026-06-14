use std::fs;
use std::process::Command;

fn run(args: &[&str]) -> (String, String, i32) {
    let mut cmd = Command::new("cargo");
    cmd.arg("run");
    cmd.arg("--");
    for a in args {
        cmd.arg(*a);
    }
    cmd.current_dir(env!("CARGO_MANIFEST_DIR"));
    let output = cmd.output().expect("failed to run diagram");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);
    (stdout, stderr, code)
}

fn temp_mmd(contents: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let mut path = std::env::temp_dir();
    path.push(format!("diagram_test_{}_{}.mmd", std::process::id(), n));
    fs::write(&path, contents).unwrap();
    path
}

#[test]
fn test_cli_validate_clean() {
    let tmp = temp_mmd("graph TD\n    A[Start] --> B[End]\n");
    let path = tmp.to_str().unwrap();
    let (stdout, _, code) = run(&["validate", path]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Valid: no issues found"), "stdout: {stdout}");
}

#[test]
fn test_cli_validate_cycle() {
    let tmp = temp_mmd("graph TD\n    A --> B\n    B --> A\n");
    let path = tmp.to_str().unwrap();
    let (stdout, _, code) = run(&["validate", path]);
    assert_eq!(code, 0);
    assert!(stdout.contains("cycle detected"), "stdout: {stdout}");
}

#[test]
fn test_cli_get_node_found() {
    let tmp = temp_mmd("graph TD\n    A[Hello] --> B[World]\n");
    let path = tmp.to_str().unwrap();
    let (stdout, _, code) = run(&["get-node", path, "A"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("A [rect] Hello"), "stdout: {stdout}");
}

#[test]
fn test_cli_get_node_not_found() {
    let tmp = temp_mmd("graph TD\n    A[Hello] --> B[World]\n");
    let path = tmp.to_str().unwrap();
    let (stdout, _, code) = run(&["get-node", path, "Z"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("not found"), "stdout: {stdout}");
}

#[test]
fn test_cli_get_edge_found() {
    let tmp = temp_mmd("graph TD\n    A[Hello] --> B[World]\n");
    let path = tmp.to_str().unwrap();
    let (stdout, _, code) = run(&["get-edge", path, "A", "B"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("A --> B"), "stdout: {stdout}");
}

#[test]
fn test_cli_update_edge_changes_style() {
    let tmp = temp_mmd("graph TD\n    A[Hello] --> B[World]\n");
    let path = tmp.to_str().unwrap();
    let (_, _, code) = run(&["update-edge", path, "A", "B", "--style", "dashed"]);
    assert_eq!(code, 0);
    let (stdout, _, code) = run(&["get-edge", path, "A", "B"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("-.->"), "expected dashed edge, got: {stdout}");
}

#[test]
fn test_cli_add_node_invalid_shape() {
    let tmp = temp_mmd("graph TD\n    A[Hello] --> B[World]\n");
    let path = tmp.to_str().unwrap();
    let (_, stderr, code) = run(&["add-node", path, "X", "Test", "--shape", "triangle"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("Invalid shape") || stderr.contains("error"), "stderr: {stderr}");
}

#[test]
fn test_cli_add_edge_invalid_style() {
    let tmp = temp_mmd("graph TD\n    A[Hello] --> B[World]\n");
    let path = tmp.to_str().unwrap();
    let (_, stderr, code) = run(&["add-edge", path, "A", "B", "--style", "dotted"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("Invalid edge style") || stderr.contains("error"), "stderr: {stderr}");
}

#[test]
fn test_cli_info_counts() {
    let tmp = temp_mmd("graph TD\n    A[Start] --> B{Is it?}\n    B --> C[End]\n");
    let path = tmp.to_str().unwrap();
    let (stdout, _, code) = run(&["info", path]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Nodes: 3"), "stdout: {stdout}");
    assert!(stdout.contains("Edges: 2"), "stdout: {stdout}");
    assert!(stdout.contains("diamond:  1"), "stdout: {stdout}");
}

#[test]
fn test_cli_list_nodes() {
    let tmp = temp_mmd("graph TD\n    A[Start] --> B[End]\n");
    let path = tmp.to_str().unwrap();
    let (stdout, _, code) = run(&["list-nodes", path]);
    assert_eq!(code, 0);
    assert!(stdout.contains("A [rect] Start"), "stdout: {stdout}");
    assert!(stdout.contains("B [rect] End"), "stdout: {stdout}");
}

#[test]
fn test_cli_list_edges() {
    let tmp = temp_mmd("graph TD\n    A[Start] -->|yes| B[End]\n");
    let path = tmp.to_str().unwrap();
    let (stdout, _, code) = run(&["list-edges", path]);
    assert_eq!(code, 0);
    assert!(stdout.contains("A --> B |yes|"), "stdout: {stdout}");
}

#[test]
fn test_cli_render_watch_help() {
    let (stdout, _, code) = run(&["render", "--help"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("--watch"), "render --help should mention --watch flag\nstdout: {stdout}");
}
