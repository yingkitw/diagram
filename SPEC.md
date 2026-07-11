# Specification

## Overview

`diagram` is a Rust CLI and MCP **diagram platform**: render, generate, analyze, and interchange diagrams via a canonical **IR**, with **Compatibility** adapters for Mermaid (today) and PlantUML/DOT/D2 (planned). See `CONTEXT.md` and `docs/adr/0001-canonical-ir-and-format-adapters.md`.

**Current shipping surface:** Mermaid or JSON IR → `Document`; parse/ir/import/export; info/render/preview; flowchart generate/edit; validate/diff/merge; SVG export.

## Canonical JSON IR (shipped)

```json
{
  "version": 1,
  "diagrams": [
    {
      "kind": "flowchart",
      "data": { "rankdir": "TD", "nodes": [], "edges": [], ... }
    }
  ]
}
```

`kind` values: `flowchart`, `sequence`, `class`, `gantt`. Multi-diagram `diagrams[]` is schema-ready; single-diagram export/render today.

## Platform contracts

| Capability | Contract |
|------------|----------|
| Import | `Format` bytes → `Document` IR (`import`, `import_diagram`) |
| Export | `Document` IR → `Format` bytes (`export`, `export_diagram`) |
| Render | `Document` / `Diagram` → SVG (PNG/PDF later) |
| Analyze | `Document` → validation issues, diff, metrics JSON |
| Generate | MCP/CLI mutations and scaffolds against IR |

Supported **Formats** today: `mermaid`, `json` (native IR). Detection: JSON object prefix or `.json` / `.mmd` extension.

## Supported Mermaid Syntax (Compatibility)

Mermaid remains the primary authored Format in this version. Syntax below is Compatibility coverage, not the long-term identity of the product.

### Flowcharts

- **Direction**: `graph TD`, `graph LR`, `graph RL`, `graph BT`
- **Nodes**:
  - `A[text]` — rectangle (default)
  - `A{text}` — diamond
  - `A{{text}}` — hexagon
  - `A(text)` — stadium / rounded
  - `A((text))` — circle
  - `A[(text)]` — cylinder
  - `A` — bare rectangle (id as text)
  - `"my id"[text]` — quoted ID with special characters
- **Edges**: `A --> B` (directed), `A -->|label| B` (labeled)
- **Edge types**: `-.->` (dashed), `==>` (thick)
- **Subgraphs**: `subgraph Name ... end`
- **Styling**: `style NodeId fill:#f9f`, `classDef name fill:#bbf`, `class A,B name`
- **Comments**: `%% line comment`

### Sequence diagrams (MVP)

- **Header**: `sequenceDiagram`
- **Participants**: `participant A`, `participant A as Alice`, `actor A`
- **Messages**: `A->>B: text` (solid), `A-->>B: text` (dashed)
- Implicit participants created from message endpoints
- Rendered with lifelines, header/footer boxes, and labeled arrows
- Not yet: notes, loops, alt/opt, activations, self-messages

### Class diagrams (MVP)

- **Header**: `classDiagram`
- **Classes**: `class Name`, `class Name { members }`, `Name : +member`
- **Relations**: `<|--` inheritance, `*--` composition, `o--` aggregation, `-->` association, `--` link, `..>` dependency, `..|>` realization
- Optional relation labels after `:`
- Layered SVG layout (parents above children)
- Not yet: interfaces, generics, cardinality, notes

### Gantt charts (MVP)

- **Header**: `gantt`
- **Meta**: `title ...`, `dateFormat YYYY-MM-DD` (only this format)
- **Sections**: `section Name`
- **Tasks**: `Name : [tags,] [id,] start|after id, duration|end`
- Tags: `crit`, `active`, `done`
- Durations: `Nd` / `Nh`; `after <id>` scheduling
- SVG timeline with section labels and colored bars
- Not yet: milestones, excludes, today marker, other dateFormats

## Error Handling

The parser returns structured errors with optional line numbers:
```rust
pub struct ParseError {
    pub message: String,
    pub line: Option<usize>,
}
```

## CLI Interface

```
diagram <COMMAND>

Commands:
  parse        Parse and print canonical JSON IR
  ir           Alias for parse (canonical JSON IR)
  import       Import Mermaid/DOT/JSON → JSON IR file
  export       Export diagram → Mermaid or JSON IR
  info         Show diagram summary
  render       Render as SVG (use --watch and --theme)
  mcp          Start MCP server (stdio)
  add-node     Add a node
  remove-node  Remove a node
  update-node  Update node text/shape
  add-edge     Add an edge
  remove-edge  Remove an edge
  update-edge  Update edge label/style
  get-node     Get a single node
  get-edge     Get a single edge
  get-mermaid  Get raw mermaid source
  set-mermaid  Write raw mermaid source
  list-nodes   List all nodes
  list-edges   List all edges
  validate     Validate diagram
  metrics      Structural metrics as JSON
  create       Create a new diagram scaffold
  diff         Compare two diagrams
  merge        Merge two diagrams
  preview      Live browser SVG preview
```

## MCP Tools

All tools accept `path` (path to `.mmd` file) plus operation-specific parameters:

| Tool | Parameters | Returns |
|------|-----------|---------|
| `parse_diagram` | path | Canonical JSON IR |
| `get_info` | path | Summary JSON (kind, counts, ir_version) |
| `import_diagram` | path, output, from? | Status JSON |
| `export_diagram` | path, output, to? | Status JSON |
| `metrics_diagram` | path | Metrics JSON |
| `create_diagram` | kind, output | Status JSON |
| `render_svg` | path, theme? | SVG string |
| `diff_diagram` | left, right | Diff JSON |
| `merge_diagram` | left, right, output | Status JSON |
| `add_node` | path, id, text, shape? | Status JSON |
| `remove_node` | path, id | Status JSON |
| `update_node` | path, id, text?, shape? | Status JSON |
| `add_edge` | path, from, to, label?, style? | Status JSON |
| `remove_edge` | path, from, to | Status JSON |
| `get_mermaid` | path | Mermaid source |
| `set_mermaid` | path, source | Status JSON |
| `list_nodes` | path | Node list JSON |
| `list_edges` | path | Edge list JSON |
| `update_edge` | path, from, to, label?, style? | Status JSON |
| `get_node` | path, id | Node JSON |
| `get_edge` | path, from, to | Edge JSON |
| `validate_diagram` | path | Validation JSON |
| `add_nodes` | path, items[] | Status JSON |
| `add_edges` | path, items[] | Status JSON |

## Data Model

```rust
struct Diagram {
    rankdir: String,  // "TB", "LR", "RL", "BT"
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    subgraphs: Vec<Subgraph>,
    styles: Vec<NodeStyle>,
    class_defs: Vec<ClassDef>,
    class_applies: Vec<ClassApply>,
}

struct Node {
    id: String,
    text: String,
    shape: NodeShape,  // Rect | Diamond | Stadium | Hexagon | Cylinder | Circle
}

struct Edge {
    from: String,
    to: String,
    label: String,
    style: EdgeStyle,  // Arrow | Dashed | Thick
}

struct Subgraph {
    id: String,
    nodes: Vec<String>,
}

struct NodeStyle {
    node_id: String,
    properties: String,
}

struct ClassDef {
    name: String,
    properties: String,
}

struct ClassApply {
    node_ids: Vec<String>,
    class_name: String,
}
```

## Library Interface

The crate exposes a public library API via `src/lib.rs`:
```rust
pub mod analyze;
pub mod class;
pub mod cli;
pub mod diagram;
pub mod formats;
pub mod gantt;
pub mod generate;
pub mod ir;
pub mod layout;
pub mod mcp;
pub mod parser;
pub mod preview;
pub mod renderer;
pub mod sequence;
```

All core modules are importable for programmatic use or integration tests.

## Preview Server

`diagram preview <file.mmd> [--port 3030] [--theme dark|light]` starts a localhost HTTP server:

| Path | Response |
|------|----------|
| `/` | HTML page that polls `/svg` every second |
| `/svg` | Current SVG render of the file |
| `/health` | `ok` |

No extra HTTP dependencies — uses `tokio::net::TcpListener` with a minimal HTTP/1.1 responder.

## Rendering

SVG output uses a layered graph layout algorithm:
1. Sources (no incoming edges) are assigned layer 0
2. BFS assigns subsequent layers
3. Nodes within each layer are positioned with even spacing
4. Edges are drawn as straight lines with arrow markers
5. Dark theme by default

**Features:** `style` directives and `classDef`/`class` are fully applied to node fill/stroke. Subgraphs are rendered as bounding boxes with dashed borders. `linkStyle` applies stroke color and width to edges. Edges use smooth cubic bezier curves with barycenter crossing-reduction. Nodes can have clickable `href` links and `<title>` tooltips in SVG. Light and dark themes are supported.

## MCP Resources

| Template | Description |
|----------|-------------|
| `file://{path}` | Read any `.mmd` file as a text resource |

## MCP Prompts

| Prompt | Description |
|--------|-------------|
| `create_flowchart` | Create a new flowchart from a description |
| `refactor_diagram` | Refactor an existing diagram for clarity |

## Transport

The MCP server uses stdio transport (JSON-RPC over stdin/stdout). Suitable for local AI assistant integration.
