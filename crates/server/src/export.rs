//! Export a paper: compile main.tex to PDF with the journal's own class, or
//! convert it to Word with Pandoc. Intermediate files go to the user's cache
//! folder; only the finished file lands in `papers/<name>/export/`, which
//! carries its own .gitignore so exports never reach the repository.

use crate::texmeasure::tool;
use pc_core::paper;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::process::Command;

#[derive(Serialize, Debug, Default, PartialEq)]
pub struct Problem {
    /// Path relative to the paper folder, when TeX reported one.
    pub file: Option<String>,
    pub line: Option<u32>,
    pub message: String,
    /// The source text TeX stopped at (the `l.12 ...` line).
    pub context: Option<String>,
    /// What to do about it, when we know.
    pub hint: Option<String>,
    /// Which paragraph of the section file the line falls in.
    pub para: Option<usize>,
}

#[derive(Serialize, Debug, Default)]
pub struct Report {
    pub ok: bool,
    /// The finished file, relative to the paper folder.
    pub file: Option<String>,
    pub pages: Option<u32>,
    pub errors: Vec<Problem>,
    pub overfull: u32,
    pub undefined_cites: Vec<String>,
    pub undefined_refs: Vec<String>,
    pub warnings: Vec<String>,
    pub seconds: f64,
}

/// What the log of one LaTeX run says.
#[derive(Debug, Default, PartialEq)]
pub struct LogInfo {
    pub pages: Option<u32>,
    pub errors: Vec<Problem>,
    pub overfull: u32,
    pub undefined_cites: Vec<String>,
    pub undefined_refs: Vec<String>,
    pub rerun: bool,
}

/// `% !TEX program = xelatex`, or a package that needs a Unicode engine.
pub fn engine_for(main: &str) -> &'static str {
    for line in main.lines().take(20) {
        let l = line.trim().to_ascii_lowercase();
        if let Some(p) = l.strip_prefix("%").map(str::trim).and_then(|l| l.strip_prefix("!tex")).and_then(|l| l.split('=').nth(1)) {
            match p.trim() {
                "xelatex" => return "xelatex",
                "lualatex" => return "lualatex",
                "pdflatex" => return "pdflatex",
                _ => {}
            }
        }
    }
    let code: String = main.lines().map(|l| l.split('%').next().unwrap_or("")).collect::<Vec<_>>().join("\n");
    if ["{fontspec}", "{xeCJK}", "{ctex}", "{unicode-math}"].iter().any(|p| code.contains(p)) || code.contains("{ctexart}") {
        "xelatex"
    } else {
        "pdflatex"
    }
}

/// The bibliography program main.tex needs, if any.
pub fn bib_tool(main: &str) -> Option<&'static str> {
    let code: String = main.lines().map(|l| l.split('%').next().unwrap_or("")).collect::<Vec<_>>().join("\n");
    if code.contains("{biblatex}") {
        return Some(if code.contains("backend=bibtex") { "bibtex" } else { "biber" });
    }
    code.contains("\\bibliography{").then_some("bibtex")
}

fn between<'a>(s: &'a str, open: &str, close: char) -> Option<&'a str> {
    let i = s.find(open)? + open.len();
    let j = s[i..].find(close)?;
    Some(&s[i..i + j])
}

fn hint_for(msg: &str) -> Option<String> {
    if let Some(f) = between(msg, "File `", '\'').filter(|f| f.ends_with(".sty") || f.ends_with(".cls")) {
        let pkg = f.trim_end_matches(".sty").trim_end_matches(".cls");
        return Some(format!("缺少 {f}：sudo tlmgr install {pkg}"));
    }
    if msg.contains("Undefined control sequence") {
        return Some("指令拼錯，或少載入它的套件".into());
    }
    if msg.contains("Missing $ inserted") {
        return Some("數學符號要放在 $…$ 裡".into());
    }
    if msg.contains("Misplaced alignment tab character &") {
        return Some("表格外的 & 要寫成 \\&".into());
    }
    None
}

/// Read a log written with -file-line-error and max_print_line large enough
/// that lines are not wrapped.
pub fn parse_log(log: &str) -> LogInfo {
    let mut info = LogInfo::default();
    let lines: Vec<&str> = log.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        // "./sections/02-method.tex:14: Undefined control sequence."
        let located = line.split_once(".tex:").and_then(|(f, rest)| {
            let (n, msg) = rest.split_once(": ")?;
            let n: u32 = n.parse().ok()?;
            Some((format!("{f}.tex"), n, msg))
        });
        let (file, ln, msg) = match located.filter(|(_, _, m)| !m.trim_start().starts_with("==>")) {
            Some((f, n, m)) if !f.contains(' ') => (Some(f.trim_start_matches("./").to_string()), Some(n), m.to_string()),
            _ => match line.strip_prefix("! ") {
                Some(m) => (None, None, m.to_string()),
                None => {
                    scan_warning(line, &mut info);
                    continue;
                }
            },
        };
        let context = lines[i + 1..].iter().take(12).find(|l| l.starts_with("l.")).map(|l| {
            let rest = l.trim_start_matches("l.");
            rest.split_once(' ').map(|(_, t)| t.trim().to_string()).unwrap_or_default()
        });
        let line_no = ln.or_else(|| {
            lines[i + 1..].iter().take(12).find(|l| l.starts_with("l.")).and_then(|l| l[2..].split(' ').next()?.parse().ok())
        });
        let hint = hint_for(&msg);
        info.errors.push(Problem { file, line: line_no, message: msg, context, hint, para: None });
    }
    // "Emergency stop" only repeats that an earlier error halted the run.
    if info.errors.len() > 1 {
        info.errors.retain(|e| !e.message.starts_with("Emergency stop"));
    }
    info
}

fn scan_warning(line: &str, info: &mut LogInfo) {
    if line.starts_with("Overfull \\hbox") {
        info.overfull += 1;
    } else if let Some(k) = line.contains("Warning: Citation").then(|| between(line, "Citation `", '\'')).flatten() {
        for k in k.split(',').map(str::trim) {
            if !info.undefined_cites.iter().any(|x| x == k) {
                info.undefined_cites.push(k.to_string());
            }
        }
    } else if let Some(k) = (line.contains("Warning: Reference") && line.contains("undefined")).then(|| between(line, "Reference `", '\'')).flatten() {
        if !info.undefined_refs.iter().any(|x| x == k) {
            info.undefined_refs.push(k.to_string());
        }
    } else if let Some(n) = line.contains("Output written on").then(|| between(line, " (", ' ')).flatten() {
        info.pages = n.parse().ok();
    }
    if line.contains("Rerun to get") || line.contains("Label(s) may have changed") || line.contains("undefined references") {
        info.rerun = true;
    }
}

/// Which paragraph of a section file holds `line` (1-based). Paragraphs are
/// separated by blank lines after the \section line, as `parse_section` reads them.
pub fn para_at(file: &str, text: &str, line: u32) -> Option<usize> {
    let target = text.lines().nth(line.checked_sub(1)? as usize)?.trim();
    if target.is_empty() {
        return None;
    }
    let sec = paper::parse_section(file, text);
    sec.paragraphs.iter().position(|p| p.lines().any(|l| l.trim() == target))
        .or_else(|| sec.paragraphs.iter().position(|p| p.contains(target)))
}

pub fn cache_dir(name: &str) -> PathBuf {
    dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("PaperComposer/build").join(name)
}

/// `papers/<name>/export/`, ignored by git through its own .gitignore.
pub fn export_dir(paper: &Path) -> std::io::Result<PathBuf> {
    let d = paper.join("export");
    std::fs::create_dir_all(&d)?;
    let ignore = d.join(".gitignore");
    if !ignore.exists() {
        std::fs::write(ignore, "# Exports are rebuilt from the sources.\n*\n")?;
    }
    Ok(d)
}

async fn run(cmd: &mut Command, secs: u64) -> Result<std::process::Output, String> {
    tokio::time::timeout(Duration::from_secs(secs), cmd.kill_on_drop(true).output())
        .await
        .map_err(|_| "編譯逾時".to_string())?
        .map_err(|e| e.to_string())
}

fn search_path(paper: &Path) -> String {
    let sep = if cfg!(windows) { ";" } else { ":" };
    // the paper folder, then the usual search path (trailing separator)
    format!("{}{sep}", paper.display())
}

/// Mirror the paper's sub-folders in the build folder: TeX cannot create
/// folders, and \include writes an .aux next to each included file.
fn mirror_dirs(src: &Path, dst: &Path) {
    let Ok(rd) = std::fs::read_dir(src) else { return };
    for e in rd.flatten() {
        let n = e.file_name();
        let n = n.to_string_lossy();
        if e.path().is_dir() && !n.starts_with('.') && n != "export" {
            let d = dst.join(&*n);
            let _ = std::fs::create_dir_all(&d);
            mirror_dirs(&e.path(), &d);
        }
    }
}

/// Fill in which paragraph each located error points at.
fn locate(paper: &Path, errors: &mut [Problem]) {
    for e in errors {
        if let (Some(f), Some(n)) = (e.file.as_ref().filter(|f| f.starts_with("sections/")), e.line) {
            if let Ok(text) = std::fs::read_to_string(paper.join(f)) {
                let name = Path::new(f).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                e.para = para_at(&name, &text, n);
            }
        }
    }
}

/// Compile `main.tex` to `export/<name>.pdf`: engine, bibliography, then
/// engine again until cross-references settle (at most four runs).
pub async fn build_pdf(paper: &Path, name: &str, prebuilt_bbl: bool) -> Result<Report, String> {
    let start = Instant::now();
    let main = std::fs::read_to_string(paper.join("main.tex")).map_err(|_| "論文沒有 main.tex".to_string())?;
    let engine = engine_for(&main);
    let tex = tool(engine).ok_or_else(|| format!("找不到 {engine}：請先安裝 TeX"))?;
    let out = cache_dir(name);
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    mirror_dirs(paper, &out);
    let _ = std::fs::remove_file(out.join("main.pdf"));
    if prebuilt_bbl {
        std::fs::copy(paper.join("main.bbl"), out.join("main.bbl")).map_err(|e| e.to_string())?;
    }
    let latex = |out: &Path| {
        let mut c = Command::new(&tex);
        c.current_dir(paper)
            .env("max_print_line", "100000")
            .env("error_line", "254")
            .env("half_error_line", "238")
            // TeX's default restricted shell escape: lets epstopdf convert
            // .eps figures, nothing else
            .args(["-interaction=nonstopmode", "-halt-on-error", "-file-line-error"])
            .arg(format!("-output-directory={}", out.display()))
            .arg("main.tex");
        c
    };
    let mut report = Report::default();
    let mut info = LogInfo::default();
    for pass in 0..4 {
        let o = run(&mut latex(&out), 120).await?;
        let log = std::fs::read_to_string(out.join("main.log")).unwrap_or_default();
        info = parse_log(&log);
        if !o.status.success() || !info.errors.is_empty() {
            if info.errors.is_empty() {
                info.errors.push(Problem { message: format!("{engine} 失敗（結束碼 {:?}）", o.status.code()), ..Default::default() });
            }
            break;
        }
        if pass == 0 {
            if let Some(b) = bib_tool(&main).filter(|_| !prebuilt_bbl) {
                report.warnings.extend(run_bib(b, paper, &out).await?);
                continue;
            }
        }
        if !info.rerun {
            break;
        }
    }
    report.seconds = (start.elapsed().as_secs_f64() * 10.0).round() / 10.0;
    report.pages = info.pages;
    report.overfull = info.overfull;
    report.undefined_cites = info.undefined_cites;
    report.undefined_refs = info.undefined_refs;
    report.errors = info.errors;
    locate(paper, &mut report.errors);
    if report.errors.is_empty() {
        let dest = export_dir(paper).map_err(|e| e.to_string())?.join(format!("{name}.pdf"));
        std::fs::copy(out.join("main.pdf"), &dest).map_err(|_| "編譯完成但沒有產生 PDF".to_string())?;
        report.ok = true;
        report.file = Some(format!("export/{name}.pdf"));
    }
    Ok(report)
}

/// Run BibTeX or Biber in the build folder; returns its warnings.
async fn run_bib(prog: &str, paper: &Path, out: &Path) -> Result<Vec<String>, String> {
    let exe = tool(prog).ok_or_else(|| format!("找不到 {prog}：sudo tlmgr install {prog}"))?;
    let mut c = Command::new(exe);
    c.current_dir(out).env("BIBINPUTS", search_path(paper)).env("BSTINPUTS", search_path(paper));
    if prog == "biber" {
        c.arg(format!("--input-directory={}", paper.display()));
    }
    let o = run(c.arg("main"), 60).await?;
    let log = std::fs::read_to_string(out.join("main.blg")).unwrap_or_default();
    let mut warn = bib_warnings(&log);
    if !o.status.success() && warn.is_empty() {
        warn.push(format!("{prog} 失敗"));
    }
    Ok(warn)
}

/// The useful lines of a .blg file, with where the problem is.
pub fn bib_warnings(blg: &str) -> Vec<String> {
    let mut v = Vec::new();
    let lines: Vec<&str> = blg.lines().collect();
    for (i, l) in lines.iter().enumerate() {
        // "Repeated entry---line 494 of file references.bib", then " : @article{Key"
        let at = l.split_once("---line ").and_then(|(what, rest)| {
            let (n, file) = rest.split_once(" of file ")?;
            Some((what, n.trim(), file.trim()))
        });
        let msg = if let Some(f) = l.strip_prefix("I couldn't open style file ") {
            format!("缺少 {f}：tlmgr search --global --file /{f}")
        } else if let Some((what, n, file)) = at {
            let key = lines.get(i + 1).and_then(|x| x.split_once('{')).map(|(_, k)| k.trim().trim_end_matches(',').to_string()).filter(|k| !k.is_empty());
            let what = if what.starts_with("Repeated entry") {
                format!("重複的條目{}", key.map(|k| format!(" {k}")).unwrap_or_default())
            } else if what.starts_with("I was expecting") {
                "格式錯誤（可能少了逗號或括號）".to_string()
            } else {
                what.to_string()
            };
            format!("{file} 第 {n} 行：{what}")
        } else if l.contains("I couldn't open") || l.contains("I found no") || l.starts_with("ERROR") {
            l.trim().to_string()
        } else {
            continue;
        };
        if !v.contains(&msg) {
            v.push(msg);
        }
    }
    v
}

/// Convert to `export/<name>.docx` with Pandoc. A `reference.docx` in the
/// paper folder (copied from the publisher template) sets the Word styles,
/// and a `.csl` file there sets the citation style.
pub async fn build_docx(paper: &Path, name: &str, bibs: &[String]) -> Result<Report, String> {
    let start = Instant::now();
    let pandoc = tool("pandoc").ok_or("pandoc-missing")?;
    let dest = export_dir(paper).map_err(|e| e.to_string())?.join(format!("{name}.docx"));
    let mut c = Command::new(pandoc);
    c.current_dir(paper).args(["main.tex", "--from=latex", "--to=docx", "--resource-path=."]).arg("-o").arg(&dest);
    let bibs: Vec<&String> = bibs.iter().filter(|b| paper.join(b).is_file()).collect();
    if !bibs.is_empty() {
        c.arg("--citeproc");
        for b in bibs {
            c.arg(format!("--bibliography={b}"));
        }
        if let Some(csl) = first_with_ext(paper, "csl") {
            c.arg(format!("--csl={csl}"));
        }
    }
    if paper.join("reference.docx").is_file() {
        c.arg("--reference-doc=reference.docx");
    }
    let o = run(&mut c, 120).await?;
    let stderr = String::from_utf8_lossy(&o.stderr);
    let mut report = Report { seconds: (start.elapsed().as_secs_f64() * 10.0).round() / 10.0, ..Default::default() };
    for l in stderr.lines().filter(|l| !l.trim().is_empty()).take(20) {
        let l = l.trim_start_matches("[WARNING] ").to_string();
        if let Some(k) = l.strip_prefix("Citeproc: citation ").and_then(|r| r.split(' ').next()) {
            report.undefined_cites.push(k.to_string());
        } else if !report.warnings.contains(&l) {
            report.warnings.push(l);
        }
    }
    if o.status.success() {
        report.ok = true;
        report.file = Some(format!("export/{name}.docx"));
    } else {
        report.errors.push(Problem { message: report.warnings.first().cloned().unwrap_or_else(|| "pandoc 失敗".into()), ..Default::default() });
    }
    Ok(report)
}

fn first_with_ext(dir: &Path, ext: &str) -> Option<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).ok()?.flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(&format!(".{ext}")))
        .collect();
    v.sort();
    v.into_iter().next()
}

/// Open a finished export, or show it in the file manager.
pub fn open(path: &Path, reveal: bool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = std::process::Command::new("open");
        if reveal {
            c.arg("-R");
        }
        c.arg(path);
        c
    };
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = std::process::Command::new("explorer");
        if reveal { c.arg(format!("/select,{}", path.display())); } else { c.arg(path); }
        c
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut cmd = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(if reveal { path.parent().unwrap_or(path) } else { path });
        c
    };
    cmd.spawn().map(|_| ()).map_err(|e| format!("無法開啟：{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_engine_and_bibliography_program() {
        assert_eq!(engine_for("\\documentclass{IEEEtran}\n\\begin{document}"), "pdflatex");
        assert_eq!(engine_for("% !TEX program = XeLaTeX\n\\documentclass{article}"), "xelatex");
        assert_eq!(engine_for("\\usepackage{xeCJK}"), "xelatex");
        assert_eq!(engine_for("% \\usepackage{fontspec}"), "pdflatex", "commented out");
        assert_eq!(bib_tool("\\bibliographystyle{IEEEtran}\n\\bibliography{refs}"), Some("bibtex"));
        assert_eq!(bib_tool("\\usepackage[backend=biber]{biblatex}"), Some("biber"));
        assert_eq!(bib_tool("\\usepackage[backend=bibtex]{biblatex}"), Some("bibtex"));
        assert_eq!(bib_tool("% \\bibliography{refs}"), None);
    }

    #[test]
    fn reads_errors_and_warnings_from_the_log() {
        let log = "\
(./main.tex
./sections/02-method.tex:14: Undefined control sequence.
l.14 We use \\foo
                 to measure.
./main.tex:3: LaTeX Error: File `nicetable.sty' not found.
./main.tex:3:  ==> Fatal error occurred, no output PDF file produced!
Overfull \\hbox (3.2pt too wide) in paragraph at lines 5--9
Overfull \\hbox (1.0pt too wide) in paragraph at lines 12--13
LaTeX Warning: Citation `smith2020' on page 1 undefined on input line 7.
Package natbib Warning: Citation `smith2020' on page 2 undefined on input line 9.
LaTeX Warning: Reference `fig:x' on page 1 undefined on input line 4.
LaTeX Warning: There were undefined references.
Output written on /tmp/b/main.pdf (5 pages, 12345 bytes).
";
        let i = parse_log(log);
        assert_eq!(i.errors.len(), 2);
        assert_eq!(i.errors[0].file.as_deref(), Some("sections/02-method.tex"));
        assert_eq!(i.errors[0].line, Some(14));
        assert_eq!(i.errors[0].context.as_deref(), Some("We use \\foo"));
        assert!(i.errors[1].hint.as_deref().unwrap().contains("tlmgr install nicetable"));
        assert_eq!((i.overfull, i.pages, i.rerun), (2, Some(5), true));
        assert_eq!(i.undefined_cites, ["smith2020"]);
        assert_eq!(i.undefined_refs, ["fig:x"]);
        let bare = parse_log("! Emergency stop.\n<*> main.tex\n");
        assert_eq!(bare.errors[0].message, "Emergency stop.");
        assert_eq!(bare.errors[0].file, None);
    }

    #[test]
    fn finds_the_paragraph_of_a_line() {
        let text = "\\section{Method}\n\nFirst paragraph\nstill first.\n\nSecond with \\foo here.\n";
        assert_eq!(para_at("02-method.tex", text, 6), Some(1));
        assert_eq!(para_at("02-method.tex", text, 4), Some(0));
        assert_eq!(para_at("02-method.tex", text, 2), None);
    }

    #[test]
    fn reads_bibtex_warnings() {
        let blg = "Warning--I didn't find a database entry for \"nokey\"\nWarning--I didn't find a database entry for \"nokey\"\nI couldn't open database file refs.bib\n";
        assert_eq!(bib_warnings(blg), ["I couldn't open database file refs.bib"]);
        assert_eq!(bib_warnings("I couldn't open style file IEEEtran.bst\n"), ["缺少 IEEEtran.bst：tlmgr search --global --file /IEEEtran.bst"]);
        let blg = "Repeated entry---line 494 of file references.bib\n : @article{Tropp2007Signal\n :                         ,\nI was expecting a `,' or a `}'---line 512 of file references.bib\n";
        assert_eq!(bib_warnings(blg), ["references.bib 第 494 行：重複的條目 Tropp2007Signal", "references.bib 第 512 行：格式錯誤（可能少了逗號或括號）"]);
    }
}
