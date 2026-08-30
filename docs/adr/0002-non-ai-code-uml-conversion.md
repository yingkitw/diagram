# Non-AI code ↔ UML conversion as the core mission

Status: accepted

We state, explicitly: `diagram`'s purpose is **non-AI code ↔ UML conversion**. Every conversion path is local, deterministic, and free of model/LLM tokens. AI helpers are not in the product surface; tree-sitter grammars and IR walks are.

**Decision:** `diagram` is positioned as a native, non-AI code ↔ UML platform.

- **Code → UML** runs through tree-sitter grammars + the canonical IR; no model call, no API key, no network.
- **UML → Code** runs through a skeleton generator that preserves signatures from the IR and emits empty bodies; again no model call.
- **Format interchange** (Mermaid / PlantUML / DOT / D2 / JSON IR) is a compatibility surface around the same IR — and equally non-AI.
- The CLI and MCP tools expose these paths directly so agents call them instead of round-tripping diagram work through an LLM.

**Why non-AI?** Because AI-based code→diagram is the wrong tool for this job:

| Property | AI helper | `diagram` |
|---|---|---|
| Latency | model call (seconds) | tree-sitter (microseconds) |
| Cost | tokens per call | zero |
| Determinism | varies (temperature, prompt, model version) | byte-identical, always |
| Privacy | source leaves the box | source stays local |
| Offline | no | yes |

The non-AI path wins on every axis that matters for documents, CI, and agent loops. That is the project's edge, and we will not trade it for "smarter" output.

**Consequences:**

- New conversion paths must be implementable by grammar + IR walk alone. If a feature can only be done by a model, it does not belong on the platform.
- New languages and diagram kinds are added at the boundary (per-language extractors, skeleton generators). The IR and `Document`/`Diagram`/`Kind` types remain the spine.
- The CLI/MCP surface stays the agent-facing API. When an MCP tool can answer in microseconds, the agent should prefer it over asking an LLM.
- "Skeleton" is intentionally shallow: signatures from the IR, empty bodies. The point is roundtrip stability and a starting point for humans — not guessing intent.
- Docs and ADRs (this one included) repeat the non-AI framing so the principle is not quietly eroded by a future feature.

**Out of scope:** semantic refactoring, "explain this diagram", "summarize this codebase" — those are LLM-shaped problems and are intentionally not shipped here.