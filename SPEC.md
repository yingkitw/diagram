# Specification

## Overview

`diagram` is a Rust CLI and MCP **diagram platform**: render, generate, analyze, and interchange diagrams via a canonical **IR**, with **Compatibility** adapters for Mermaid, Graphviz DOT, PlantUML (sequence, class, activity), and more planned. See `CONTEXT.md` and `docs/adr/0001-canonical-ir-and-format-adapters.md`.

**Current shipping surface:** Mermaid, JSON IR, DOT, D2, PlantUML → `Document`; `import`/`export`/`lossiness`; info/render/preview (SVG/PNG/PDF); flowchart generate/edit; validate/diff/merge/metrics; markdown pipeline; multi-diagram documents.

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

`kind` values: `flowchart`, `sequence`, `class`, `gantt`. Multi-diagram `diagrams[]` is supported for JSON IR import, composite render, per-index render (`--index`), output-dir batch render, and Mermaid export with `%% diagram N:` markers.

## Platform contracts

| Capability | Contract |
|------------|----------|
| Import | `Format` bytes → `Document` IR (`import`, `import_diagram`) |
| Export | `Document` IR → `Format` bytes (`export`, `export_diagram`) |
| Lossiness | Export fidelity report per target Format (`lossiness`, `export --report`, `lossiness_report`) |
| Render | `Document` / `Diagram` → SVG, PNG, or PDF (raster) |
| Analyze | `Document` → validation issues, diff, metrics JSON |
| Generate | MCP/CLI mutations and scaffolds against IR |

### Supported interchange formats

| Format | Import | Export | Notes |
|--------|--------|--------|-------|
| `mermaid` / `mmd` | ✓ | ✓ | All kinds; multi-diagram uses `%% diagram N:` markers |
| `json` / `ir` | ✓ | ✓ | Lossless native IR |
| `dot` / `gv` | ✓ flowchart | ✓ flowchart | Digraph subset |
| `d2` | ✓ flowchart | ✓ flowchart | Flat flowchart subset |
| `plantuml` / `puml` | ✓ sequence, class, activity | ✓ sequence, class, activity-shaped flowchart | Activity imports as flowchart IR |

Detection: content heuristics (`@startuml`, `digraph`, `direction:`, `{` JSON) plus path extension. Override with `--from` / `--to` on CLI or MCP.

## Supported Mermaid Syntax (Compatibility)

Mermaid remains the primary authored Format. Syntax below is Compatibility coverage, not the long-term identity of the product.

### Flowcharts

- **Direction**: `graph TD`, `graph LR`, `graph RL`, `graph BT`
- **Nodes**: `[text]` rect, `{text}` diamond, `{{text}}` hexagon, `(text)` stadium, `((text))` circle, `[(text)]` cylinder, bare id, quoted ids
- **Edges**: `-->`, `-.->`, `==>`, optional `|label|`
- **Subgraphs**: `subgraph Name ... end`
- **Styling**: `style`, `classDef`, `class`, `linkStyle`
- **Interactive SVG**: `href`, tooltip via click targets in render output

### Sequence diagrams (MVP)

- `sequenceDiagram`, participants, `->>` / `-->>` messages
- Not yet: notes, loops, alt/opt, activations, self-messages

### Class diagrams (MVP)

- `classDiagram`, classes with members, relations (`<|--`, `*--`, `o--`, `-->`, `--`, `..>`, `..|>`)
- Not yet: interfaces, generics, cardinality, notes

### Gantt charts (MVP)

- `gantt`, `title`, `dateFormat YYYY-MM-DD`, sections, tasks with `crit`/`active`/`done`
- Not yet: milestones, excludes, today marker, other dateFormats

## PlantUML Compatibility (subset)

### Sequence

- `@startuml` … `@enduml`, `participant` / `actor`, `->` / `-->` messages

### Class

- `class`, `interface`, `enum`, `abstract class`, members, relations (same tokens as Mermaid class)

### Activity

- `start` / `stop` / `end`, `:action;`, `if (cond) then (label)` / `else (label)` / `endif`
- Import and export (activity-shaped flowcharts with `start` stadium node)

## Graphviz DOT Compatibility (subset)

- `digraph` / `graph`, `rankdir`, node `[label=, shape=]`, edges `->` / `--`, chained edges, `subgraph`
- Import and export map to flowchart IR
- Not yet: ports, HTML labels, full Graphviz attribute surface

## D2 Compatibility (subset)

- `direction`, node `label` / `shape`, connections `->` / `<-` / `--` / `<->`, edge labels, dashed edges via `style.stroke-dash`
- Import and export map to flowchart IR (flat graphs; containers/subgraphs export as D2 blocks)
- Not yet: sequence/class shapes, nested container import, themes, icons, layout engine options

## Lossiness reporting

`lossiness` analyzes what IR semantics a target Format cannot represent before export:

- JSON IR: lossless
- Mermaid: warns on `href`/`tooltip`, multi-diagram marker convention
- DOT: flowchart only; warns on skipped kinds, styles/classDefs, href/tooltip
- D2: flowchart only; warns on skipped kinds, styles/classDefs, href/tooltip, subgraph flattening
- PlantUML export: sequence/class only; flowchart/gantt omitted

Blocked exports return an error with the first warning message.

## Error Handling

Parsers and adapters return structured errors with optional line numbers:

```rust
pub struct ParseError {
    pub message: String,
    pub line: Option<usize>,
}
```

IR/import/export errors use `IrError` (string messages).

## CLI Interface

```
diagram <COMMAND>

Commands:
  parse        Parse and print canonical JSON IR
  ir           Alias for parse (canonical JSON IR)
  import       Import Mermaid/DOT/D2/PlantUML/JSON → JSON IR file
  export       Export IR → Mermaid, JSON, DOT, D2, or PlantUML
  lossiness    Report export fidelity / unsupported fields
  info         Show diagram summary
  render       Render as SVG, PNG, or PDF (use --output extension, --watch, --theme)
  preview      Live browser SVG preview
  validate     Validate diagram
  metrics      Structural metrics as JSON
  create       Create a new diagram scaffold
  diff         Compare two documents (any import format; IR-level structural diff)
  merge        Merge two diagrams
  markdown     Render fenced diagram blocks; rewrite image links
  mcp          Start MCP server (stdio)
  add-node | remove-node | update-node | add-edge | remove-edge | update-edge ...
  get-node | get-edge | list-nodes | list-edges | get-mermaid | set-mermaid ...
```

`render` flags: `--output` (`.svg`/`.png`/`.pdf`), `--output-dir` (batch; raster hint from `--output` extension), `--index`, `--watch`, `--theme dark|light`.

## MCP Tools

| Tool | Parameters | Returns |
|------|-----------|---------|
| `parse_diagram` | path | Canonical JSON IR |
| `get_info` | path | Summary JSON (kind, counts, ir_version) |
| `import_diagram` | path, output, from? | Status JSON |
| `export_diagram` | path, output, to? | Status JSON + lossiness |
| `lossiness_report` | path, to? | Lossiness JSON |
| `metrics_diagram` | path | Metrics JSON |
| `create_diagram` | kind, output | Status JSON |
| `render_svg` | path, theme? | SVG string |
| `render_png` | path, output, theme? | Status JSON |
| `render_pdf` | path, output, theme? | Status JSON |
| `process_markdown` | path, output_dir, output, format?, theme? | Status JSON |
| `diff_diagram` | left, right | `DocumentDiff` JSON (per-diagram entries, summary) |
| `merge_diagram` | left, right, output | Status JSON |
| `add_node` | path, id, text, shape? | Status JSON |
| `remove_node` | path, id | Status JSON |
| `update_node` | path, id, text?, shape? | Status JSON |
| `add_edge` | path, from, to, label?, style? | Status JSON |
| `remove_edge` | path, from, to | Status JSON |
| `update_edge` | path, from, to, label?, style? | Status JSON |
| `get_mermaid` | path | Mermaid source |
| `set_mermaid` | path, source | Status JSON |
| `list_nodes` | path | Node list JSON |
| `list_edges` | path | Edge list JSON |
| `get_node` | path, id | Node JSON |
| `get_edge` | path, from, to | Edge JSON |
| `validate_diagram` | path | Validation JSON |
| `add_nodes` | path, items[] | Status JSON |
| `add_edges` | path, items[] | Status JSON |

## Data Model (flowchart IR excerpt)

```rust
struct Diagram {
    rankdir: String,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    subgraphs: Vec<Subgraph>,
    styles: Vec<NodeStyle>,
    class_defs: Vec<ClassDef>,
    class_applies: Vec<ClassApply>,
    link_styles: Vec<LinkStyle>,
}

struct Node {
    id: String,
    text: String,
    shape: NodeShape,
    href: Option<String>,
    tooltip: Option<String>,
}
```

Canonical wrapper: `Document { version, diagrams: Vec<Diagram> }` where `Diagram` is a tagged enum by `kind`.

## Analyze: `diff` / `DocumentDiff`

`diagram diff <left> <right>` and MCP `diff_diagram` load both paths via `ir::load_path` (Mermaid, JSON IR, DOT, D2, PlantUML) and return:

```json
{
  "left_diagrams": 1,
  "right_diagrams": 1,
  "diagram_count_changed": false,
  "unchanged": false,
  "summary": ["diagram 0: changed (flowchart)"],
  "entries": [
    {
      "index": 0,
      "status": "changed",
      "left_kind": "flowchart",
      "right_kind": "flowchart",
      "detail": { "added_nodes": [], "removed_nodes": [], "added_edges": [], ... }
    }
  ]
}
```

Per-kind `detail`: flowchart (`added_nodes` / `removed_nodes` / `modified_nodes` / edges / `rankdir_changed`), sequence (participants, messages), class (classes, relations), gantt (title, tasks). Entries may be `added`, `removed`, `kind_changed`, `unchanged`, or `changed`. `merge` remains flowchart-only.

## Library Interface

```rust
pub mod analyze;
pub mod class;
pub mod cli;
pub mod composite;
pub mod diagram;
pub mod formats;
pub mod gantt;
pub mod generate;
pub mod ir;
pub mod layout;
pub mod lossiness;
pub mod markdown;
pub mod mcp;
pub mod parser;
pub mod pdf;
pub mod png;
pub mod preview;
pub mod renderer;
pub mod sequence;
```

## Preview Server

`diagram preview <file> [--port 3030] [--theme dark|light]` — localhost HTTP:

| Path | Response |
|------|----------|
| `/` | HTML page polling `/svg` |
| `/svg` | Current SVG render |
| `/health` | `ok` |

## Rendering

1. Kind-specific IR → layout (flowchart: layered BFS) → SVG
2. Optional raster: SVG → pixmap (resvg) → PNG or PDF (printpdf embed)

**Flowchart SVG features:** themes, subgraphs, bezier edges, styles/classDef, linkStyle, href/tooltip.

**PDF note:** raster embed at 96 DPI (not vector PDF).

## MCP Resources & Prompts

| Template | Description |
|----------|-------------|
| `file://{path}` | Read diagram source as text |
| `create_flowchart` | Prompt: create flowchart from description |
| `refactor_diagram` | Prompt: refactor for clarity |

## Transport

MCP uses stdio (JSON-RPC). Suitable for local AI assistant integration.
