use std::fs;

fn examples_dir() -> std::path::PathBuf {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("examples");
    path
}

#[test]
fn test_example_simple_flowchart() {
    let path = examples_dir().join("simple-flowchart.mmd");
    let source = fs::read_to_string(&path).unwrap();
    let diagram = diagram::parser::parse(&source).unwrap();
    assert!(diagram.nodes.len() >= 4);
    assert!(diagram.edges.len() >= 4);
}

#[test]
fn test_example_shapes() {
    let path = examples_dir().join("shapes.mmd");
    let source = fs::read_to_string(&path).unwrap();
    let diagram = diagram::parser::parse(&source).unwrap();
    assert_eq!(diagram.nodes.len(), 6);
    assert_eq!(diagram.edges.len(), 5);
}

#[test]
fn test_example_edge_styles() {
    let path = examples_dir().join("edge-styles.mmd");
    let source = fs::read_to_string(&path).unwrap();
    let diagram = diagram::parser::parse(&source).unwrap();
    assert_eq!(diagram.nodes.len(), 5);
    assert_eq!(diagram.edges.len(), 4);
}

#[test]
fn test_example_subgraphs() {
    let path = examples_dir().join("subgraphs.mmd");
    let source = fs::read_to_string(&path).unwrap();
    let diagram = diagram::parser::parse(&source).unwrap();
    assert!(diagram.nodes.len() >= 5);
    assert!(diagram.subgraphs.len() >= 2);
}

#[test]
fn test_example_styling() {
    let path = examples_dir().join("styling.mmd");
    let source = fs::read_to_string(&path).unwrap();
    let diagram = diagram::parser::parse(&source).unwrap();
    assert!(diagram.styles.len() >= 1);
    assert!(diagram.class_defs.len() >= 1);
    assert!(diagram.class_applies.len() >= 1);
}

#[test]
fn test_example_quoted_ids() {
    let path = examples_dir().join("quoted-ids.mmd");
    let source = fs::read_to_string(&path).unwrap();
    let diagram = diagram::parser::parse(&source).unwrap();
    assert_eq!(diagram.nodes.len(), 4);
    assert!(diagram.nodes.iter().any(|n| n.id == "user login"));
    assert!(diagram.nodes.iter().any(|n| n.id == "auth service"));
}

#[test]
fn test_all_examples_roundtrip() {
    for entry in fs::read_dir(examples_dir()).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("mmd") {
            let source = fs::read_to_string(&path).unwrap();
            let diagram = diagram::parser::parse(&source).unwrap();
            let output = diagram.to_mermaid();
            let reparsed = diagram::parser::parse(&output).unwrap();
            assert_eq!(diagram.nodes.len(), reparsed.nodes.len(), "node count mismatch for {:?}", path);
            assert_eq!(diagram.edges.len(), reparsed.edges.len(), "edge count mismatch for {:?}", path);
        }
    }
}

#[test]
fn test_render_shapes_example() {
    let path = examples_dir().join("shapes.mmd");
    let source = fs::read_to_string(&path).unwrap();
    let diagram = diagram::parser::parse(&source).unwrap();
    let layout = diagram::layout::layout(&diagram);
    let svg = diagram::renderer::render_svg(&layout);
    assert!(svg.starts_with("<svg"));
    assert!(svg.ends_with("</svg>"));
}
