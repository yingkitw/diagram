//! Lightweight HTTP preview server for live SVG viewing.

use crate::renderer::Theme;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Render a diagram file to SVG (Mermaid or JSON IR).
pub fn render_file(path: &str, theme: Theme) -> Result<String, String> {
    let doc = crate::ir::load_path(path).map_err(|e| e.to_string())?;
    doc.render_svg(theme)
}

/// Write each diagram in a document to separate files in a directory.
pub fn write_render_outputs_to_dir(
    diagram_path: &str,
    output_dir: &std::path::Path,
    theme: Theme,
    png: bool,
) -> Result<Vec<std::path::PathBuf>, String> {
    let doc = crate::ir::load_path(diagram_path).map_err(|e| e.to_string())?;
    if doc.diagrams.is_empty() {
        return Err("document has no diagrams".into());
    }
    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create '{}': {e}", output_dir.display()))?;
    let stem = std::path::Path::new(diagram_path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "diagram".into());
    let ext = if png { "png" } else { "svg" };
    let mut paths = Vec::new();
    for i in 0..doc.diagrams.len() {
        let svg = doc.render_diagram_at(i, theme)?;
        let filename = format!("{stem}-{i}.{ext}");
        let path = output_dir.join(&filename);
        if png {
            let bytes = crate::png::svg_to_png(&svg)?;
            std::fs::write(&path, bytes)
                .map_err(|e| format!("Failed to write '{}': {e}", path.display()))?;
        } else {
            std::fs::write(&path, svg)
                .map_err(|e| format!("Failed to write '{}': {e}", path.display()))?;
        }
        paths.push(path);
    }
    Ok(paths)
}

/// Write rendered output to a path (`.png` → PNG, otherwise SVG).
pub fn write_render_output(path: &str, diagram_path: &str, theme: Theme) -> Result<(), String> {
    let svg = render_file(diagram_path, theme)?;
    if crate::png::output_is_png(path) {
        let png = crate::png::svg_to_png(&svg)?;
        std::fs::write(path, png).map_err(|e| format!("Failed to write '{path}': {e}"))?;
    } else {
        std::fs::write(path, svg).map_err(|e| format!("Failed to write '{path}': {e}"))?;
    }
    Ok(())
}

/// HTML shell that polls `/svg` for live updates.
pub fn preview_html(file_name: &str, theme: Theme) -> String {
    let bg = match theme {
        Theme::Dark => "#1a1a2e",
        Theme::Light => "#ffffff",
    };
    let fg = match theme {
        Theme::Dark => "#94a3b8",
        Theme::Light => "#64748b",
    };
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>diagram — {file_name}</title>
  <style>
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      min-height: 100vh;
      background: {bg};
      color: {fg};
      font-family: ui-sans-serif, system-ui, sans-serif;
      display: flex;
      flex-direction: column;
    }}
    header {{
      padding: 0.75rem 1.25rem;
      border-bottom: 1px solid {fg}33;
      font-size: 0.875rem;
      display: flex;
      justify-content: space-between;
      align-items: center;
    }}
    #stage {{
      flex: 1;
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 1.5rem;
      overflow: auto;
    }}
    #stage svg {{ max-width: 100%; height: auto; }}
    #error {{ color: #f87171; white-space: pre-wrap; max-width: 40rem; }}
  </style>
</head>
<body>
  <header>
    <span>{file_name}</span>
    <span id="status">loading…</span>
  </header>
  <div id="stage"><div id="error"></div></div>
  <script>
    const stage = document.getElementById('stage');
    const status = document.getElementById('status');
    async function refresh() {{
      try {{
        const r = await fetch('/svg');
        const text = await r.text();
        if (!r.ok) {{
          stage.innerHTML = '<div id="error"></div>';
          document.getElementById('error').textContent = text;
          status.textContent = 'error';
          return;
        }}
        stage.innerHTML = text;
        status.textContent = 'live';
      }} catch (e) {{
        status.textContent = 'disconnected';
      }}
    }}
    refresh();
    setInterval(refresh, 1000);
  </script>
</body>
</html>
"#
    )
}

fn http_response(status: &str, content_type: &str, body: &str) -> Vec<u8> {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n",
        body.len()
    );
    let mut out = header.into_bytes();
    out.extend_from_slice(body.as_bytes());
    out
}

/// Handle a single HTTP request path, returning status / content-type / body.
pub fn route(request_path: &str, file_path: &str, theme: Theme) -> (&'static str, &'static str, String) {
    let path = request_path.split('?').next().unwrap_or("/");
    match path {
        "/svg" => match render_file(file_path, theme) {
            Ok(svg) => ("200 OK", "image/svg+xml; charset=utf-8", svg),
            Err(e) => ("500 Internal Server Error", "text/plain; charset=utf-8", e),
        },
        "/health" => ("200 OK", "text/plain; charset=utf-8", "ok".to_string()),
        "/" | "/index.html" => {
            let name = std::path::Path::new(file_path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(file_path);
            (
                "200 OK",
                "text/html; charset=utf-8",
                preview_html(name, theme),
            )
        }
        _ => (
            "404 Not Found",
            "text/plain; charset=utf-8",
            "not found".to_string(),
        ),
    }
}

fn parse_request_target(buf: &[u8]) -> String {
    let text = String::from_utf8_lossy(buf);
    let first = text.lines().next().unwrap_or("");
    first
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .to_string()
}

/// Serve a live preview of `file_path` on `127.0.0.1:port`.
pub async fn serve(file_path: String, port: u16, theme: Theme) -> anyhow::Result<()> {
    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr).await?;
    eprintln!("Preview: http://{addr}/  (Ctrl+C to stop)");
    serve_with_listener(listener, file_path, theme).await
}

/// Serve using an already-bound listener (useful for tests with port 0).
pub async fn serve_with_listener(
    listener: TcpListener,
    file_path: String,
    theme: Theme,
) -> anyhow::Result<()> {
    loop {
        let (mut socket, _) = listener.accept().await?;
        let file_path = file_path.clone();
        tokio::spawn(async move {
            let mut buf = [0u8; 2048];
            let n = match socket.read(&mut buf).await {
                Ok(0) | Err(_) => return,
                Ok(n) => n,
            };
            let target = parse_request_target(&buf[..n]);
            let (status, ctype, body) = route(&target, &file_path, theme);
            let resp = http_response(status, ctype, &body);
            let _ = socket.write_all(&resp).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_html_includes_poll_script() {
        let html = preview_html("demo.mmd", Theme::Dark);
        assert!(html.contains("fetch('/svg')"));
        assert!(html.contains("demo.mmd"));
        assert!(html.contains("#1a1a2e"));
    }

    #[test]
    fn route_health() {
        let (status, ctype, body) = route("/health", "x.mmd", Theme::Dark);
        assert_eq!(status, "200 OK");
        assert!(ctype.contains("text/plain"));
        assert_eq!(body, "ok");
    }

    #[test]
    fn route_404() {
        let (status, _, body) = route("/nope", "x.mmd", Theme::Dark);
        assert_eq!(status, "404 Not Found");
        assert_eq!(body, "not found");
    }

    #[test]
    fn route_svg_missing_file() {
        let (status, _, body) = route("/svg", "/nonexistent/path.mmd", Theme::Dark);
        assert_eq!(status, "500 Internal Server Error");
        assert!(body.contains("Failed to read"));
    }
}
