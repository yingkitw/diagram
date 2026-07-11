//! Combine multiple diagram SVGs into one vertical figure.

const GAP: f64 = 32.0;

/// Stack SVGs vertically (centred on the widest diagram).
pub fn combine_svgs(svgs: &[String]) -> Result<String, String> {
    if svgs.is_empty() {
        return Err("no SVGs to combine".into());
    }
    if svgs.len() == 1 {
        return Ok(svgs[0].clone());
    }

    let parsed: Vec<ParsedSvg> = svgs.iter().map(|s| parse_svg(s)).collect::<Result<_, _>>()?;
    let max_w = parsed
        .iter()
        .map(|p| p.width)
        .fold(0.0_f64, f64::max);
    let total_h: f64 = parsed.iter().map(|p| p.height).sum::<f64>()
        + GAP * (parsed.len() - 1) as f64;

    let mut out = String::new();
    out.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{max_w:.0}" height="{total_h:.0}" viewBox="0 0 {max_w:.0} {total_h:.0}">"#
    ));

    let mut y = 0.0;
    for part in &parsed {
        let x = (max_w - part.width) / 2.0;
        out.push_str(&format!(r#"<g transform="translate({x:.1},{y:.1})">"#));
        out.push_str(&part.inner);
        out.push_str("</g>");
        y += part.height + GAP;
    }
    out.push_str("</svg>");
    Ok(out)
}

struct ParsedSvg {
    width: f64,
    height: f64,
    inner: String,
}

fn parse_svg(svg: &str) -> Result<ParsedSvg, String> {
    let open_end = svg.find('>').ok_or("SVG missing opening tag")?;
    let open = &svg[..=open_end];
    let width = attr_number(open, "width").ok_or("SVG missing width")?;
    let height = attr_number(open, "height").ok_or("SVG missing height")?;
    let close = svg
        .rfind("</svg>")
        .ok_or("SVG missing closing tag")?;
    if close <= open_end {
        return Err("SVG has empty body".into());
    }
    let inner = svg[open_end + 1..close].trim().to_string();
    Ok(ParsedSvg {
        width,
        height,
        inner,
    })
}

fn attr_number(tag: &str, name: &str) -> Option<f64> {
    for pattern in [format!(r#"{name}=""#), format!(r"{name}='"), format!(r"{name}=")] {
        let Some(start) = tag.find(&pattern) else {
            continue;
        };
        let rest = &tag[start + pattern.len()..];
        let end = rest
            .find(['"', '\''])
            .or_else(|| rest.find(|c: char| c.is_whitespace() || c == '>'))?;
        return rest[..end].trim_end_matches('%').parse().ok();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(w: f64, h: f64, label: &str) -> String {
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}"><text>{label}</text></svg>"#
        )
    }

    #[test]
    fn combine_two_increases_height() {
        let a = sample(100.0, 50.0, "A");
        let b = sample(80.0, 40.0, "B");
        let out = combine_svgs(&[a, b]).unwrap();
        assert!(out.contains("translate(0.0,0.0)"));
        assert!(out.contains("translate(10.0,82.0)"));
        assert!(out.contains(r#"height="122"#) || out.contains(r#"height="122.0"#));
    }
}
