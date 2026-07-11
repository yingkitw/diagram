# TODO

## Completed

- [x] Core data model (`Node`, `Edge`, `Diagram`, `Subgraph`, styles, classDefs)
- [x] Mermaid flowchart parser (`graph TD/LR/RL/BT`)
- [x] Layered graph layout algorithm (BFS-based, supports all 4 directions)
- [x] SVG renderer with dark theme and 6 node shapes
- [x] CLI: `parse`, `info`, `render`, `mcp`, `add-node`, `remove-node`, `update-node`, `add-edge`, `remove-edge`, `get-mermaid`, `set-mermaid`, `list-nodes`, `list-edges`
- [x] MCP server with 12 tools (`parse_diagram`, `get_info`, `render_svg`, `add_node`, `remove_node`, `update_node`, `add_edge`, `remove_edge`, `get_mermaid`, `set_mermaid`, `list_nodes`, `list_edges`)
- [x] Roundtrip fidelity: parse → manipulate → `to_mermaid()` → parse
- [x] Node shapes: rect, diamond, stadium, hexagon, cylinder, circle
- [x] Edge styles: arrow (`-->`), dashed (`-.->`), thick (`==>`)
- [x] Subgraph support (`subgraph ... end`)
- [x] Styling directives: `style`, `classDef`, `class`
- [x] Quoted node IDs with special characters
- [x] Integration tests for all `examples/` roundtrip + render
- [x] CI: GitHub Actions (test + build)

## Short-term

- [x] `update-edge` CLI + MCP tool (change label or style on existing edge)
- [x] `get-node` / `get-edge` CLI + MCP tools (retrieve single item)
- [x] `validate` CLI command + `validate_diagram` MCP tool (orphaned nodes, dangling edges, cycles)
- [x] Batch `add-nodes` / `add-edges` MCP tools (reduce round-trips)
- [x] CLI unit tests (subprocess integration)
- [x] MCP resource support (`file://{path}` template, read `.mmd` files)
- [x] MCP prompts (`create_flowchart`, `refactor_diagram`)

## Medium-term

- [x] Subgraph visual rendering in SVG (bounding boxes with dashed borders and labels)
- [x] Node styling (`style` + `classDef`/`class`) applied to SVG fill/stroke
- [x] Edge styling (`linkStyle`) applied to SVG output (stroke color + width)
- [x] Watch mode: `diagram render --watch` auto-re-renders SVG on file change
- [x] Edge routing improvements (reduce crossings, curved beziers)
- [x] Theme support: light/dark toggle in renderer
- [x] Tagged releases + `cargo publish` (dry-run passes, crate ready)
- [x] Diagram diff / merge utilities

## Longer-term

- [x] Web UI — live browser preview (`diagram preview`)
- [x] Interactive SVG output (links, tooltips, click events)
- [x] Support for sequence diagrams (MVP: participants, `->>` / `-->>`, SVG)
- [ ] Support for class diagrams
- [ ] Support for Gantt charts
- [ ] VSCode extension via MCP
- [ ] Plugin system for custom shape renderers
- [ ] Multi-diagram file support (multiple graphs per `.mmd` file)
- [ ] PNG/PDF export (parity with mermaid-cli / Kroki)
- [ ] Markdown in-place render (find ```mermaid blocks, write SVGs)
- [ ] State / ER diagram types (common Mermaid coverage gap vs mermaid.js)
- [ ] Sequence diagram extras (notes, loops, alt/opt, activations)

## Brainstorming (competitive)

Gaps vs mermaid-cli, Kroki, and mermaid.js that would strengthen this crate:

- Broader diagram-type coverage (sequence, class, state, ER, Gantt) — largest parity gap
- Multi-diagram / markdown pipeline support for docs CI
- Raster export without Chromium (keep the native-binary advantage)
- VS Code / editor UX beyond raw MCP stdio
- Optional wasm embed for browser preview without a local server

