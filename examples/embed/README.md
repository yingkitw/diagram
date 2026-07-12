# Embed / Wasm examples

Compact fixtures for the string-in API (`diagram::embed` / Wasm `render_to_svg`).

| File | Format / kind |
|------|----------------|
| `flowchart.mmd` | Mermaid flowchart |
| `sequence.mmd` | Mermaid sequence |
| `class.mmd` | Mermaid class (stereotype + cardinality) |
| `state.mmd` | Mermaid state |
| `er.mmd` | Mermaid ER |
| `gantt.mmd` | Mermaid gantt + milestone |
| `flow.dot` | Graphviz DOT |
| `flow.d2` | D2 + container |
| `sequence.puml` | PlantUML sequence |
| `sample.ir.json` | Canonical JSON IR |

```bash
cargo test --test embed_tests
diagram render examples/embed/flowchart.mmd
# After `make wasm`, open examples/wasm/ and paste any of these sources.
```
