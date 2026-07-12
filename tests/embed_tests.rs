//! Embed API coverage: string-in SVG/JSON without filesystem (Wasm path).

use std::fs;
use std::path::{Path, PathBuf};

fn embed_examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/embed")
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn assert_svg(svg: &str, needle: &str, path: &Path) {
    assert!(
        svg.contains("<svg"),
        "expected <svg> for {}",
        path.display()
    );
    assert!(
        svg.contains(needle),
        "expected {:?} in SVG for {}\nSVG head: {}",
        needle,
        path.display(),
        &svg[..svg.len().min(200)]
    );
}

#[test]
fn embed_renders_all_example_sources() {
    let cases: &[(&str, &str)] = &[
        ("flowchart.mmd", "Request"),
        ("sequence.mmd", "Client"),
        ("class.mmd", "User"),
        ("state.mmd", "Idle"),
        ("er.mmd", "CUSTOMER"),
        ("gantt.mmd", "Core"),
        ("flow.dot", "Ingest"),
        ("flow.d2", "Parse"),
        ("sequence.puml", "User"),
        ("sample.ir.json", "Embed"),
    ];

    for (name, needle) in cases {
        let path = embed_examples_dir().join(name);
        let source = read(&path);
        for theme in ["dark", "light", ""] {
            let svg = diagram::embed::render_to_svg(&source, theme).unwrap_or_else(|e| {
                panic!("render_to_svg failed for {} theme={theme:?}: {e}", path.display())
            });
            assert_svg(&svg, needle, &path);
        }
    }
}

#[test]
fn embed_parse_to_ir_json_all_examples() {
    let files = [
        "flowchart.mmd",
        "sequence.mmd",
        "class.mmd",
        "state.mmd",
        "er.mmd",
        "gantt.mmd",
        "flow.dot",
        "flow.d2",
        "sequence.puml",
        "sample.ir.json",
    ];

    for name in files {
        let path = embed_examples_dir().join(name);
        let source = read(&path);
        let json = diagram::embed::parse_to_ir_json(&source)
            .unwrap_or_else(|e| panic!("parse_to_ir_json {}: {e}", path.display()));
        assert!(
            json.contains("diagrams") || json.contains("Flowchart") || json.contains("Sequence"),
            "unexpected IR JSON for {}: {}",
            path.display(),
            &json[..json.len().min(120)]
        );
        // Round-trip: JSON IR renders again
        if name.ends_with(".json") || json.trim_start().starts_with('{') {
            let svg = diagram::embed::render_to_svg(&json, "dark")
                .unwrap_or_else(|e| panic!("render IR from {}: {e}", path.display()));
            assert!(svg.contains("<svg"), "IR render for {}", path.display());
        }
    }
}

#[test]
fn embed_mermaid_kinds_inline() {
    let samples = [
        ("graph TD\n  A[One] --> B[Two]\n", "One"),
        ("sequenceDiagram\n  A->>B: hi\n", "A"),
        ("classDiagram\n  class Foo\n  Foo --> Bar\n", "Foo"),
        ("stateDiagram-v2\n  [*] --> S\n  S --> [*]\n", "S"),
        ("erDiagram\n  A ||--o{ B : r\n", "A"),
        ("gantt\n  title T\n  dateFormat YYYY-MM-DD\n  section S\n  Task :a1, 2026-01-01, 1d\n", "Task"),
    ];
    for (src, needle) in samples {
        let svg = diagram::embed::render_to_svg(src, "dark").unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains(needle), "missing {needle} in {src}");
        let json = diagram::embed::parse_to_ir_json(src).unwrap();
        assert!(json.contains('{'));
    }
}

#[test]
fn embed_dot_and_d2_and_plantuml_inline() {
    let svg = diagram::embed::render_to_svg("digraph G { A -> B }", "light").unwrap();
    assert!(svg.contains("<svg"));

    let svg = diagram::embed::render_to_svg("a: Alpha\nb: Beta\na -> b\n", "dark").unwrap();
    assert!(svg.contains("<svg"));
    assert!(svg.contains("Alpha") || svg.contains("a"));

    let svg = diagram::embed::render_to_svg(
        "@startuml\nAlice -> Bob: hello\n@enduml\n",
        "dark",
    )
    .unwrap();
    assert!(svg.contains("<svg"));
    assert!(svg.contains("Alice") || svg.contains("Bob"));
}

#[test]
fn embed_json_ir_roundtrip() {
    let path = embed_examples_dir().join("sample.ir.json");
    let source = read(&path);
    let json = diagram::embed::parse_to_ir_json(&source).unwrap();
    let svg1 = diagram::embed::render_to_svg(&source, "dark").unwrap();
    let svg2 = diagram::embed::render_to_svg(&json, "light").unwrap();
    assert!(svg1.contains("Embed"));
    assert!(svg2.contains("Embed"));
}

#[test]
fn embed_rejects_invalid_theme() {
    let err = diagram::embed::render_to_svg("graph TD\n  A-->B\n", "neon").unwrap_err();
    assert!(err.contains("theme") || err.contains("neon"));
}

#[test]
fn embed_plantuml_activity_inline() {
    let src = "@startuml\nstart\n:Hello;\nstop\n@enduml\n";
    let svg = diagram::embed::render_to_svg(src, "dark").unwrap();
    assert!(svg.contains("<svg"));
}

#[test]
fn embed_d2_container_example_has_subgraph_in_ir() {
    let path = embed_examples_dir().join("flow.d2");
    let source = read(&path);
    let json = diagram::embed::parse_to_ir_json(&source).unwrap();
    assert!(
        json.contains("subgraphs") && (json.contains("core") || json.contains("parse")),
        "expected container subgraph in IR: {}",
        &json[..json.len().min(400)]
    );
    let svg = diagram::embed::render_to_svg(&source, "dark").unwrap();
    assert!(svg.contains("<svg"));
}

#[test]
fn embed_class_example_preserves_stereotype_in_ir() {
    let path = embed_examples_dir().join("class.mmd");
    let source = read(&path);
    let json = diagram::embed::parse_to_ir_json(&source).unwrap();
    assert!(
        json.to_lowercase().contains("interface") || json.contains("Repository"),
        "expected interface stereotype or Repository in {json}"
    );
}

#[test]
fn embed_top_level_examples_also_render() {
    // Sanity: existing root examples still work through the embed API.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");
    let samples = [
        "simple-flowchart.mmd",
        "sequence.mmd",
        "simple-flowchart.dot",
        "simple-flowchart.d2",
        "containers.d2",
        "multi-document.json",
    ];
    for name in samples {
        let path = root.join(name);
        let source = read(&path);
        let svg = diagram::embed::render_to_svg(&source, "dark")
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(svg.contains("<svg"), "{}", path.display());
    }
}
