//! SVG → PDF rasterization (Chromium-free; embeds rendered bitmap).

use printpdf::{
    ColorBits, ColorSpace, Image, ImageTransform, ImageXObject, Mm, PdfDocument, Px, SMask,
};

const PDF_DPI: f32 = 96.0;

/// Rasterize SVG text to a single-page PDF (bitmap embedded).
pub fn svg_to_pdf(svg: &str) -> Result<Vec<u8>, String> {
    let pixmap = crate::png::svg_to_pixmap(svg)?;
    let w = pixmap.width() as usize;
    let h = pixmap.height() as usize;
    let width_mm = Mm(w as f32 * (25.4 / PDF_DPI));
    let height_mm = Mm(h as f32 * (25.4 / PDF_DPI));

    let (doc, page1, layer1) = PdfDocument::new("diagram", width_mm, height_mm, "Layer 1");
    let layer = doc.get_page(page1).get_layer(layer1);
    let image = Image::from(pixmap_to_xobject(&pixmap));
    image.add_to_layer(
        layer,
        ImageTransform {
            dpi: Some(PDF_DPI),
            ..Default::default()
        },
    );
    doc.save_to_bytes().map_err(|e| e.to_string())
}

fn pixmap_to_xobject(pixmap: &tiny_skia::Pixmap) -> ImageXObject {
    let w = pixmap.width() as usize;
    let h = pixmap.height() as usize;
    let mut rgb = Vec::with_capacity(w * h * 3);
    let mut alpha = Vec::with_capacity(w * h);
    let mut has_alpha = false;
    for p in pixmap.pixels().iter() {
        let a = p.alpha();
        if a < 255 {
            has_alpha = true;
        }
        rgb.push(p.red());
        rgb.push(p.green());
        rgb.push(p.blue());
        alpha.push(i64::from(a));
    }
    let smask = has_alpha.then(|| SMask {
        width: w as i64,
        height: h as i64,
        interpolate: false,
        bits_per_component: 8,
        matte: alpha,
    });
    ImageXObject {
        width: Px(w),
        height: Px(h),
        color_space: ColorSpace::Rgb,
        bits_per_component: ColorBits::Bit8,
        interpolate: true,
        image_data: rgb,
        image_filter: None,
        smask,
        clipping_bbox: None,
    }
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

    #[test]
    fn flowchart_svg_to_pdf() {
        let doc = ir::from_mermaid("graph TD\n  A[Start] --> B[End]\n").unwrap();
        let svg = doc.render_svg(Theme::Dark).unwrap();
        let pdf = svg_to_pdf(&svg).expect("pdf render");
        assert!(pdf.starts_with(b"%PDF-"));
        assert!(pdf.len() > 200);
    }

    #[test]
    fn sequence_svg_to_pdf() {
        let doc = ir::from_mermaid("sequenceDiagram\n  A->>B: hi\n").unwrap();
        let svg = doc.render_svg(Theme::Dark).unwrap();
        let pdf = svg_to_pdf(&svg).expect("pdf render");
        assert!(pdf.starts_with(b"%PDF-"));
    }
}
