//! Paper files on disk.
//!
//! A paper is a folder with `main.tex` and one file per section under
//! `sections/`. Section files hold one sentence per line and a blank line
//! between paragraphs, so Git diffs land on single sentences.

use crate::text::sentences;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Section {
    /// File name under `sections/`, e.g. `introduction.tex`.
    pub file: String,
    pub title: String,
    pub paragraphs: Vec<String>,
}

/// Accept plain names only: no separators, no parent references, no hidden files.
pub fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && name.chars().all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | ' '))
}

/// Section files in reading order: the order of `\input{sections/...}` in
/// main.tex, then any remaining files in `sections/` alphabetically.
pub fn section_files(paper: &Path) -> Vec<String> {
    let mut order = Vec::new();
    if let Ok(main) = std::fs::read_to_string(paper.join("main.tex")) {
        for line in main.lines() {
            let line = line.split('%').next().unwrap_or("");
            for cmd in ["\\input{", "\\include{"] {
                if let Some(rest) = line.split(cmd).nth(1) {
                    if let Some(arg) = rest.split('}').next() {
                        if let Some(name) = arg.strip_prefix("sections/") {
                            let file = if name.ends_with(".tex") { name.to_string() } else { format!("{name}.tex") };
                            if !order.contains(&file) {
                                order.push(file);
                            }
                        }
                    }
                }
            }
        }
    }
    let mut rest: Vec<String> = std::fs::read_dir(paper.join("sections"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.ends_with(".tex") && !order.contains(n))
        .collect();
    rest.sort();
    order.retain(|f| paper.join("sections").join(f).exists());
    order.extend(rest);
    order
}

/// Split a section file into its `\section{}` title and paragraphs.
pub fn parse_section(file: &str, text: &str) -> Section {
    let mut title = String::new();
    let mut body = String::new();
    for line in text.lines() {
        let t = line.trim_start();
        if title.is_empty() && t.starts_with("\\section") {
            if let Some(start) = t.find('{') {
                let inner = &t[start + 1..];
                title = inner[..inner.rfind('}').unwrap_or(inner.len())].to_string();
            }
            continue;
        }
        body.push_str(line);
        body.push('\n');
    }
    if title.is_empty() {
        title = file.trim_end_matches(".tex").replace(['-', '_'], " ");
    }
    Section { file: file.to_string(), title, paragraphs: crate::text::paragraphs(&body) }
}

/// Write back: one sentence per line for prose; blocks with environments or
/// comments are kept exactly as typed.
pub fn render_section(s: &Section) -> String {
    let mut out = format!("\\section{{{}}}\n", s.title);
    for p in &s.paragraphs {
        let p = p.trim();
        if p.is_empty() {
            continue;
        }
        out.push('\n');
        let keep = p.contains("\\begin") || p.contains('%') || p.starts_with('\\');
        if keep {
            out.push_str(p);
        } else {
            out.push_str(&sentences(p).join("\n"));
        }
        out.push('\n');
    }
    out
}

pub fn load_paper(paper: &Path) -> Vec<Section> {
    section_files(paper)
        .into_iter()
        .filter_map(|f| {
            let text = std::fs::read_to_string(paper.join("sections").join(&f)).ok()?;
            Some(parse_section(&f, &text))
        })
        .collect()
}

/// Atomic write (temp file + rename) so a crash never leaves half a file.
pub fn write_atomic(path: &Path, content: &str) -> std::io::Result<()> {
    let tmp: PathBuf = path.with_extension("tmp~");
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_section_one_sentence_per_line() {
        let text = "\\section{Result}\n\nCompared with A, B wins. It is fast.\n\n\\begin{equation}\nx = 1. y = 2.\n\\end{equation}\n";
        let s = parse_section("result.tex", text);
        assert_eq!(s.title, "Result");
        assert_eq!(s.paragraphs.len(), 2);
        let out = render_section(&s);
        assert!(out.contains("Compared with A, B wins.\nIt is fast.\n"));
        assert!(out.contains("x = 1. y = 2."), "environments kept verbatim");
        // Saving again changes nothing.
        assert_eq!(render_section(&parse_section("result.tex", &out)), out);
    }

    #[test]
    fn rejects_path_tricks() {
        assert!(safe_name("introduction.tex"));
        assert!(!safe_name("../secrets"));
        assert!(!safe_name("a/b.tex"));
        assert!(!safe_name(".git"));
    }
}
