# TODO

## Vision

Platform pillars (see `CONTEXT.md`, `ARCHITECTURE.md`, ADR-0001):

1. **Render** — layout → SVG/PNG/PDF, Chromium-free
2. **Generate** — structured create/edit (CLI + MCP)
3. **Analyze** — validate, diff, merge, metrics on IR
4. **Interchange** — import/export via canonical IR; Mermaid/PlantUML Compatibility

Competitive edge vs Mermaid.js / PlantUML: native binary, MCP-first agents, analysis without browser/JVM, multi-format IR — not “another Mermaid clone.”

## Completed (foundation)

- [x] Flowchart IR + Mermaid parse/render (shapes, edges, subgraphs, styles, themes)
- [x] Sequence / class / gantt Mermaid MVP parse → SVG
- [x] CLI + MCP generate/edit (flowchart), validate, diff/merge
- [x] Live preview, interactive SVG (href/tooltip), watch mode
- [x] Product reposition: CONTEXT, ADR-0001, architecture target (IR + adapters)
- [x] Canonical IR spine (`ir`, `formats`) + import/export CLI/MCP

## Short-term (platform spine)

- [x] Canonical **IR**: `Document` / `Diagram` / `Kind` + JSON (`diagram ir`, `parse`, `import`/`export`)
- [x] **Format detection** + `import` / `export` CLI + MCP tools
- [x] Analysis pack v1: metrics (node/edge counts, depth, cycle list, orphan rate) as JSON
- [x] Fold Mermaid parsers behind `formats::mermaid` (behavior-preserving move)
- [x] Generation: kind-aware `create` scaffold (`diagram create --kind flowchart|sequence|class|gantt`)

## Medium-term (interchange + render)

- [x] PlantUML adapter (sequence, class, activity import; sequence/class export)
- [x] Graphviz DOT import + export (digraph subset) ↔ flowchart IR
- [x] PNG export (resvg; `.png` output via `diagram render`)
- [x] PDF export (resvg + printpdf; `.pdf` output via `diagram render`)
- [x] Multi-diagram Document (several kinds / figures per file)
- [x] Markdown pipeline: extract fenced blocks → render → rewrite links
- [x] Lossiness report on export (what could not be represented)

## Longer-term

- [ ] D2 / Excalidraw / Kroki-adjacent adapters as demand warrants
- [ ] PlantUML activity export (flowchart IR → activity syntax)
- [ ] Vector PDF export (optional; raster PDF shipped)
- [ ] State + ER kinds (IR + Mermaid Compatibility)
- [ ] Sequence/class/gantt extras (notes, loops, interfaces, milestones, …)
- [ ] VS Code extension (thin client over MCP + preview)
- [ ] Plugin API for custom shapes / render backends
- [ ] Wasm embed for browser preview without local server
- [ ] Semantic diff UX (structural, not only text)

## Brainstorming (competitive)

| Advantage we want | vs Mermaid | vs PlantUML |
|-------------------|------------|-------------|
| Native speed / small install | No Node/Chromium | No JVM |
| MCP agent generate+analyze | Weak / external | Weak / external |
| Multi-format IR hub | Mermaid-centric | PlantUML-centric |
| Structural analysis API | Limited | Limited |
| Docs CI without heavy runtimes | mmdc heavy | Docker/Java common |

Prioritize next: D2 or editor UX (VS Code) → semantic diff → new kinds (state/ER) → adapter depth (DOT/PlantUML subsets).
