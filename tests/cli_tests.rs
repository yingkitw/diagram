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

#[test]
fn test_cli_diff() {
    let left = temp_mmd("graph TD\n    A[Start] --> B[End]\n");
    let right = temp_mmd("graph TD\n    A[Start] --> B[Done]\n    A --> C[New]\n");
    let (stdout, _, code) = run(&["diff", left.to_str().unwrap(), right.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("added_nodes"), "diff output should contain added_nodes\nstdout: {stdout}");
    assert!(stdout.contains("C"), "diff output should mention added node C\nstdout: {stdout}");
}

#[test]
fn test_cli_merge() {
    let left = temp_mmd("graph TD\n    A[Start] --> B[End]\n");
    let right = temp_mmd("graph TD\n    A[Start] --> C[New]\n");
    let output = std::env::temp_dir().join("merged_test.mmd");
    let (stdout, _, code) = run(&[
        "merge",
        left.to_str().unwrap(),
        right.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
    ]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Merged"), "merge should report success\nstdout: {stdout}");
    let merged = std::fs::read_to_string(&output).unwrap();
    assert!(merged.contains("C[New]"), "merged output should contain new node C\nmerged: {merged}");
    let _ = std::fs::remove_file(&output);
}

#[test]
fn test_cli_preview_help() {
    let (stdout, _, code) = run(&["preview", "--help"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("--port"), "preview --help should mention --port\nstdout: {stdout}");
    assert!(stdout.contains("--theme"), "preview --help should mention --theme\nstdout: {stdout}");
}

#[test]
fn test_cli_multi_document_render() {
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/multi-document.json");
    let dir = std::env::temp_dir().join(format!("diagram_multi_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (stdout, _, code) = run(&[
        "render",
        src.to_str().unwrap(),
        "--output-dir",
        dir.to_str().unwrap(),
    ]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Rendered 2 diagram(s)"), "stdout: {stdout}");
    assert!(dir.join("multi-document-0.svg").exists());
    assert!(dir.join("multi-document-1.svg").exists());
    let (stdout, _, code) = run(&["info", src.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Diagrams: 2"), "stdout: {stdout}");
    assert!(stdout.contains("Diagram 0: flowchart"), "stdout: {stdout}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_cli_markdown_pipeline() {
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/doc-with-diagrams.md");
    let dir = std::env::temp_dir().join(format!("diagram_md_cli_{}", std::process::id()));
    let img_dir = dir.join("assets");
    let out_md = dir.join("rendered.md");
    std::fs::create_dir_all(&img_dir).unwrap();
    let (_, _, code) = run(&[
        "markdown",
        src.to_str().unwrap(),
        "--output-dir",
        img_dir.to_str().unwrap(),
        "--output",
        out_md.to_str().unwrap(),
        "--format",
        "png",
    ]);
    assert_eq!(code, 0);
    let body = std::fs::read_to_string(&out_md).unwrap();
    assert!(body.contains("![diagram 0]"));
    assert!(!body.contains("```mermaid"));
    assert!(img_dir.join("doc-with-diagrams-0.png").exists());
    assert!(img_dir.join("doc-with-diagrams-1.png").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_cli_render_pdf() {
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/simple-flowchart.mmd");
    let out = std::env::temp_dir().join(format!("diagram_render_{}.pdf", std::process::id()));
    let (_, _, code) = run(&[
        "render",
        src.to_str().unwrap(),
        "--output",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0);
    let bytes = std::fs::read(&out).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
    let _ = std::fs::remove_file(&out);
}

#[test]
fn test_cli_render_png() {
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/simple-flowchart.mmd");
    let out = std::env::temp_dir().join(format!("diagram_render_{}.png", std::process::id()));
    let (_, _, code) = run(&[
        "render",
        src.to_str().unwrap(),
        "--output",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0);
    let bytes = std::fs::read(&out).unwrap();
    assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    let _ = std::fs::remove_file(&out);
}

#[test]
fn test_cli_sequence_info_and_render() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/sequence.mmd");
    let path = path.to_str().unwrap();
    let (stdout, _, code) = run(&["info", path]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Kind: sequence"), "stdout: {stdout}");
    assert!(stdout.contains("Participants: 2"), "stdout: {stdout}");
    let (stdout, _, code) = run(&["render", path]);
    assert_eq!(code, 0);
    assert!(stdout.contains("<svg"), "stdout should be SVG");
    assert!(stdout.contains("Alice"));
}

#[test]
fn test_cli_class_info_and_render() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/class.mmd");
    let path = path.to_str().unwrap();
    let (stdout, _, code) = run(&["info", path]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Kind: class"), "stdout: {stdout}");
    assert!(stdout.contains("Classes:"), "stdout: {stdout}");
    let (stdout, _, code) = run(&["render", path]);
    assert_eq!(code, 0);
    assert!(stdout.contains("<svg"));
    assert!(stdout.contains("Animal"));
}

#[test]
fn test_cli_gantt_info_and_render() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/gantt.mmd");
    let path = path.to_str().unwrap();
    let (stdout, _, code) = run(&["info", path]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Kind: gantt"), "stdout: {stdout}");
    assert!(stdout.contains("Tasks:"), "stdout: {stdout}");
    let (stdout, _, code) = run(&["render", path]);
    assert_eq!(code, 0);
    assert!(stdout.contains("<svg"));
    assert!(stdout.contains("Research"));
}

#[test]
fn test_cli_ir_json_document() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/simple-flowchart.mmd");
    let path = path.to_str().unwrap();
    let (stdout, _, code) = run(&["ir", path]);
    assert_eq!(code, 0);
    assert!(stdout.contains("\"version\": 1"), "stdout: {stdout}");
    assert!(stdout.contains("\"kind\": \"flowchart\""), "stdout: {stdout}");
}

#[test]
fn test_cli_metrics_flowchart() {
    let tmp = temp_mmd("graph TD\n    A-->B\n    B-->A\n    C[alone]\n");
    let path = tmp.to_str().unwrap();
    let (stdout, _, code) = run(&["metrics", path]);
    assert_eq!(code, 0);
    assert!(stdout.contains("\"kind\": \"flowchart\""), "stdout: {stdout}");
    assert!(stdout.contains("\"orphans\": 1"), "stdout: {stdout}");
    assert!(stdout.contains("cycle detected"), "stdout: {stdout}");
}

#[test]
fn test_cli_metrics_sequence() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/sequence.mmd");
    let (stdout, _, code) = run(&["metrics", path.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("\"kind\": \"sequence\""), "stdout: {stdout}");
    assert!(stdout.contains("\"messages\""), "stdout: {stdout}");
}
#[test]
fn test_cli_create_flowchart() {
    let out = std::env::temp_dir().join(format!("diagram_cli_create_{}.mmd", std::process::id()));
    let path = out.to_str().unwrap();
    let (stdout, _, code) = run(&["create", "--kind", "flowchart", "--output", path]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Created flowchart"), "stdout: {stdout}");
    let body = std::fs::read_to_string(&out).unwrap();
    assert!(body.contains("graph TD"));
    let _ = std::fs::remove_file(&out);
}

#[test]
fn test_cli_import_plantuml() {
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/sequence.puml");
    let json_out = std::env::temp_dir().join(format!("diagram_puml_ir_{}.json", std::process::id()));
    let (_, _, code) = run(&[
        "import",
        src.to_str().unwrap(),
        "--output",
        json_out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0);
    let body = std::fs::read_to_string(&json_out).unwrap();
    assert!(body.contains("\"kind\": \"sequence\""), "body: {body}");
    let _ = std::fs::remove_file(&json_out);
}

#[test]
fn test_cli_import_dot() {
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/simple-flowchart.dot");
    let json_out = std::env::temp_dir().join(format!("diagram_dot_ir_{}.json", std::process::id()));
    let (_, _, code) = run(&[
        "import",
        src.to_str().unwrap(),
        "--output",
        json_out.to_str().unwrap(),
        "--from",
        "dot",
    ]);
    assert_eq!(code, 0);
    let body = std::fs::read_to_string(&json_out).unwrap();
    assert!(body.contains("\"kind\": \"flowchart\""), "body: {body}");
    let _ = std::fs::remove_file(&json_out);
}

#[test]
fn test_cli_lossiness_json_lossless() {
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/simple-flowchart.mmd");
    let (stdout, _, code) = run(&["lossiness", src.to_str().unwrap(), "--to", "json"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("\"lossless\": true"), "stdout: {stdout}");
}

#[test]
fn test_cli_export_plantuml() {
    use std::fs;
    let src =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/sequence.puml");
    let json_out =
        std::env::temp_dir().join(format!("diagram_puml_ir_{}.json", std::process::id()));
    let puml_out =
        std::env::temp_dir().join(format!("diagram_puml_out_{}.puml", std::process::id()));
    let (_, _, code) = run(&[
        "import",
        src.to_str().unwrap(),
        "--output",
        json_out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0);
    let (_, _, code) = run(&[
        "export",
        json_out.to_str().unwrap(),
        "--output",
        puml_out.to_str().unwrap(),
        "--to",
        "plantuml",
    ]);
    assert_eq!(code, 0);
    let out = fs::read_to_string(&puml_out).unwrap();
    assert!(out.contains("@startuml"));
    assert!(out.contains("Alice"));
    let _ = fs::remove_file(&json_out);
    let _ = fs::remove_file(&puml_out);
}

#[test]
fn test_cli_export_dot() {
    use std::fs;
    let src =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/simple-flowchart.dot");
    let json_out = std::env::temp_dir().join(format!("diagram_dot_ir_{}.json", std::process::id()));
    let dot_out = std::env::temp_dir().join(format!("diagram_dot_out_{}.dot", std::process::id()));
    let (_, _, code) = run(&[
        "import",
        src.to_str().unwrap(),
        "--output",
        json_out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0);
    let (_, _, code) = run(&[
        "export",
        json_out.to_str().unwrap(),
        "--output",
        dot_out.to_str().unwrap(),
        "--to",
        "dot",
    ]);
    assert_eq!(code, 0);
    let out = fs::read_to_string(&dot_out).unwrap();
    assert!(out.contains("digraph"));
    assert!(out.contains("Start"));
    let _ = fs::remove_file(&json_out);
    let _ = fs::remove_file(&dot_out);
}

#[test]
fn test_cli_import_export_roundtrip() {
    use std::fs;
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/simple-flowchart.mmd");
    let json_out = std::env::temp_dir().join(format!("diagram_ir_{}.json", std::process::id()));
    let mmd_out = std::env::temp_dir().join(format!("diagram_out_{}.mmd", std::process::id()));
    let (_, _, code) = run(&[
        "import",
        src.to_str().unwrap(),
        "--output",
        json_out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0);
    let (_, _, code) = run(&[
        "export",
        json_out.to_str().unwrap(),
        "--output",
        mmd_out.to_str().unwrap(),
        "--to",
        "mermaid",
    ]);
    assert_eq!(code, 0);
    let out = fs::read_to_string(&mmd_out).unwrap();
    assert!(out.contains("graph"));
    let _ = fs::remove_file(&json_out);
    let _ = fs::remove_file(&mmd_out);
}
