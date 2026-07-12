.PHONY: run dev parse info render mcp test vscode-check

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
