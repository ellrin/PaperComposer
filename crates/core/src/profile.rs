use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Version of the profile file layout this build understands.
pub const SCHEMA_VERSION: u32 = 1;

pub const POSITIONS: &[&str] = &["opening", "body", "closing", "transition"];
pub const SECTIONS: &[&str] = &[
    "Abstract",
    "Introduction",
    "Related work",
    "Method",
    "Result",
    "Discussion",
    "Conclusion",
    "Any",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub schema_version: u32,
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_language() -> String {
    "en".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Phrase {
    pub id: String,
    pub section: String,
    pub function: String,
    pub position: String,
    pub template: String,
    #[serde(default)]
    pub example: String,
    /// Semicolon-separated in the CSV.
    #[serde(default, deserialize_with = "split_tags")]
    pub tags: Vec<String>,
    #[serde(default = "default_priority")]
    pub priority: u8,
}

fn default_priority() -> u8 {
    3
}

fn split_tags<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    let s = String::deserialize(d)?;
    Ok(s.split(';')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocabEntry {
    /// The word or phrase to avoid.
    pub avoid: String,
    /// What to write instead (may be empty: just avoid it).
    #[serde(default)]
    pub prefer: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RuleKind {
    /// Checked by code, instantly.
    Hard,
    /// Needs a model; skipped by the core checker.
    Soft,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: RuleKind,
    /// "paragraph", "sentence", or "section:<Name>".
    pub scope: String,
    pub check: String,
    #[serde(default)]
    pub threshold: String,
    #[serde(default = "default_severity")]
    pub severity: String,
    #[serde(default)]
    pub message: String,
}

fn default_severity() -> String {
    "warning".into()
}

/// A problem found while loading a profile, pointing at a file and row.
#[derive(Debug, Clone, Serialize)]
pub struct Issue {
    pub file: String,
    /// 1-based line in the file; 0 when it concerns the whole file.
    pub line: u64,
    pub column: Option<String>,
    pub message: String,
    /// Errors drop the row; warnings keep it.
    pub error: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Profile {
    #[serde(skip)]
    pub root: PathBuf,
    pub manifest: Manifest,
    pub phrases: Vec<Phrase>,
    pub vocabulary: Vec<VocabEntry>,
    pub rules: Vec<Rule>,
}

impl Profile {
    /// Load a profile folder. Never fails on bad rows: those are reported as
    /// issues and skipped, so one typo in a CSV does not lock the user out.
    /// Fails only when the folder or its manifest is unusable.
    pub fn load(root: &Path) -> Result<(Profile, Vec<Issue>), String> {
        let mut issues = Vec::new();
        let manifest_path = root.join("profile.toml");
        let raw = std::fs::read_to_string(&manifest_path)
            .map_err(|e| format!("cannot read {}: {e}", manifest_path.display()))?;
        let manifest: Manifest =
            toml::from_str(&raw).map_err(|e| format!("profile.toml: {e}"))?;
        if manifest.schema_version > SCHEMA_VERSION {
            return Err(format!(
                "profile.toml uses schema_version {} but this build supports up to {}; update the app",
                manifest.schema_version, SCHEMA_VERSION
            ));
        }

        let phrases = read_csv::<Phrase>(root, "phrases.csv", &mut issues)
            .into_iter()
            .filter(|(line, p)| validate_phrase(*line, p, &mut issues))
            .map(|(_, p)| p)
            .collect::<Vec<_>>();
        check_unique_ids(&phrases, &mut issues);

        let vocabulary = read_csv::<VocabEntry>(root, "vocabulary.csv", &mut issues)
            .into_iter()
            .filter(|(line, v)| {
                if v.avoid.trim().is_empty() {
                    issues.push(issue("vocabulary.csv", *line, Some("avoid"), "empty term", true));
                    false
                } else {
                    true
                }
            })
            .map(|(_, v)| v)
            .collect();

        let rules = read_csv::<Rule>(root, "rules.csv", &mut issues)
            .into_iter()
            .map(|(_, r)| r)
            .collect();

        Ok((
            Profile { root: root.to_path_buf(), manifest, phrases, vocabulary, rules },
            issues,
        ))
    }
}

fn read_csv<T: for<'de> Deserialize<'de>>(
    root: &Path,
    file: &str,
    issues: &mut Vec<Issue>,
) -> Vec<(u64, T)> {
    let path = root.join(file);
    if !path.exists() {
        issues.push(issue(file, 0, None, "file not found; treated as empty", false));
        return Vec::new();
    }
    let mut reader = match csv::ReaderBuilder::new().trim(csv::Trim::All).from_path(&path) {
        Ok(r) => r,
        Err(e) => {
            issues.push(issue(file, 0, None, &e.to_string(), true));
            return Vec::new();
        }
    };
    let headers = match reader.headers() {
        Ok(h) => h.clone(),
        Err(e) => {
            issues.push(issue(file, 0, None, &e.to_string(), true));
            return Vec::new();
        }
    };
    let mut out = Vec::new();
    for rec in reader.records() {
        let rec = match rec {
            Ok(r) => r,
            Err(e) => {
                let line = e.position().map(|p| p.line()).unwrap_or(0);
                issues.push(issue(file, line, None, &e.to_string(), true));
                continue;
            }
        };
        let line = rec.position().map(|p| p.line()).unwrap_or(0);
        match rec.deserialize::<T>(Some(&headers)) {
            Ok(v) => out.push((line, v)),
            Err(e) => issues.push(issue(file, line, None, &e.to_string(), true)),
        }
    }
    out
}

fn validate_phrase(line: u64, p: &Phrase, issues: &mut Vec<Issue>) -> bool {
    let f = "phrases.csv";
    let mut ok = true;
    for (col, val) in [
        ("id", &p.id),
        ("section", &p.section),
        ("function", &p.function),
        ("position", &p.position),
        ("template", &p.template),
    ] {
        if val.trim().is_empty() {
            issues.push(issue(f, line, Some(col), "required field is empty", true));
            ok = false;
        }
    }
    if !p.position.is_empty() && !POSITIONS.contains(&p.position.as_str()) {
        issues.push(issue(
            f,
            line,
            Some("position"),
            &format!("unknown position '{}'; use one of {}", p.position, POSITIONS.join(", ")),
            true,
        ));
        ok = false;
    }
    if !p.section.is_empty() && !SECTIONS.contains(&p.section.as_str()) {
        issues.push(issue(
            f,
            line,
            Some("section"),
            &format!("non-standard section '{}'; it still works but will not match the section menu", p.section),
            false,
        ));
    }
    if !(1..=5).contains(&p.priority) {
        issues.push(issue(f, line, Some("priority"), "priority must be 1 to 5", true));
        ok = false;
    }
    if p.template.matches('{').count() != p.template.matches('}').count() {
        issues.push(issue(f, line, Some("template"), "unbalanced { } placeholder", true));
        ok = false;
    }
    ok
}

fn check_unique_ids(phrases: &[Phrase], issues: &mut Vec<Issue>) {
    let mut seen = HashSet::new();
    for p in phrases {
        if !seen.insert(p.id.as_str()) {
            issues.push(issue(
                "phrases.csv",
                0,
                Some("id"),
                &format!("duplicate id '{}'", p.id),
                false,
            ));
        }
    }
}

fn issue(file: &str, line: u64, column: Option<&str>, message: &str, error: bool) -> Issue {
    Issue {
        file: file.to_string(),
        line,
        column: column.map(str::to_string),
        message: message.to_string(),
        error,
    }
}
