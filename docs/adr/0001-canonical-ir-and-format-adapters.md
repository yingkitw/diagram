# Canonical IR with format adapters

Status: accepted

We reposition `diagram` as a **diagram platform** (render, generate, analyze, interchange), not a Mermaid-only tool. Mermaid remains a first-class **compatibility Format**, not the identity of the product.

**Decision:** All diagram kinds converge on a canonical **IR** (in-memory + JSON). External syntaxes (Mermaid, PlantUML, DOT, D2, …) and visual outputs (SVG, PNG, PDF) are **Adapters** around that IR. New work prefers IR-centric APIs; format parsers become adapters rather than the source of truth.

**Why not stay Mermaid-native?** Competing with Mermaid/PlantUML on syntax alone loses; competing on native speed, MCP-native generation/analysis, and multi-format interchange wins. Cloning either ecosystem end-to-end is unbounded; Compatibility with lossy-but-useful roundtrips is the bar.

**Consequences (as of v0.1.x):**

- **Shipped:** `Document` / `Diagram` / `Kind` JSON IR; `import`/`export` CLI/MCP; lossiness reports; Mermaid behind `formats::mermaid`; DOT and PlantUML adapters (partial); render to SVG/PNG/vector PDF; multi-diagram documents; markdown pipeline; analysis metrics.
- **Ongoing:** Expand adapter subsets (PlantUML / DOT / D2); sequence extras; optional formats (Excalidraw) and Wasm as demand warrants. VS Code extension ships as a thin CLI client under `editors/vscode/`.
- **Rule:** New features that cross format boundaries go through IR + adapters, not Mermaid-only code paths.
