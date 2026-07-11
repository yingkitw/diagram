# Canonical IR with format adapters

Status: accepted

We reposition `diagram` as a **diagram platform** (render, generate, analyze, interchange), not a Mermaid-only tool. Mermaid remains a first-class **compatibility Format**, not the identity of the product.

**Decision:** All diagram kinds converge on a canonical **IR** (in-memory + JSON). External syntaxes (Mermaid, PlantUML, DOT, D2, …) and visual outputs (SVG, PNG, PDF) are **Adapters** around that IR. New work prefers IR-centric APIs; format parsers become adapters rather than the source of truth.

**Why not stay Mermaid-native?** Competing with Mermaid/PlantUML on syntax alone loses; competing on native speed, MCP-native generation/analysis, and multi-format interchange wins. Cloning either ecosystem end-to-end is unbounded; Compatibility with lossy-but-useful roundtrips is the bar.

**Consequences:** Short-term code still has parallel Mermaid modules (`parser`, `sequence`, `class`, `gantt`); the next architectural step is a `Document`/`Diagram` IR enum plus `import`/`export` CLI/MCP commands. PlantUML and other formats land as adapters, not as a second core model.
