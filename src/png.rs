//! SVG → PNG rasterization (Chromium-free).

/// Parse SVG and rasterize to a pixmap.
pub fn svg_to_pixmap(svg: &str) -> Result<tiny_skia::Pixmap, String> {
    let mut opt = usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();

    let tree = usvg::Tree::from_str(svg, &opt).map_err(|e| format!("SVG parse failed: {e}"))?;
    let size = tree.size().to_int_size();
    if size.width() == 0 || size.height() == 0 {
        return Err("SVG has zero width or height".into());
    }

    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height())
        .ok_or_else(|| format!("invalid PNG dimensions: {size:?}"))?;

    resvg::render(
        &tree,
        tiny_skia::Transform::default(),
        &mut pixmap.as_mut(),
    );

    Ok(pixmap)
}

/// Rasterize SVG text to PNG bytes.
pub fn svg_to_png(svg: &str) -> Result<Vec<u8>, String> {
    let pixmap = svg_to_pixmap(svg)?;
    pixmap.encode_png().map_err(|e| e.to_string())
}

/// True when a render output path should produce PNG.
pub fn output_is_png(path: &str) -> bool {
    path.to_lowercase().ends_with(".png")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir;
    use crate::renderer::Theme;

    #[test]
    fn flowchart_svg_to_png() {
        let doc = ir::from_mermaid("graph TD\n  A[Start] --> B[End]\n").unwrap();
        let svg = doc.render_svg(Theme::Dark).unwrap();
        let png = svg_to_png(&svg).expect("png render");
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(png.len() > 100);
    }

    #[test]
    fn sequence_svg_to_png() {
        let doc = ir::from_mermaid("sequenceDiagram\n  A->>B: hi\n").unwrap();
        let svg = doc.render_svg(Theme::Dark).unwrap();
        let png = svg_to_png(&svg).expect("png render");
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    }
}
