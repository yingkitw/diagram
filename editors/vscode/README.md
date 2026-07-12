# diagram VS Code extension

Minimal editor UX for the native [`diagram`](../../README.md) CLI: SVG preview, validate, and render-to-file — no Chromium or Mermaid.js.

## Requirements

1. Install the CLI: `cargo install --path .` (from the repo root), or set **diagram.cliPath** to your binary.
2. Open a diagram file (`.mmd`, `.puml`, `.dot`, `.d2`, …).

## Install (development)

```bash
# From this folder — opens VS Code / Cursor with the extension loaded
code --extensionDevelopmentPath="$(pwd)"
# or
cursor --extensionDevelopmentPath="$(pwd)"
```

Or copy/symlink this folder into your extensions directory and reload the window.

## Commands

| Command | Action |
|---------|--------|
| **Diagram: Preview SVG** | Render active file with `diagram render` into a side webview |
| **Diagram: Validate** | Run `diagram validate` and show output |
| **Diagram: Render SVG to File** | Save SVG via `diagram render --output` |

Preview refreshes on save when **diagram.autoPreviewOnSave** is enabled (default).

## Settings

| Setting | Default | Meaning |
|---------|---------|---------|
| `diagram.cliPath` | `diagram` | CLI binary on PATH or absolute path |
| `diagram.theme` | `dark` | `dark` or `light` |
| `diagram.autoPreviewOnSave` | `true` | Refresh open preview on save |

## Notes

- This extension shells out to the Rust binary; it does not embed a layout engine.
- Untitled buffers must be saved before preview.
- Language IDs for `.mmd` / `.puml` / `.dot` / `.d2` are registered for associations; full syntax highlighting can be added later.
