# Architecture

## Module Dependencies

```
main.rs
  ├── cli.rs      — clap CLI dispatch → diagram ops or MCP server
  ├── mcp.rs      — rmcp ServerHandler → tools call diagram/parser/renderer
  ├── diagram.rs  — Core data model (Diagram, Node, Edge, NodeShape)
  ├── parser.rs   — Mermaid text → Diagram
  ├── layout.rs   — Diagram → Layout (positioned nodes + routed edges)
  └── renderer.rs — Layout → SVG string
```

## Data Flow

```
.mmd file → parser::parse() → Diagram
                                    ↓
Diagram → layout::layout() → Layout
                                    ↓
Layout → renderer::render_svg() → SVG String

Diagram ↔ CLI commands (add/remove/update node/edge)
Diagram ↔ MCP tools (same operations via rmcp)
```

## Key Design Decisions

### File-based state

All MCP tools read the `.mmd` file, perform the operation, and write it back. This keeps the server stateless and ensures changes persist between sessions. Each tool call is self-contained.

### Minimal rmcp pattern

The MCP server uses the minimal struct pattern (no `ToolRouter` field needed) with `#[tool_router]` generating routes automatically. `#[tool_handler]` with a custom `get_info()` provides server metadata and capabilities.

### Layered layout

The layout algorithm assigns layers using BFS from source nodes, then positions nodes within each layer. This avoids the dependency on dagre.js while producing reasonable diagrams for common cases.

### Parsing approach

The parser processes lines by splitting on arrow delimiters (`-->`, `->`), then extracts node definitions and labels. Shape detection uses bracket matching (`[]`, `{}`, `()`).
