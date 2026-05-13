# TODO

## Short-term

- [x] Core data model (Node, Edge, Diagram)
- [x] Mermaid parser (flowchart syntax)
- [x] Graph layout algorithm (layered)
- [x] SVG renderer
- [x] CLI subcommands (parse, info, render, add/remove/update node/edge)
- [x] MCP server with rmcp (10 tools)
- [x] Roundtrip: parse → manipulate → to_mermaid → parse
- [ ] Support more Mermaid graph types (subgraph, style, classDef)
- [ ] Support more node shapes (hexagon, cylinder, circle)
- [ ] Support more edge types (dotted, thick)
- [ ] `set_mermaid` MCP tool (write raw mermaid source)

## Medium-term

- [ ] MCP resource support (diagram files as resources)
- [ ] MCP prompts for common diagram operations
- [ ] Watch mode: auto-reload on file change
- [ ] Better error messages with file:line info
- [ ] Integration tests with sample diagrams

## Longer-term

- [ ] Web UI (wasm-based or lightweight server)
- [ ] Support for sequence diagrams
- [ ] Support for class diagrams
- [ ] Support for Gantt charts
- [ ] VSCode extension via MCP
- [ ] Plugin system for custom shape renderers
