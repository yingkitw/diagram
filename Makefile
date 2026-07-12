.PHONY: run dev parse info render mcp test vscode-check wasm wasm-check

# Run the MCP server
run:
	cargo run -- mcp

# Parse a diagram
parse:
	cargo run -- parse sample.mmd

# Show diagram info
info:
	cargo run -- info sample.mmd

# Render as SVG
render:
	cargo run -- render sample.mmd

# Start MCP server
mcp:
	cargo run -- mcp

# Dev mode: auto-restart on code changes
dev:
	cargo watch -x 'run -- mcp'

# Full Rust tests
test:
	cargo test

# Validate VS Code extension package
vscode-check:
	node editors/vscode/check.js

# Build browser Wasm package into examples/wasm/pkg (requires wasm-pack + wasm32 target)
wasm:
	wasm-pack build --target web --out-dir examples/wasm/pkg -- --no-default-features --features wasm

# Typecheck Wasm feature without packing
wasm-check:
	cargo check --no-default-features --features wasm --target wasm32-unknown-unknown
