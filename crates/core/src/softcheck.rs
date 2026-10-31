//! Soft-rule checks: a model reads one paragraph against the soft rules in
//! rules.csv (type = soft) and answers pass or fail for each, pointing at the
//! sentence at fault. The rules are the author's own; the model only judges
//! them, it never rewrites the paragraph.

use crate::check::{applies, Severity};
use crate::compose::Prompt;
use crate::profile::{Profile, Rule, RuleKind};
use crate::text::sentences;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct SoftResult {
    pub rule: String,
    pub severity: Severity,
    pub message: String,
    pub pass: bool,
    /// 0-based sentence index within the paragraph, when it concerns one.
    pub sentence: Option<usize>,
    /// One short sentence from the model.
    pub reason: String,
}

/// Soft rules that apply to a paragraph in `section`.
pub fn soft_rules<'a>(profile: &'a Profile, section: Option<&str>) -> Vec<&'a Rule> {
    profile
        .rules
        .iter()
        .filter(|r| r.kind == RuleKind::Soft && applies(r, "paragraph", section))
        .collect()
}

pub fn build_prompt(rules: &[&Rule], paragraph: &str, section: Option<&str>, before: Option<&str>, after: Option<&str>) -> Prompt {
    let system = "You check one paragraph of an academic paper against the author's own writing rules. \
Judge each listed rule strictly as written. Do not rewrite the paragraph, do not suggest new text, do not add rules. \
Reply with JSON only, in this shape: \
{\"results\":[{\"rule\":\"R101\",\"pass\":true,\"sentence\":null,\"reason\":\"...\"}]}. \
Give one result per listed rule. `sentence` is the number of the sentence the problem is in, or null when it concerns the whole paragraph or the rule passes. \
`reason` is one short sentence in Traditional Chinese (繁體中文) saying why it passes or fails."
        .to_string();

    let mut user = String::new();
    if let Some(s) = section {
        user += &format!("Section: {s}\n");
    }
    user += "\nRules:\n";
    for r in rules {
        let what = if r.message.is_empty() { String::new() } else { format!(": {}", r.message) };
        user += &format!("- {} ({}){what}\n", r.id, r.check);
    }
    if let Some(b) = before.filter(|s| !s.trim().is_empty()) {
        user += &format!("\nPrevious paragraph (context only):\n{b}\n");
    }
    user += "\nParagraph to check, one numbered sentence per line:\n";
    for (i, s) in sentences(paragraph).iter().enumerate() {
        user += &format!("{}. {s}\n", i + 1);
    }
    if let Some(a) = after.filter(|s| !s.trim().is_empty()) {
        user += &format!("\nNext paragraph (context only):\n{a}\n");
    }
    Prompt { system, user }
}

#[derive(Deserialize)]
struct Reply {
    results: Vec<Raw>,
}
#[derive(Deserialize)]
struct Raw {
    rule: String,
    pass: bool,
    #[serde(default)]
    sentence: Option<usize>,
    #[serde(default)]
    reason: String,
}

/// Parse the model's reply. Results for rules that were not asked about are
/// dropped; sentence numbers outside the paragraph are cleared.
pub fn parse_reply(reply: &str, rules: &[&Rule], paragraph: &str) -> Result<Vec<SoftResult>, String> {
    let start = reply.find('{').ok_or("no JSON object in reply")?;
    let end = reply.rfind('}').ok_or("no JSON object in reply")?;
    let r: Reply = serde_json::from_str(&reply[start..=end]).map_err(|e| e.to_string())?;
    let n = sentences(paragraph).len();
    let out: Vec<SoftResult> = rules
        .iter()
        .filter_map(|rule| {
            let raw = r.results.iter().find(|x| x.rule.trim() == rule.id)?;
            Some(SoftResult {
                rule: rule.id.clone(),
                severity: Severity::parse(&rule.severity),
                message: if rule.message.is_empty() { rule.check.clone() } else { rule.message.clone() },
                pass: raw.pass,
                sentence: raw.sentence.filter(|&s| !raw.pass && s >= 1 && s <= n).map(|s| s - 1),
                reason: raw.reason.trim().to_string(),
            })
        })
        .collect();
    if out.is_empty() && !rules.is_empty() {
        return Err("reply has no results for the listed rules".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(id: &str, check: &str, scope: &str) -> Rule {
        Rule {
            id: id.into(),
            kind: RuleKind::Soft,
            scope: scope.into(),
            check: check.into(),
            threshold: String::new(),
            severity: "error".into(),
            message: format!("{check} message"),
        }
    }

    #[test]
    fn prompt_numbers_sentences_and_lists_rules() {
        let a = rule("R101", "has_topic_sentence", "paragraph");
        let p = build_prompt(&[&a], "First idea. Second idea.", Some("Result"), Some("Before."), None);
        assert!(p.user.contains("- R101 (has_topic_sentence): has_topic_sentence message"));
        assert!(p.user.contains("1. First idea.\n2. Second idea.\n"), "{}", p.user);
        assert!(p.user.contains("Previous paragraph"));
        assert!(!p.user.contains("Next paragraph"));
    }

    #[test]
    fn parses_reply_and_keeps_only_asked_rules() {
        let a = rule("R101", "has_topic_sentence", "paragraph");
        let b = rule("R102", "claim_has_evidence", "paragraph");
        let reply = "```json\n{\"results\":[{\"rule\":\"R101\",\"pass\":true,\"sentence\":null,\"reason\":\"ok\"},\
{\"rule\":\"R102\",\"pass\":false,\"sentence\":2,\"reason\":\"沒有引用\"},\
{\"rule\":\"R999\",\"pass\":false,\"sentence\":1,\"reason\":\"invented\"}]}\n```";
        let out = parse_reply(reply, &[&a, &b], "One. Two.").unwrap();
        assert_eq!(out.len(), 2);
        assert!(out[0].pass && out[0].sentence.is_none());
        assert!(!out[1].pass);
        assert_eq!(out[1].sentence, Some(1), "1-based in the reply, 0-based out");
        assert_eq!(out[1].reason, "沒有引用");
    }

    #[test]
    fn out_of_range_sentence_is_cleared() {
        let a = rule("R104", "one_complete_idea", "paragraph");
        let out = parse_reply("{\"results\":[{\"rule\":\"R104\",\"pass\":false,\"sentence\":9}]}", &[&a], "Only one.").unwrap();
        assert_eq!(out[0].sentence, None);
    }
}
