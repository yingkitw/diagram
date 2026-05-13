# TODO

## Short-term

- [x] Core data model (Node, Edge, Diagram)
- [x] Mermaid parser (flowchart syntax)
- [x] Graph layout algorithm (layered)
- [x] SVG renderer
- [x] CLI subcommands (parse, info, render, add/remove/update node/edge)
- [x] MCP server with rmcp (12 tools)
- [x] Roundtrip: parse → manipulate → to_mermaid → parse
- [x] Node shapes: hexagon (`{{}}`), cylinder (`[()]`), circle (`(())`)
- [x] Edge types: dashed (`-.->`), thick (`==>`)
- [x] `set_mermaid` MCP tool (write raw mermaid source)
- [ ] Subgraph support (`subgraph ... end`)
- [ ] Styling: `style` and `classDef` directives
- [ ] CLI: add `get-mermaid`, `set-mermaid`, `list-nodes`, `list-edges` subcommands
- [ ] CLI `add-edge`: add `--style` parameter (arrow/dashed/thick)
- [ ] CLI `update-node --shape` help: update to list all 6 shapes
- [ ] Quoted node IDs with special characters

## Medium-term

- [ ] MCP resource support (diagram files as resources)
- [ ] MCP prompts for common diagram operations
- [ ] Watch mode: auto-reload on file change
- [ ] Better error messages with file:line info
- [ ] Integration tests with sample diagrams
- [ ] CLI unit tests
- [ ] CI setup (GitHub Actions: test + build)
- [ ] Populate `examples/` directory with sample `.mmd` files
- [ ] Tagged releases + `cargo publish`

## Longer-term

- [ ] Web UI (wasm-based or lightweight server)
- [ ] Support for sequence diagrams
- [ ] Support for class diagrams
- [ ] Support for Gantt charts
- [ ] VSCode extension via MCP
- [ ] Plugin system for custom shape renderers
- [ ] Interactive SVG output (links, tooltips, click events)
- [ ] Multi-diagram file support (multiple graphs per file)
