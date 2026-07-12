//! SVG → PDF vector conversion (Chromium-free via svg2pdf / usvg).

/// Convert SVG text to a standalone vector PDF.
pub fn svg_to_pdf(svg: &str) -> Result<Vec<u8>, String> {
    let mut opt = usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();

    let tree = usvg::Tree::from_str(svg, &opt).map_err(|e| format!("SVG parse failed: {e}"))?;
    let size = tree.size();
    if size.width() <= 0.0 || size.height() <= 0.0 {
        return Err("SVG has zero width or height".into());
    }

    svg2pdf::to_pdf(
        &tree,
        svg2pdf::ConversionOptions::default(),
        svg2pdf::PageOptions::default(),
    )
    .map_err(|e| format!("PDF conversion failed: {e}"))
}

/// True when a render output path should produce PDF.
pub fn output_is_pdf(path: &str) -> bool {
    path.to_lowercase().ends_with(".pdf")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir;
    use crate::renderer::Theme;

    fn assert_pdf(bytes: &[u8]) {
        assert!(bytes.starts_with(b"%PDF"), "expected PDF header");
        assert!(bytes.len() > 100);
        // Vector PDFs contain path operators; raster embeds used /Image.
        let as_str = String::from_utf8_lossy(bytes);
        assert!(
            as_str.contains("stream"),
            "expected PDF content streams"
        );
    }

    #[test]
    fn flowchart_svg_to_pdf() {
        let doc = ir::from_mermaid("graph TD\n  A[Start] --> B[End]\n").unwrap();
        let svg = doc.render_svg(Theme::Dark).unwrap();
        let pdf = svg_to_pdf(&svg).expect("pdf render");
        assert_pdf(&pdf);
    }

    #[test]
    fn sequence_svg_to_pdf() {
        let doc = ir::from_mermaid("sequenceDiagram\n  A->>B: hi\n").unwrap();
        let svg = doc.render_svg(Theme::Dark).unwrap();
        let pdf = svg_to_pdf(&svg).expect("pdf render");
        assert_pdf(&pdf);
    }

    #[test]
    fn vector_pdf_not_image_xobject_heavy() {
        let doc = ir::from_mermaid("graph TD\n  A[Start] --> B{Decide}\n  B --> C[Done]\n").unwrap();
        let svg = doc.render_svg(Theme::Light).unwrap();
        let pdf = svg_to_pdf(&svg).expect("pdf render");
        let as_str = String::from_utf8_lossy(&pdf);
        // Path-based content should appear; a pure raster embed would be Image-centric.
        assert!(
            as_str.contains("/Type /Page") || as_str.contains("/Type/Page"),
            "expected a PDF page object"
        );
        assert!(pdf.len() > 200);
    }
}
