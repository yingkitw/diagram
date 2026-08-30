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
    let path = temp_path("mmd");
    fs::write(&path, contents).unwrap();
    path
}

fn temp_path(ext: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!(
        "diagram_cli_{}_{}_{}.{}",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        ext
    ))
}

fn temp_dir() -> std::path::PathBuf {
    let dir = temp_path("d").with_extension("");
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_cli_validate_clean() {
    let tmp = temp_mmd("graph TD\n    A[Start] --> B[End]\n");
    let path = tmp.to_str().unwrap();
    let (stdout, _, code) = run(&["validate", path]);
    assert_eq!(code, 0);
    assert!(
        stdout.contains("Valid:") && stdout.contains("no issues found"),
        "stdout: {stdout}"
    );
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
    assert!(stdout.contains("entries"), "diff output should contain entries\nstdout: {stdout}");
    assert!(stdout.contains("added_nodes"), "diff output should contain flowchart added_nodes\nstdout: {stdout}");
    assert!(stdout.contains("C"), "diff output should mention added node C\nstdout: {stdout}");
}

#[test]
fn test_cli_merge() {
    let left = temp_mmd("graph TD\n    A[Start] --> B[End]\n");
    let right = temp_mmd("graph TD\n    A[Start] --> C[New]\n");
    let output = temp_path("mmd");
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
    let dir = temp_dir();
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
    let dir = temp_dir();
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
    let out = temp_path("pdf");
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
    let out = temp_path("png");
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
fn test_cli_state_info_and_render() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/state.mmd");
    let path = path.to_str().unwrap();
    let (stdout, _, code) = run(&["info", path]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Kind: state"), "stdout: {stdout}");
    assert!(stdout.contains("Transitions:"), "stdout: {stdout}");
    let (stdout, _, code) = run(&["render", path]);
    assert_eq!(code, 0);
    assert!(stdout.contains("<svg"));
    assert!(stdout.contains("Still"));
}

#[test]
fn test_cli_er_info_and_render() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/er.mmd");
    let path = path.to_str().unwrap();
    let (stdout, _, code) = run(&["info", path]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Kind: er"), "stdout: {stdout}");
    assert!(stdout.contains("Entities:"), "stdout: {stdout}");
    let (stdout, _, code) = run(&["render", path]);
    assert_eq!(code, 0);
    assert!(stdout.contains("<svg"));
    assert!(stdout.contains("CUSTOMER"));
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
    let out = temp_path("mmd");
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
    let json_out = temp_path("json");
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
    let json_out = temp_path("json");
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
        temp_path("json");
    let puml_out =
        temp_path("puml");
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
    let json_out = temp_path("json");
    let dot_out = temp_path("dot");
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
fn test_cli_import_d2() {
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/simple-flowchart.d2");
    let json_out = temp_path("json");
    let (_, _, code) = run(&[
        "import",
        src.to_str().unwrap(),
        "--output",
        json_out.to_str().unwrap(),
        "--from",
        "d2",
    ]);
    assert_eq!(code, 0);
    let body = std::fs::read_to_string(&json_out).unwrap();
    assert!(body.contains("\"kind\": \"flowchart\""), "body: {body}");
    let _ = std::fs::remove_file(&json_out);
}

#[test]
fn test_cli_export_d2() {
    use std::fs;
    let src =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/simple-flowchart.d2");
    let json_out = temp_path("json");
    let d2_out = temp_path("d2");
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
        d2_out.to_str().unwrap(),
        "--to",
        "d2",
    ]);
    assert_eq!(code, 0);
    let out = fs::read_to_string(&d2_out).unwrap();
    assert!(out.contains("start"));
    assert!(out.contains("Start"));
    let _ = fs::remove_file(&json_out);
    let _ = fs::remove_file(&d2_out);
}

#[test]
fn test_cli_import_export_roundtrip() {
    use std::fs;
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/simple-flowchart.mmd");
    let json_out = temp_path("json");
    let mmd_out = temp_path("mmd");
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

#[test]
fn test_cli_generate_class_from_rust() {
    use std::fs;
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/code-sample.rs");
    let out = temp_path("mmd");
    let (stdout, _, code) = run(&[
        "generate-class",
        src.to_str().unwrap(),
        "--output",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stdout: {stdout}");
    let body = fs::read_to_string(&out).unwrap();
    assert!(body.contains("classDiagram"), "missing classDiagram header: {body}");
    assert!(body.contains("Point"), "missing Point class: {body}");
    assert!(body.contains("Shape"), "missing Shape trait: {body}");
    let _ = fs::remove_file(&out);
}

#[test]
fn test_cli_generate_tree_from_rust() {
    use std::fs;
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/code-sample.rs");
    let out = temp_path("mmd");
    let (stdout, _, code) = run(&[
        "generate-tree",
        src.to_str().unwrap(),
        "--output",
        out.to_str().unwrap(),
    ]);
assert_eq!(code, 0, "stdout: {stdout}");
    let body = fs::read_to_string(&out).unwrap();
    assert!(body.contains("graph"), "missing graph header: {body}");
    assert!(body.contains("compute"), "missing compute node: {body}");
    assert!(body.contains("adjust"), "missing adjust callee: {body}");
    let _ = fs::remove_file(&out);
}

#[test]
fn test_cli_generate_class_to_json_ir() {
    use std::fs;
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/code-sample.rs");
    let out = temp_path("json");
    let (stdout, _, code) = run(&[
        "generate-class",
        src.to_str().unwrap(),
        "--output",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stdout: {stdout}");
    let body = fs::read_to_string(&out).unwrap();
    let value: serde_json::Value =
        serde_json::from_str(&body).expect("generated json must parse");
    assert_eq!(value["version"], 1);
    let diagrams = value["diagrams"].as_array().expect("diagrams array");
    assert_eq!(diagrams.len(), 1);
    assert_eq!(diagrams[0]["kind"], "class");
    let classes = diagrams[0]["data"]["classes"].as_array().unwrap();
    let names: Vec<&str> = classes
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"Point"));
    assert!(names.contains(&"Shape"));
    let _ = fs::remove_file(&out);
}

#[test]
fn test_cli_generate_tree_explicit_lang_typescript() {
    use std::fs;
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/code-sample.ts");
    let out = temp_path("mmd");
    let (stdout, _, code) = run(&[
        "generate-tree",
        src.to_str().unwrap(),
        "--output",
        out.to_str().unwrap(),
        "--lang",
        "typescript",
    ]);
    assert_eq!(code, 0, "stdout: {stdout}");
    let body = fs::read_to_string(&out).unwrap();
    assert!(body.contains("class::Circle"));
    assert!(body.contains("interface::Shape"));
    let _ = fs::remove_file(&out);
}

#[test]
fn test_cli_generate_call_to_dot() {
    use std::fs;
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/code-sample.rs");
    let out = temp_path("dot");
    let (stdout, _, code) = run(&[
        "generate-call",
        src.to_str().unwrap(),
        "--output",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stdout: {stdout}");
    let body = fs::read_to_string(&out).unwrap();
    assert!(body.contains("digraph"), "missing digraph header: {body}");
    assert!(body.contains("compute"));
    let _ = fs::remove_file(&out);
}

#[test]
fn test_cli_generate_skeleton_from_rust_class() {
    use std::fs;
    // Generate a class diagram from Rust source first, then skeleton that.
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/code-sample.rs");
    let diagram = temp_path("mmd");
    let (_, _, code) = run(&[
        "generate-class",
        src.to_str().unwrap(),
        "--output",
        diagram.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "class generate failed");
    let skeleton = temp_path("rs");
    let (stdout, stderr, code) = run(&[
        "generate-skeleton",
        diagram.to_str().unwrap(),
        "--lang",
        "rust",
        "--output",
        skeleton.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    let body = fs::read_to_string(&skeleton).unwrap();
    assert!(body.contains("struct Point"), "missing Point struct: {body}");
    assert!(body.contains("enum Color"), "missing Color enum: {body}");
    assert!(
        body.contains("impl Shape for Point"),
        "missing trait impl: {body}"
    );
    let _ = fs::remove_file(&diagram);
    let _ = fs::remove_file(&skeleton);
}

#[test]
fn test_cli_generate_skeleton_from_typescript_class() {
    use std::fs;
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/code-sample.ts");
    let diagram = temp_path("mmd");
    let (_, _, code) = run(&[
        "generate-class",
        src.to_str().unwrap(),
        "--output",
        diagram.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "class generate failed");
    let skeleton = temp_path("ts");
    let (stdout, _, code) = run(&[
        "generate-skeleton",
        diagram.to_str().unwrap(),
        "--lang",
        "typescript",
        "--output",
        skeleton.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stdout: {stdout}");
    let body = fs::read_to_string(&skeleton).unwrap();
    assert!(body.contains("interface Shape"), "missing Shape: {body}");
    assert!(body.contains("class Point"), "missing Point: {body}");
    assert!(
        body.contains("implements Shape"),
        "missing implements: {body}"
    );
    let _ = fs::remove_file(&diagram);
    let _ = fs::remove_file(&skeleton);
}

#[test]
fn test_cli_generate_skeleton_from_flowchart() {
    use std::fs;
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/code-sample.rs");
    let diagram = temp_path("mmd");
    let (_, _, code) = run(&[
        "generate-tree",
        src.to_str().unwrap(),
        "--output",
        diagram.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "tree generate failed");
    let skeleton = temp_path("rs");
    let (stdout, _, code) = run(&[
        "generate-skeleton",
        diagram.to_str().unwrap(),
        "--lang",
        "rust",
        "--output",
        skeleton.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stdout: {stdout}");
    let body = fs::read_to_string(&skeleton).unwrap();
    assert!(body.contains("fn compute"), "missing compute fn: {body}");
    assert!(body.contains("todo!()"), "missing stub body: {body}");
    let _ = fs::remove_file(&diagram);
    let _ = fs::remove_file(&skeleton);
}

#[test]
fn test_cli_generate_skeleton_unknown_language() {
    use std::fs;
    let diagram = temp_path("mmd");
    fs::write(&diagram, "classDiagram\n  class Foo\n").unwrap();
    let out = temp_path("py");
    let (_, stderr, code) = run(&[
        "generate-skeleton",
        diagram.to_str().unwrap(),
        "--lang",
        "python",
        "--output",
        out.to_str().unwrap(),
    ]);
    assert_ne!(code, 0);
    assert!(stderr.contains("unsupported language"));
    let _ = fs::remove_file(&diagram);
}

#[test]
fn test_cli_generate_call_from_rust() {
    use std::fs;
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/code-sample.rs");
    let out = temp_path("mmd");
    let (stdout, _, code) = run(&[
        "generate-call",
        src.to_str().unwrap(),
        "--output",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stdout: {stdout}");
    let body = fs::read_to_string(&out).unwrap();
    assert!(body.contains("graph"), "missing graph header: {body}");
    assert!(body.contains("compute"), "missing compute node: {body}");
    assert!(body.contains("adjust"), "missing adjust callee: {body}");
    let _ = fs::remove_file(&out);
}

#[test]
fn test_cli_generate_class_unknown_extension() {
    use std::fs;
    let bad = temp_path("xyz");
    fs::write(&bad, "fn main() {}").unwrap();
    let out = temp_path("mmd");
    let (_, stderr, code) = run(&[
        "generate-class",
        bad.to_str().unwrap(),
        "--output",
        out.to_str().unwrap(),
    ]);
    assert_ne!(code, 0, "expected failure for unknown extension");
    assert!(
        stderr.contains("unsupported source extension") || stderr.contains("Failed"),
        "stderr should mention unsupported extension: {stderr}"
    );
    let _ = fs::remove_file(&bad);
    let _ = fs::remove_file(&out);
}

#[test]
fn test_cli_generate_class_from_typescript() {
    use std::fs;
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/code-sample.ts");
    let out = temp_path("mmd");
    let (stdout, _, code) = run(&[
        "generate-class",
        src.to_str().unwrap(),
        "--output",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stdout: {stdout}");
    let body = fs::read_to_string(&out).unwrap();
    assert!(body.contains("classDiagram"), "missing classDiagram header: {body}");
    assert!(body.contains("Circle"), "missing Circle class: {body}");
    assert!(body.contains("Shape"), "missing Shape interface: {body}");
    // implements → Realization
    assert!(body.contains("Circle ..|> Shape"), "missing realization edge: {body}");
    let _ = fs::remove_file(&out);
}

#[test]
fn test_cli_generate_tree_from_typescript() {
    use std::fs;
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/code-sample.ts");
    let out = temp_path("mmd");
    let (stdout, _, code) = run(&[
        "generate-tree",
        src.to_str().unwrap(),
        "--output",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stdout: {stdout}");
    let body = fs::read_to_string(&out).unwrap();
    assert!(body.contains("graph"), "missing graph header: {body}");
    assert!(body.contains("Circle"), "missing Circle node: {body}");
    assert!(body.contains("Shape"), "missing Shape interface node: {body}");
    assert!(body.contains("import::"), "missing import edge: {body}");
    let _ = fs::remove_file(&out);
}

#[test]
fn test_cli_generate_call_from_typescript() {
    use std::fs;
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/code-sample.ts");
    let out = temp_path("mmd");
    let (stdout, _, code) = run(&[
        "generate-call",
        src.to_str().unwrap(),
        "--output",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stdout: {stdout}");
    let body = fs::read_to_string(&out).unwrap();
    assert!(body.contains("graph"), "missing graph header: {body}");
    assert!(body.contains("compute"), "missing compute node: {body}");
    assert!(body.contains("adjust"), "missing adjust callee: {body}");
    let _ = fs::remove_file(&out);
}

#[test]
fn test_cli_create_template() {
    let out = temp_path("mmd");
    let path = out.to_str().unwrap();
    let (stdout, _, code) = run(&["create", "--template", "aws-3tier", "--output", path]);
    assert_eq!(code, 0, "stdout: {stdout}");
    let body = fs::read_to_string(&out).unwrap();
    assert!(body.contains("CloudFront") || body.contains("Cloudfront"), "body: {body}");
    let _ = fs::remove_file(&out);
}

#[test]
fn test_cli_create_template_to_json() {
    let out = temp_path("json");
    let path = out.to_str().unwrap();
    let (stdout, _, code) = run(&["create", "--template", "gcp-microservices", "--output", path]);
    assert_eq!(code, 0, "stdout: {stdout}");
    let body = fs::read_to_string(&out).unwrap();
    assert!(body.contains("\"kind\": \"flowchart\"") || body.contains("\"kind\":\"flowchart\""), "body: {body}");
    let _ = fs::remove_file(&out);
}

#[test]
fn test_cli_list_templates() {
    let (stdout, _, code) = run(&["list-templates"]);
    assert_eq!(code, 0, "stdout: {stdout}");
    assert!(stdout.contains("aws-3tier"), "stdout: {stdout}");
    assert!(stdout.contains("gcp-microservices"), "stdout: {stdout}");
    assert!(stdout.contains("azure-hub-spoke"), "stdout: {stdout}");
}

#[test]
fn test_cli_render_ascii() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/simple-flowchart.mmd");
    let out = temp_path("txt");
    let (stdout, _, code) = run(&[
        "render",
        path.to_str().unwrap(),
        "--output",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stdout: {stdout}");
    let body = fs::read_to_string(&out).unwrap();
    assert!(body.contains('+'), "expected ASCII box border: {body}");
    assert!(body.contains("Start") || body.contains("Fix Issue"), "expected a label: {body}");
    let _ = fs::remove_file(&out);
}

#[test]
fn test_cli_import_drawio() {
    let xml = "<?xml version=\"1.0\"?>\n<mxfile>\n  <diagram id=\"d0\" name=\"Page-1\">\n    <mxGraphModel>\n      <root>\n        <mxCell id=\"0\"/>\n        <mxCell id=\"1\" parent=\"0\"/>\n        <mxCell id=\"2\" value=\"Alpha\" style=\"rounded=0;whiteSpace=wrap;html=1;\" vertex=\"1\" parent=\"1\"/>\n        <mxCell id=\"3\" value=\"Beta\" style=\"rounded=1;whiteSpace=wrap;html=1;\" vertex=\"1\" parent=\"1\"/>\n        <mxCell id=\"4\" value=\"link\" style=\"endArrow=classic;html=1;\" edge=\"1\" parent=\"1\" source=\"2\" target=\"3\"/>\n      </root>\n    </mxGraphModel>\n  </diagram>\n</mxfile>\n";
    let src = temp_path("drawio");
    fs::write(&src, xml).unwrap();
    let json_out = temp_path("json");
    let (stdout, _, code) = run(&[
        "import",
        src.to_str().unwrap(),
        "--output",
        json_out.to_str().unwrap(),
        "--from",
        "drawio",
    ]);
    assert_eq!(code, 0, "stdout: {stdout}");
    let body = fs::read_to_string(&json_out).unwrap();
    assert!(body.contains("Alpha"), "body: {body}");
    assert!(body.contains("Beta"), "body: {body}");
    assert!(body.contains("link"), "body: {body}");
    let _ = fs::remove_file(&src);
    let _ = fs::remove_file(&json_out);
}

#[test]
fn test_cli_export_drawio_roundtrip() {
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/simple-flowchart.mmd");
    let drawio_out = temp_path("drawio");
    let (stdout, _, code) = run(&[
        "export",
        src.to_str().unwrap(),
        "--output",
        drawio_out.to_str().unwrap(),
        "--to",
        "drawio",
    ]);
    assert_eq!(code, 0, "stdout: {stdout}");
    let xml = fs::read_to_string(&drawio_out).unwrap();
    assert!(xml.contains("<mxfile"), "xml: {xml}");
    assert!(xml.contains("<diagram"), "xml: {xml}");
    // Re-import to IR and check the key node labels survive.
    let json_out = temp_path("json");
    let (stdout2, _, code2) = run(&[
        "import",
        drawio_out.to_str().unwrap(),
        "--output",
        json_out.to_str().unwrap(),
        "--from",
        "drawio",
    ]);
    assert_eq!(code2, 0, "stdout: {stdout2}");
    let body = fs::read_to_string(&json_out).unwrap();
    assert!(body.contains("Start") || body.contains("Is it working?"), "roundtrip body: {body}");
    let _ = fs::remove_file(&drawio_out);
    let _ = fs::remove_file(&json_out);
}

#[test]
fn test_cli_lossiness_drawio() {
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/simple-flowchart.mmd");
    let (stdout, _, code) = run(&["lossiness", src.to_str().unwrap(), "--to", "drawio"]);
    assert_eq!(code, 0, "stdout: {stdout}");
    assert!(stdout.contains("drawio"), "stdout: {stdout}");
}
