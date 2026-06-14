# Architecture

## Module Dependencies

```
main.rs
  ├── cli.rs      — clap CLI dispatch → diagram ops or MCP server
  ├── mcp.rs      — rmcp ServerHandler → 12 tools via modify_file()
  ├── diagram.rs  — Core data model (Diagram, Node, Edge, Subgraph, styles, classDefs)
  ├── parser.rs   — Mermaid text → Diagram (line-based, arrow splitting)
  ├── layout.rs   — Diagram → Layout (positioned nodes + routed edges)
  └── renderer.rs — Layout → SVG string (dark theme, 6 shapes, 3 edge styles)
```

## Data Flow

```
.mmd file → parser::parse() → Diagram
                                    ↓
Diagram → layout::layout() → Layout
                                    ↓
Layout → renderer::render_svg() → SVG String

Diagram ↔ CLI commands (read → mutate → write)
Diagram ↔ MCP tools (read → mutate → write via modify_file)
```

## Key Design Decisions

### File-based state

All MCP tools and mutating CLI commands read the `.mmd` file, perform the operation, and write it back. This keeps the server stateless and ensures changes persist between sessions. Each tool call is self-contained.

### Minimal rmcp pattern

The MCP server uses the minimal struct pattern (no `ToolRouter` field needed) with `#[tool_router]` generating routes automatically. `#[tool_handler]` with a custom `get_info()` provides server metadata and capabilities.

### `modify_file` helper

`mcp.rs` centralizes read-modify-write in a `modify_file(path, f)` helper. This guarantees every mutating tool follows the same pattern: parse → apply → serialize → write. Error handling uses `CallToolResult::error` for consistent MCP responses.

### Layered layout

The layout algorithm assigns layers using BFS from source nodes, then positions nodes within each layer. It supports all four directions (`TB`, `LR`, `RL`, `BT`) by swapping x/y axes and reversing layer order. This avoids the dependency on dagre.js while producing reasonable diagrams for common cases.

### Parsing approach

The parser processes lines by splitting on arrow delimiters (`-->`, `->`, `-.->`, `==>`), then extracts node definitions and labels. Shape detection uses bracket matching (`[]`, `{}`, `()`, `{{}}`, `(())`, `[()]`). Quoted IDs (`"my node"`) are handled via quote-aware tokenization.

### Parsed-but-not-rendered features

Subgraphs, `style` directives, and `classDef`/`class` are fully parsed, stored in the `Diagram` model, and roundtripped via `to_mermaid()`. They are **not yet rendered** in SVG output.
