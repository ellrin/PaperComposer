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
        // A real comment is an unescaped %; "12.4\%" is ordinary prose.
        let has_comment = p.lines().any(|l| strip_comment(l).len() != l.len());
        let keep = p.contains("\\begin") || has_comment || p.starts_with('\\');
        if keep {
            out.push_str(p);
        } else {
            out.push_str(&sentences(p).join("\n"));
        }
        out.push('\n');
    }
    out
}

/// `\newcommand`, `\renewcommand`, `\DeclareMathOperator` and simple `\def`
/// from LaTeX source, as name -> body, for the math previewer. Argument
/// counts are dropped: `#1` in the body works the same way in KaTeX.
pub fn extract_macros(src: &str) -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::new();
    let src: String = src.lines().map(|l| strip_comment(l)).collect::<Vec<_>>().join("\n");
    let bytes: Vec<char> = src.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != '\\' {
            i += 1;
            continue;
        }
        let rest: String = bytes[i..bytes.len().min(i + 24)].iter().collect();
        let (kind, skip) = ["\\newcommand", "\\renewcommand", "\\providecommand", "\\DeclareMathOperator", "\\def"]
            .iter()
            .find(|k| rest.starts_with(**k))
            .map(|k| (*k, k.chars().count()))
            .unwrap_or(("", 0));
        if skip == 0 {
            i += 1;
            continue;
        }
        let mut j = i + skip;
        let starred = bytes.get(j) == Some(&'*');
        if starred {
            j += 1;
        }
        // Name: {\name} or \name
        let name;
        while bytes.get(j).is_some_and(|c| c.is_whitespace()) {
            j += 1;
        }
        if bytes.get(j) == Some(&'{') {
            let (n, k) = group(&bytes, j);
            name = n.trim().to_string();
            j = k;
        } else if bytes.get(j) == Some(&'\\') {
            let start = j;
            j += 1;
            while bytes.get(j).is_some_and(|c| c.is_alphabetic()) {
                j += 1;
            }
            name = bytes[start..j].iter().collect();
        } else {
            i += 1;
            continue;
        }
        // Skip [n] and [default] (newcommand) or #1#2 (def).
        loop {
            while bytes.get(j).is_some_and(|c| c.is_whitespace()) {
                j += 1;
            }
            match bytes.get(j) {
                Some('[') => {
                    while j < bytes.len() && bytes[j] != ']' {
                        j += 1;
                    }
                    j += 1;
                }
                Some('#') if kind == "\\def" => j += 2,
                _ => break,
            }
        }
        if bytes.get(j) == Some(&'{') && name.starts_with('\\') {
            let (body, k) = group(&bytes, j);
            // The starred form puts limits under the operator in display math.
            let body = match (kind, starred) {
                ("\\DeclareMathOperator", true) => format!("\\operatorname*{{{body}}}"),
                ("\\DeclareMathOperator", false) => format!("\\operatorname{{{body}}}"),
                _ => body,
            };
            out.insert(name, body);
            i = k;
        } else {
            i = j.max(i + 1);
        }
    }
    out
}

fn strip_comment(line: &str) -> &str {
    let b = line.as_bytes();
    for (i, &c) in b.iter().enumerate() {
        if c == b'%' && (i == 0 || b[i - 1] != b'\\') {
            return &line[..i];
        }
    }
    line
}

/// Content of a balanced `{...}` starting at `open`, and the index after it.
fn group(chars: &[char], open: usize) -> (String, usize) {
    let mut depth = 0;
    let mut j = open;
    while j < chars.len() {
        match chars[j] {
            '\\' => j += 1, // skip escaped char
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return (chars[open + 1..j].iter().collect(), j + 1);
                }
            }
            _ => {}
        }
        j += 1;
    }
    (chars[open + 1..].iter().collect(), chars.len())
}

/// Macros for a paper: main.tex preamble plus any macros*.tex beside it.
pub fn paper_macros(paper: &Path) -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::new();
    let mut files = vec![paper.join("main.tex")];
    if let Ok(dir) = std::fs::read_dir(paper) {
        for e in dir.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if n.starts_with("macros") && n.ends_with(".tex") {
                files.push(e.path());
            }
        }
    }
    for f in files {
        if let Ok(src) = std::fs::read_to_string(f) {
            out.extend(extract_macros(&src));
        }
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

/// Sections a blank paper starts with: (file stem, heading).
const BLANK_SECTIONS: &[(&str, &str)] = &[
    ("introduction", "Introduction"),
    ("related-work", "Related work"),
    ("method", "Method"),
    ("result", "Result"),
    ("discussion", "Discussion"),
    ("conclusion", "Conclusion"),
];

/// Create `papers/<name>/` with a plain article main.tex, empty sections and
/// an empty refs.bib. Refuses to touch an existing folder.
pub fn create_paper(papers: &Path, name: &str) -> Result<PathBuf, String> {
    let name = name.trim();
    if !safe_name(name) {
        return Err("名稱只能用文字、數字、空格、- _ .".into());
    }
    let dir = papers.join(name);
    if dir.exists() {
        return Err(format!("已經有一篇叫 {name} 的論文"));
    }
    let io = |e: std::io::Error| e.to_string();
    std::fs::create_dir_all(dir.join("sections")).map_err(io)?;
    let inputs: String = BLANK_SECTIONS.iter().map(|(f, _)| format!("\\input{{sections/{f}}}\n")).collect();
    let main = format!(
        "\\documentclass{{article}}\n\\usepackage{{amsmath,amssymb}}\n\n\\title{{{name}}}\n\\author{{}}\n\n\\begin{{document}}\n\\maketitle\n\n{inputs}\n\\bibliographystyle{{plain}}\n\\bibliography{{refs}}\n\\end{{document}}\n"
    );
    write_atomic(&dir.join("main.tex"), &main).map_err(io)?;
    for (f, title) in BLANK_SECTIONS {
        let s = Section { file: format!("{f}.tex"), title: (*title).into(), paragraphs: vec![] };
        write_atomic(&dir.join("sections").join(&s.file), &render_section(&s)).map_err(io)?;
    }
    write_atomic(&dir.join("refs.bib"), "").map_err(io)?;
    Ok(dir)
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
        let text = "\\section{Result}\n\nCompared with A, B wins by 12.4\\%. It is fast.\n\n\\begin{equation}\nx = 1. y = 2.\n\\end{equation}\n";
        let s = parse_section("result.tex", text);
        assert_eq!(s.title, "Result");
        assert_eq!(s.paragraphs.len(), 2);
        let out = render_section(&s);
        assert!(out.contains("Compared with A, B wins by 12.4\\%.\nIt is fast.\n"), "{out}");
        assert!(out.contains("x = 1. y = 2."), "environments kept verbatim");
        // Saving again changes nothing.
        assert_eq!(render_section(&parse_section("result.tex", &out)), out);
    }

    #[test]
    fn creates_a_blank_paper_that_loads() {
        let tmp = std::env::temp_dir().join(format!("pc-blank-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let dir = create_paper(&tmp, "我的 論文-1").unwrap();
        let secs = load_paper(&dir);
        assert_eq!(secs.len(), BLANK_SECTIONS.len());
        assert_eq!(secs[0].title, "Introduction");
        assert!(secs.iter().all(|s| s.paragraphs.is_empty()));
        assert!(create_paper(&tmp, "我的 論文-1").is_err(), "never overwrites");
        assert!(create_paper(&tmp, "../escape").is_err());
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn extracts_macros() {
        let src = r"\newcommand{\R}{\mathbb{R}} % reals
\newcommand\norm[1]{\left\lVert #1 \right\rVert}
\renewcommand*{\vec}[1]{\mathbf{#1}}
\DeclareMathOperator*{\argmax}{arg\,max}
\def\eps{\varepsilon}
% \newcommand{\hidden}{x}";
        let m = extract_macros(src);
        assert_eq!(m["\\R"], "\\mathbb{R}");
        assert_eq!(m["\\norm"], "\\left\\lVert #1 \\right\\rVert");
        assert_eq!(m["\\vec"], "\\mathbf{#1}");
        assert_eq!(m["\\argmax"], "\\operatorname*{arg\\,max}");
        assert_eq!(m["\\eps"], "\\varepsilon");
        assert!(!m.contains_key("\\hidden"));
    }

    #[test]
    fn rejects_path_tricks() {
        assert!(safe_name("introduction.tex"));
        assert!(!safe_name("../secrets"));
        assert!(!safe_name("a/b.tex"));
        assert!(!safe_name(".git"));
    }
}
