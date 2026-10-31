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
    /// Whatever follows `\section{...}` on its line, e.g. `\label{sec1}`;
    /// kept verbatim so references survive a save.
    #[serde(default)]
    pub tail: String,
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
    let mut tail = String::new();
    let mut body = String::new();
    let mut seen = false;
    for line in text.lines() {
        let t = line.trim_start();
        if !seen && t.starts_with("\\section") {
            if let Some((arg, rest)) = t.find('{').and_then(|i| braced(&t[i..])) {
                title = arg.to_string();
                tail = rest.trim().to_string();
                seen = true;
                continue;
            }
        }
        body.push_str(line);
        body.push('\n');
    }
    if title.is_empty() {
        title = file.trim_end_matches(".tex").replace(['-', '_'], " ");
    }
    Section { file: file.to_string(), title, paragraphs: crate::text::paragraphs(&body), tail }
}

/// `{...}rest` → (inside, rest), honouring nested braces.
fn braced(s: &str) -> Option<(&str, &str)> {
    let mut depth = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some((&s[1..i], &s[i + 1..]));
                }
            }
            _ => {}
        }
    }
    None
}

/// Write back: one sentence per line for prose; blocks with environments or
/// comments are kept exactly as typed.
pub fn render_section(s: &Section) -> String {
    let mut out = format!("\\section{{{}}}{}\n", s.title, s.tail);
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
        let s = Section { file: format!("{f}.tex"), title: (*title).into(), paragraphs: vec![], tail: String::new() };
        write_atomic(&dir.join("sections").join(&s.file), &render_section(&s)).map_err(io)?;
    }
    write_atomic(&dir.join("refs.bib"), "").map_err(io)?;
    Ok(dir)
}

/// Files in a publisher folder that are not copied into a paper.
const TEMPLATE_SKIP: &[&str] = &["sample.tex", "template.toml", "README", "README.md", "user-manual.pdf"];

/// Create `papers/<name>/` from a publisher template (`publishers/<x>/`):
/// the sample's preamble, front matter and back matter stay in main.tex,
/// each `\section` becomes a file in `sections/` (comment lines dropped),
/// and the class, style and support files are copied alongside so the paper
/// compiles on its own.
pub fn create_paper_from_template(papers: &Path, name: &str, template: &Path) -> Result<PathBuf, String> {
    let name = name.trim();
    if !safe_name(name) {
        return Err("名稱只能用文字、數字、空格、- _ .".into());
    }
    let bytes = std::fs::read(template.join("sample.tex")).map_err(|_| "模板裡沒有 sample.tex".to_string())?;
    // Some publishers still ship Latin-1 samples; read those byte for byte.
    let sample = String::from_utf8(bytes).unwrap_or_else(|e| e.into_bytes().iter().map(|&b| b as char).collect());
    let split = split_sample(&sample).ok_or("sample.tex 裡找不到 \\begin{document}")?;
    let dir = papers.join(name);
    if dir.exists() {
        return Err(format!("已經有一篇叫 {name} 的論文"));
    }
    let io = |e: std::io::Error| e.to_string();
    std::fs::create_dir_all(dir.join("sections")).map_err(io)?;
    copy_support(template, &dir).map_err(io)?;

    let mut inputs = String::new();
    for (i, (title, tail, body)) in split.sections.iter().enumerate() {
        let file = format!("{:02}-{}.tex", i + 1, slug(title));
        let mut s = parse_section(&file, &format!("\\section{{{title}}}{tail}\n{body}"));
        s.file = file.clone();
        write_atomic(&dir.join("sections").join(&file), &render_section(&s)).map_err(io)?;
        inputs += &format!("\\input{{sections/{}}}\n", file.trim_end_matches(".tex"));
    }
    let main = format!("{}\\begin{{document}}\n{}\n{}\n{}\\end{{document}}\n", split.preamble, split.front.trim_end(), inputs, split.back);
    write_atomic(&dir.join("main.tex"), &main).map_err(io)?;
    Ok(dir)
}

struct Sample {
    preamble: String,
    front: String,
    /// (title, rest of the \section line, body without comment lines)
    sections: Vec<(String, String, String)>,
    back: String,
}

/// Split a single-file sample into preamble / front matter / sections /
/// back matter. Back matter starts at the bibliography or appendices.
fn split_sample(src: &str) -> Option<Sample> {
    let begin = src.find("\\begin{document}")?;
    let preamble = src[..begin].to_string();
    let after = &src[begin + "\\begin{document}".len()..];
    let end = after.rfind("\\end{document}").unwrap_or(after.len());
    let doc = &after[..end];

    // Line-based walk over the document, ignoring commented-out lines when
    // looking for structure.
    let back_marks = ["\\appendix", "\\appendices", "\\begin{appendices}", "\\bibliography", "\\printbibliography", "\\begin{thebibliography}", "\\backmatter", "\\bibliographystyle"];
    let mut front = String::new();
    let mut back = String::new();
    let mut sections: Vec<(String, String, String)> = Vec::new();
    let mut in_back = false;
    for line in doc.lines() {
        let code = strip_comment(line).trim();
        if in_back {
            back += line;
            back.push('\n');
            continue;
        }
        if back_marks.iter().any(|m| code.starts_with(m)) {
            in_back = true;
            back += line;
            back.push('\n');
            continue;
        }
        if code.starts_with("\\section") && !code.starts_with("\\section*") {
            if let Some((t, rest)) = code.find('{').and_then(|i| braced(&code[i..])) {
                sections.push((t.to_string(), rest.trim().to_string(), String::new()));
                continue;
            }
        }
        match sections.last_mut() {
            Some((_, _, body)) => {
                // Drop pure comment lines: samples are full of instructions.
                if line.trim_start().starts_with('%') {
                    continue;
                }
                body.push_str(line);
                body.push('\n');
            }
            None => {
                front += line;
                front.push('\n');
            }
        }
    }
    Some(Sample { preamble, front, sections, back })
}

fn slug(title: &str) -> String {
    let s: String = title
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    let s = s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    if s.is_empty() { "section".into() } else { s.chars().take(40).collect() }
}

fn copy_support(from: &Path, to: &Path) -> std::io::Result<()> {
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let name = e.file_name();
        let n = name.to_string_lossy();
        if n.starts_with('.') || TEMPLATE_SKIP.contains(&n.as_ref()) {
            continue;
        }
        let dst = to.join(&name);
        if e.file_type()?.is_dir() {
            std::fs::create_dir_all(&dst)?;
            copy_support(&e.path(), &dst)?;
        } else {
            std::fs::copy(e.path(), dst)?;
        }
    }
    Ok(())
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
    fn section_title_with_label_round_trips() {
        let s = parse_section("a.tex", "\\section{Results for $\\mathbb{R}^{n}$}\\label{sec:res}\n\nOne. Two.\n");
        assert_eq!(s.title, "Results for $\\mathbb{R}^{n}$");
        assert_eq!(s.tail, "\\label{sec:res}");
        let out = render_section(&s);
        assert!(out.starts_with("\\section{Results for $\\mathbb{R}^{n}$}\\label{sec:res}\n"), "{out}");
    }

    #[test]
    fn creates_a_paper_from_a_single_file_template() {
        let root = std::env::temp_dir().join(format!("pc-tpl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let tpl = root.join("publishers/demo");
        std::fs::create_dir_all(tpl.join("bst")).unwrap();
        std::fs::write(tpl.join("demo.cls"), "% class").unwrap();
        std::fs::write(tpl.join("bst/x.bst"), "").unwrap();
        std::fs::write(tpl.join("README"), "skip me").unwrap();
        std::fs::write(
            tpl.join("sample.tex"),
            "\\documentclass{demo}\n\\begin{document}\n\\title{T}\\maketitle\n\\begin{abstract}A.\\end{abstract}\n\
% instructions\n\\section{Introduction}\\label{sec1}\n% delete me\nFirst idea. Second idea.\n\n\\section{Methods}\nWe did it.\n\
\\bibliography{refs}\n\\end{document}\n",
        )
        .unwrap();
        let dir = create_paper_from_template(&root.join("papers"), "My paper", &tpl).unwrap();
        let main = std::fs::read_to_string(dir.join("main.tex")).unwrap();
        assert!(main.starts_with("\\documentclass{demo}"));
        assert!(main.contains("\\begin{abstract}A.\\end{abstract}"));
        assert!(main.contains("\\input{sections/01-introduction}\n\\input{sections/02-methods}"), "{main}");
        assert!(main.contains("\\bibliography{refs}"));
        assert!(dir.join("demo.cls").exists() && dir.join("bst/x.bst").exists());
        assert!(!dir.join("README").exists() && !dir.join("sample.tex").exists());
        let secs = load_paper(&dir);
        assert_eq!(secs.len(), 2);
        assert_eq!((secs[0].title.as_str(), secs[0].tail.as_str()), ("Introduction", "\\label{sec1}"));
        assert_eq!(secs[0].paragraphs, vec!["First idea.\nSecond idea.".to_string()], "one sentence per line, comments dropped");
        assert!(create_paper_from_template(&root.join("papers"), "My paper", &tpl).is_err());
        std::fs::remove_dir_all(&root).unwrap();
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
