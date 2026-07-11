//! Markdown pipeline: extract fenced diagram blocks, render, rewrite image links.

use crate::formats::{self, Format};
use crate::renderer::Theme;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FencedBlock {
    pub start_line: usize,
    pub end_line: usize,
    pub lang: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct RenderedBlock {
    pub index: usize,
    pub image_path: PathBuf,
    pub link: String,
}

#[derive(Debug, Clone)]
pub struct ProcessResult {
    pub blocks_found: usize,
    pub blocks_rendered: usize,
    pub rendered: Vec<RenderedBlock>,
    pub output_markdown: String,
}

#[derive(Debug, Clone)]
pub struct ProcessOptions {
    pub image_format: ImageFormat,
    pub theme: Theme,
    pub name_prefix: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Svg,
}

impl ImageFormat {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "png" => Some(Self::Png),
            "svg" => Some(Self::Svg),
            _ => None,
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Svg => "svg",
        }
    }
}

/// Extract fenced code blocks from Markdown (line indices are 0-based).
pub fn extract_fenced_blocks(source: &str) -> Vec<FencedBlock> {
    let lines: Vec<&str> = source.lines().collect();
    let mut blocks = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        if let Some(lang) = line.strip_prefix("```") {
            let lang = lang.trim().to_string();
            let start = i;
            i += 1;
            let mut content = String::new();
            while i < lines.len() && !lines[i].trim().starts_with("```") {
                if !content.is_empty() {
                    content.push('\n');
                }
                content.push_str(lines[i]);
                i += 1;
            }
            if i < lines.len() {
                blocks.push(FencedBlock {
                    start_line: start,
                    end_line: i,
                    lang,
                    content,
                });
            }
        }
        i += 1;
    }
    blocks
}

pub fn is_diagram_block(block: &FencedBlock) -> bool {
    if !block.lang.is_empty() {
        return matches!(
            block.lang.to_lowercase().as_str(),
            "mermaid" | "mmd" | "plantuml" | "puml" | "dot" | "graphviz" | "gv"
        );
    }
    should_process_unlabeled(&block.content)
}

fn should_process_unlabeled(content: &str) -> bool {
    let trimmed = content.trim_start();
    if trimmed.starts_with('{') {
        return false;
    }
    if crate::formats::plantuml::is_plantuml(content) || crate::formats::dot::is_dot(content) {
        return true;
    }
    crate::sequence::is_sequence(content)
        || crate::class::is_class(content)
        || crate::gantt::is_gantt(content)
        || trimmed.starts_with("graph ")
        || trimmed.starts_with("flowchart ")
}

fn lang_to_format(lang: &str, content: &str) -> Format {
    match lang.to_lowercase().as_str() {
        "mermaid" | "mmd" => Format::Mermaid,
        "plantuml" | "puml" => Format::PlantUml,
        "dot" | "graphviz" | "gv" => Format::Dot,
        _ => formats::detect(content, None),
    }
}

fn relative_link(markdown_out: &Path, image: &Path) -> String {
    let base = markdown_out.parent().unwrap_or_else(|| Path::new("."));
    let mut from = base.components().collect::<Vec<_>>();
    let mut to = image.components().collect::<Vec<_>>();
    while !from.is_empty() && !to.is_empty() && from[0] == to[0] {
        from.remove(0);
        to.remove(0);
    }
    let mut parts: Vec<String> = (0..from.len()).map(|_| "..".to_string()).collect();
    for c in to {
        parts.push(c.as_os_str().to_string_lossy().into_owned());
    }
    if parts.is_empty() {
        image.file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| image.display().to_string())
    } else {
        parts.join("/")
    }
}

/// Process Markdown: render diagram fences to images and rewrite with `![diagram](...)`.
pub fn process_markdown(
    source: &str,
    markdown_out: &Path,
    output_dir: &Path,
    opts: &ProcessOptions,
) -> Result<ProcessResult, String> {
    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create '{}': {e}", output_dir.display()))?;

    let blocks: Vec<_> = extract_fenced_blocks(source)
        .into_iter()
        .filter(is_diagram_block)
        .collect();

    let mut rendered = Vec::new();
    let mut replacements: Vec<(usize, usize, String)> = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    for (index, block) in blocks.iter().enumerate() {
        let format = lang_to_format(&block.lang, &block.content);
        let doc = formats::import_str(&block.content, format)
            .map_err(|e| format!("block {}: {e}", index))?;
        let svg = doc
            .render_svg(opts.theme)
            .map_err(|e| format!("block {}: {e}", index))?;

        let filename = format!("{}-{}.{}", opts.name_prefix, index, opts.image_format.extension());
        let image_path = output_dir.join(&filename);

        match opts.image_format {
            ImageFormat::Svg => {
                std::fs::write(&image_path, &svg)
                    .map_err(|e| format!("Failed to write '{}': {e}", image_path.display()))?;
            }
            ImageFormat::Png => {
                let png = crate::png::svg_to_png(&svg)?;
                std::fs::write(&image_path, png)
                    .map_err(|e| format!("Failed to write '{}': {e}", image_path.display()))?;
            }
        }

        let link = relative_link(markdown_out, &image_path);
        let replacement = format!("![diagram {index}]({link})");
        replacements.push((block.start_line, block.end_line, replacement));
        rendered.push(RenderedBlock {
            index,
            image_path,
            link,
        });
    }

    let mut output_lines: Vec<String> = lines.iter().map(|s| (*s).to_string()).collect();
    replacements.sort_by(|a, b| b.0.cmp(&a.0));
    for (start, end, replacement) in replacements {
        output_lines.splice(start..=end, [replacement]);
    }

    Ok(ProcessResult {
        blocks_found: blocks.len(),
        blocks_rendered: rendered.len(),
        rendered,
        output_markdown: output_lines.join("\n"),
    })
}

/// Process a Markdown file on disk.
pub fn process_markdown_file(
    input: &Path,
    markdown_out: &Path,
    output_dir: &Path,
    opts: &ProcessOptions,
) -> Result<ProcessResult, String> {
    let source = std::fs::read_to_string(input)
        .map_err(|e| format!("Failed to read '{}': {e}", input.display()))?;
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "doc".into());
    let mut opts = opts.clone();
    opts.name_prefix = stem;
    let result = process_markdown(&source, markdown_out, output_dir, &opts)?;
    std::fs::write(markdown_out, &result.output_markdown)
        .map_err(|e| format!("Failed to write '{}': {e}", markdown_out.display()))?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_two_mermaid_blocks() {
        let md = "# Title\n\n```mermaid\ngraph TD\n  A-->B\n```\n\n```mermaid\nsequenceDiagram\n  A->>B: hi\n```\n";
        let blocks = extract_fenced_blocks(md);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].lang, "mermaid");
        assert!(blocks[0].content.contains("graph TD"));
    }

    #[test]
    fn skips_non_diagram_fences() {
        let md = "```rust\nfn main() {}\n```\n```mermaid\ngraph TD\n  A-->B\n```";
        let blocks: Vec<_> = extract_fenced_blocks(md)
            .into_iter()
            .filter(is_diagram_block)
            .collect();
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn process_rewrites_links() {
        let md = "# Doc\n\n```mermaid\ngraph TD\n  A-->B\n```\n";
        let dir = std::env::temp_dir().join(format!("diagram_md_{}", std::process::id()));
        let out_md = dir.join("out.md");
        let img_dir = dir.join("assets");
        std::fs::create_dir_all(&img_dir).unwrap();
        let result = process_markdown(
            md,
            &out_md,
            &img_dir,
            &ProcessOptions {
                image_format: ImageFormat::Png,
                theme: Theme::Dark,
                name_prefix: "test".into(),
            },
        )
        .unwrap();
        assert_eq!(result.blocks_rendered, 1);
        assert!(result.output_markdown.contains("![diagram 0]"));
        assert!(!result.output_markdown.contains("```mermaid"));
        assert!(result.rendered[0].image_path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
