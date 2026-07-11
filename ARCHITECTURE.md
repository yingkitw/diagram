# Architecture

## Module Dependencies

```
main.rs
  ├── cli.rs      — clap CLI dispatch → diagram ops, preview server, or MCP server
  ├── mcp.rs      — rmcp ServerHandler → tools/resources/prompts via modify_file()
  ├── preview.rs  — Lightweight HTTP preview (HTML shell + /svg live reload)
  ├── diagram.rs  — Core data model (Diagram, Node, Edge, Subgraph, styles, classDefs)
  ├── parser.rs   — Mermaid flowchart text → Diagram
  ├── sequence.rs — Mermaid sequence text → SequenceDiagram → SVG
  ├── layout.rs   — Diagram → Layout (positioned nodes + routed edges)
  └── renderer.rs — Layout → SVG string (themes, shapes, styles, interactivity)
```

## Data Flow

```
.mmd file → detect type
              ├── flowchart → parser::parse() → layout → SVG
              └── sequence  → sequence::parse() → sequence::render_svg()

Diagram ↔ CLI commands (read → mutate → write; flowchart mutating ops)
Diagram ↔ MCP tools (read → mutate → write via modify_file)
.mmd file → preview server → browser (HTML + polled SVG)
```

## Key Design Decisions

### File-based state

All MCP tools and mutating CLI commands read the `.mmd` file, perform the operation, and write it back. This keeps the server stateless and ensures changes persist between sessions. Each tool call is self-contained.

### Sequence diagrams

Sequence support lives in `sequence.rs` as a parallel pipeline (not forced into the flowchart `Diagram` model). Auto-detection routes `parse` / `info` / `render` / `preview` / MCP equivalents. Mutating tools remain flowchart-only for now.

### Minimal rmcp pattern

The MCP server uses the minimal struct pattern (no `ToolRouter` field needed) with `#[tool_router]` generating routes automatically. `#[tool_handler]` with a custom `get_info()` provides server metadata and capabilities.

### `modify_file` helper

`mcp.rs` centralizes read-modify-write in a `modify_file(path, f)` helper. This guarantees every mutating tool follows the same pattern: parse → apply → serialize → write. Error handling uses `CallToolResult::error` for consistent MCP responses.

### Layered layout

The layout algorithm assigns layers using BFS from source nodes, then positions nodes within each layer. It supports all four directions (`TB`, `LR`, `RL`, `BT`) by swapping x/y axes and reversing layer order. This avoids the dependency on dagre.js while producing reasonable diagrams for common cases.

### Parsing approach

The parser processes lines by splitting on arrow delimiters (`-->`, `->`, `-.->`, `==>`), then extracts node definitions and labels. Shape detection uses bracket matching (`[]`, `{}`, `()`, `{{}}`, `(())`, `[()]`). Quoted IDs (`"my node"`) are handled via quote-aware tokenization.

### Preview server

`diagram preview` binds a tiny HTTP server on localhost (no extra HTTP crates). `GET /` serves an HTML shell that polls `GET /svg` every second so edits to the `.mmd` file appear live. Theme is fixed at server start via `--theme`.

### Rendered features

Subgraphs, `style` / `classDef` / `class`, and `linkStyle` are parsed, stored, roundtripped, and applied in SVG. Nodes may include `href` links and tooltips. Light and dark themes are supported.
