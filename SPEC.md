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

`kind` values: `flowchart`, `sequence`, `class`, `gantt`, `state`, `er`. Multi-diagram `diagrams[]` is supported for JSON IR import, composite render, per-index render (`--index`), output-dir batch render, and Mermaid export with `%% diagram N:` markers.

## Platform contracts

| Capability | Contract |
|------------|----------|
| Import | `Format` bytes → `Document` IR (`import`, `import_diagram`) |
| Export | `Document` IR → `Format` bytes (`export`, `export_diagram`) |
| Lossiness | Export fidelity report per target Format (`lossiness`, `export --report`, `lossiness_report`) |
| Render | `Document` / `Diagram` → SVG, PNG, or vector PDF |
| Analyze | `Document` → validation issues, diff, metrics JSON |
| Generate | MCP/CLI mutations and scaffolds against IR |

### Supported interchange formats

| Format | Import | Export | Notes |
|--------|--------|--------|-------|
| `mermaid` / `mmd` | ✓ | ✓ | All kinds; multi-diagram uses `%% diagram N:` markers |
| `json` / `ir` | ✓ | ✓ | Lossless native IR |
| `dot` / `gv` | ✓ flowchart | ✓ flowchart | Digraph subset |
| `d2` | ✓ flowchart | ✓ flowchart | Containers as subgraphs |
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

### Sequence diagrams (MVP + notes + fragments)

- `sequenceDiagram`, participants, `->>` / `-->>` messages, self-messages (`A->>A`)
- Notes: `Note left of X:`, `Note right of X:`, `Note over A,B:`
- Fragments: `loop` / `alt` / `else` / `opt` … `end` (nested; SVG frames; PlantUML import/export)
- Not yet: activations, parallel (`par`) fragments

### Class diagrams (MVP + stereotypes + cardinality + generics + notes)

- `classDiagram`, classes with members, relations (`<|--`, `*--`, `o--`, `-->`, `--`, `..>`, `..|>`)
- Stereotypes: `class Foo <<interface>>` or body line `<<interface>>` (SVG «…»; PlantUML `interface`/`enum`/`abstract class`)
- Cardinality: `A "1" --> "*" B` / `A --> "1..*" B` (SVG near endpoints; PlantUML roundtrip)
- Generics: Mermaid `Stack~T~` / `List~List~int~~` (SVG `‹›`; PlantUML `Stack<T>` import/export)
- Notes: `note for ClassName "text"` (SVG callout; PlantUML `note for` / `note left|right of`)

### Gantt charts (MVP + milestones)

- `gantt`, `title`, `dateFormat YYYY-MM-DD`, sections, tasks with `crit`/`active`/`done`
- Milestones: `milestone` tag with `0d` (point-in-time; SVG diamond; `after` starts on milestone day)
- Not yet: excludes, today marker, other dateFormats

### State diagrams (MVP)

- `stateDiagram-v2`, `[*]` start/end, transitions `-->` with optional `: label`
- `state "label" as id`, `state id <<choice>>` / `<<fork>>` / `<<join>>`
- Not yet: composite/nested states, notes, concurrency regions

### ER diagrams (MVP)

- `erDiagram`, entity attribute blocks (`type name` with optional `PK`/`FK`/`UK`)
- Relationships with cardinalities (`||`, `|o`, `}o`, `}|`, `o|`, `o{`, `|{`) and `--` / `..`
- Optional relationship labels (`: places`)
- Not yet: aliases, comments on attributes, crow's foot SVG glyphs beyond text cardinality

## PlantUML Compatibility (subset)

### Sequence

- `@startuml` … `@enduml`, `participant` / `actor`, `->` / `-->` messages
- Notes: one-liner `note left|right of Actor: text`, multiline `note …` / `end note`, and `note over A, B`
- Fragments: `loop` / `alt` / `else` / `opt` … `end` (nested)
- Export emits PlantUML note and fragment syntax from IR (roundtrip with Mermaid)

### Class

- `class`, `interface`, `enum`, `abstract class`, members, relations (same tokens as Mermaid class)
- Stereotypes and generics (`Stack<T>` ↔ Mermaid `Stack~T~`)
- Cardinality quotes on relations; notes (`note for X : text`, `note left|right of X` / `end note`)

### Activity

- `start` / `stop` / `end`, `:action;`, `if (cond) then (label)` / `else (label)` / `endif`
- Import and export (activity-shaped flowcharts with `start` stadium node)

## Graphviz DOT Compatibility (subset)

- `digraph` / `graph`, `rankdir`, node `[label=, shape=, fillcolor=, color=, fontcolor=, URL=/href=]`, edges `->` / `--`, chained edges, `subgraph`
- Node colors map to Mermaid-style `style` properties (`fill` / `stroke` / `color`); `URL`/`href` maps to node hyperlinks
- Import and export map to flowchart IR
- Not yet: ports, HTML labels, full Graphviz attribute surface

## D2 Compatibility (subset)

- `direction`, node `label` / `shape`, connections `->` / `<-` / `--` / `<->`, edge labels, dashed edges via `style.stroke-dash`
- Containers (`id: { members… }`) import/export as flowchart subgraphs (nested containers become sibling subgraphs)
- Import and export map to flowchart IR
- Not yet: sequence/class shapes, themes, icons, layout engine options

## Lossiness reporting

`lossiness` analyzes what IR semantics a target Format cannot represent before export:

- JSON IR: lossless
- Mermaid: warns on `href`/`tooltip`, multi-diagram marker convention
- DOT: flowchart only; warns on skipped kinds, tooltips, classDefs (`URL` and fill/stroke styles are exported)
- D2: flowchart only; warns on skipped kinds, styles/classDefs, href/tooltip (containers round-trip as subgraphs)
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
| `merge_diagram` | left, right, output | Status JSON (**flowchart-only**) |
| `add_node` | path, id, text, shape? | Status JSON (**flowchart-only**) |
| `remove_node` | path, id | Status JSON (**flowchart-only**) |
| `update_node` | path, id, text?, shape? | Status JSON (**flowchart-only**) |
| `add_edge` | path, from, to, label?, style? | Status JSON (**flowchart-only**) |
| `remove_edge` | path, from, to | Status JSON (**flowchart-only**) |
| `update_edge` | path, from, to, label?, style? | Status JSON (**flowchart-only**) |
| `get_mermaid` | path | Mermaid Compatibility source (any Format via IR) |
| `set_mermaid` | path, source | Status JSON (validates parse before write) |
| `list_nodes` | path | Node list JSON (**flowchart-only**) |
| `list_edges` | path | Edge list JSON (**flowchart-only**) |
| `get_node` | path, id | Node JSON (**flowchart-only**) |
| `get_edge` | path, from, to | Edge JSON (**flowchart-only**) |
| `validate_diagram` | path | Validation JSON (flowchart structural; other kinds parse-ok) |
| `add_nodes` | path, items[] | Status JSON (**flowchart-only**) |
| `add_edges` | path, items[] | Status JSON (**flowchart-only**) |

Import/export `from`/`to`: `mermaid`, `json`, `dot`, `d2`, `plantuml`.

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

Per-kind `detail`: flowchart (`added_nodes` / `removed_nodes` / `modified_nodes` / edges / `rankdir_changed`), sequence (participants, messages, notes, fragments), class (classes, relations, notes, stereotypes, cardinality), gantt (title, tasks incl. milestones), state/er. Entries may be `added`, `removed`, `kind_changed`, `unchanged`, or `changed`. `merge` remains flowchart-only.

## Library Interface

```rust
pub mod analyze;
pub mod class;
pub mod cli;
pub mod composite;
pub mod diagram;
pub mod er;
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
pub mod state;
```

## Preview Server

`diagram preview <file> [--port 3030] [--theme dark|light]` — localhost HTTP:

| Path | Response |
|------|----------|
| `/` | HTML page polling `/svg` |
| `/svg` | Current SVG render |
| `/health` | `ok` |

## VS Code extension

`editors/vscode/` — optional editor UX that invokes the installed `diagram` CLI:

| Command | CLI |
|---------|-----|
| Preview SVG | `diagram render <file> --theme …` → webview |
| Validate | `diagram validate <file>` |
| Render SVG to File | `diagram render <file> --output …` |

Settings: `diagram.cliPath`, `diagram.theme`, `diagram.autoPreviewOnSave`.

## Wasm embed

Feature `wasm` (build with `--no-default-features --features wasm`) exports:

| JS API | Contract |
|--------|----------|
| `render_to_svg(source, theme)` | Auto-detect Format → IR → SVG (`theme`: `dark`\|`light`) |
| `parse_to_ir_json(source)` | Auto-detect Format → Document IR JSON |

Demo: `make wasm` then serve `examples/wasm/`. Fixtures: `examples/embed/` (covered by `cargo test --test embed_tests`). Native CLI/MCP/PNG/PDF stay behind the default `native` feature.

## Rendering

1. Kind-specific IR → layout (flowchart: layered BFS) → SVG
2. Optional PNG: SVG → pixmap (resvg) — native feature
3. Optional PDF: SVG → usvg tree → vector PDF (svg2pdf) — native feature
4. Optional Wasm: same SVG path in the browser via `embed` / `wasm`

**Flowchart SVG features:** themes, subgraphs, bezier edges, styles/classDef, linkStyle, href/tooltip.

**PDF note:** vector paths/content streams via svg2pdf (not a raster embed).

## MCP Resources & Prompts

| Template | Description |
|----------|-------------|
| `file://{path}` | Read diagram source as text |
| `create_flowchart` | Prompt: create flowchart from description |
| `refactor_diagram` | Prompt: refactor for clarity |

## Transport

MCP uses stdio (JSON-RPC). Suitable for local AI assistant integration.
