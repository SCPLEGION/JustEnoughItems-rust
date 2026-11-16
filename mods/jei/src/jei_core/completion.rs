//! `SearchCompletionOverlay`'s candidates: what the search box can complete at the cursor (prefixes, the `-` and `|`
//! operators, and the strings of the indexes that complete dynamically).

use crate::jei_core::prefix::SearchMode;
use crate::jei_core::tokenizer::{tokenize, Token};

/// `DYNAMIC_CANDIDATE_BATCH_SIZE`: how many dynamic candidates are listed before "more" is asked for.
pub const DYNAMIC_BATCH: usize = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Prefix,
    Operator,
    Dynamic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// The text that replaces the current token.
    pub insertion: String,
    pub display: String,
    /// A lang key (the prefix's or the operator's description).
    pub description: String,
    pub category: Category,
}

/// One search index as the completion sees it.
#[derive(Clone, Debug)]
pub struct PrefixView {
    /// `'\0'` for the unprefixed names.
    pub prefix: char,
    pub mode: SearchMode,
    pub dynamic: bool,
    pub description: String,
    /// Every string of the index, sorted and unique (`getDynamicStrings`).
    pub strings: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Completion {
    pub candidates: Vec<Candidate>,
    /// More dynamic candidates exist than `limit` listed.
    pub truncated: bool,
}

fn utf16_slice(text: &str, start: usize, end: usize) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    String::from_utf16_lossy(&units[start.min(units.len())..end.min(units.len())])
}

fn token_start(tokens: &[Token], cursor: usize) -> usize {
    tokens.iter().find(|t| cursor >= t.start && cursor <= t.end).map_or(cursor, |t| t.start)
}

fn current_token(tokens: &[Token], text: &str, cursor: usize) -> String {
    if cursor == 0 {
        return String::new();
    }
    let token = utf16_slice(text, token_start(tokens, cursor), cursor);
    match token.strip_prefix('-') {
        Some(rest) if !rest.is_empty() => rest.to_owned(),
        _ => token,
    }
}

fn has_predicate_before(tokens: &[Token], cursor: usize) -> bool {
    let mut found = false;
    for t in tokens {
        if cursor >= t.start && cursor < t.end {
            return found;
        }
        if !t.operator {
            found = true;
        }
    }
    found
}

fn strip_matching_quotes(s: &str) -> &str {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s.strip_prefix('"').unwrap_or(s)
    }
}

fn add_prefixes(out: &mut Vec<Candidate>, views: &[PrefixView], token: &str) {
    for v in views {
        if v.mode == SearchMode::Disabled || v.prefix == '\0' {
            continue;
        }
        let p = v.prefix.to_string();
        if p.starts_with(token) {
            out.push(Candidate { insertion: p.clone(), display: p, description: v.description.clone(), category: Category::Prefix });
        }
    }
}

fn add_static(out: &mut Vec<Candidate>, views: &[PrefixView], token: &str, predicate_before: bool) {
    add_prefixes(out, views, token);
    if "-".starts_with(token) {
        out.push(Candidate { insertion: "-".into(), display: "-".into(), description: "jei.search.completion.operator.not".into(), category: Category::Operator });
    }
    if predicate_before && token.is_empty() {
        out.push(Candidate { insertion: "|".into(), display: "|".into(), description: "jei.search.completion.operator.or".into(), category: Category::Operator });
    }
}

/// The candidates for `text` with the cursor at `cursor` (UTF-16 units), listing at most `limit` dynamic ones.
pub fn candidates(text: &str, cursor: usize, views: &[PrefixView], limit: usize) -> Completion {
    let mut out = Vec::new();
    let mut truncated = false;
    if text.trim().is_empty() && !text.is_empty() {
        return Completion::default();
    }
    let tokens = tokenize(text);
    let token = current_token(&tokens, text, cursor);
    let predicate_before = has_predicate_before(&tokens, cursor);
    if text.is_empty() {
        add_prefixes(&mut out, views, &token);
    } else if !token.is_empty() {
        let first = token.chars().next().unwrap_or('\0');
        match views.iter().find(|v| v.prefix == first && first != '\0') {
            Some(v) if v.mode != SearchMode::Disabled => {
                if v.dynamic {
                    let query = strip_matching_quotes(&token[first.len_utf8()..]).to_lowercase();
                    let quoted = token[first.len_utf8()..].starts_with('"');
                    let mut count = 0;
                    for s in &v.strings {
                        if !s.to_lowercase().starts_with(&query) {
                            continue;
                        }
                        let insertion = if quoted { format!("{first}\"{s}\"") } else { format!("{first}{s}") };
                        if insertion == token {
                            continue;
                        }
                        if count >= limit {
                            truncated = true;
                            break;
                        }
                        out.push(Candidate { insertion, display: s.clone(), description: v.description.clone(), category: Category::Dynamic });
                        count += 1;
                    }
                } else {
                    add_static(&mut out, views, &token, predicate_before);
                }
            }
            _ => {
                add_static(&mut out, views, &token, predicate_before);
                // `addNoPrefixDynamicCandidates`: the strings of every enabled dynamic index, once each.
                let query = strip_matching_quotes(&token).to_lowercase();
                let quoted = token.starts_with('"');
                let mut seen: Vec<String> = Vec::new();
                let mut dynamic = Vec::new();
                'outer: for v in views.iter().filter(|v| v.mode == SearchMode::Enabled && v.dynamic) {
                    for s in &v.strings {
                        if s.to_lowercase().starts_with(&query) && !seen.contains(s) {
                            if seen.len() >= limit {
                                truncated = true;
                                break 'outer;
                            }
                            seen.push(s.clone());
                            let insertion = if quoted { format!("\"{s}\"") } else { s.clone() };
                            dynamic.push(Candidate { insertion, display: s.clone(), description: v.description.clone(), category: Category::Dynamic });
                        }
                    }
                }
                out.extend(dynamic);
            }
        }
    } else {
        add_static(&mut out, views, &token, predicate_before);
    }
    Completion { candidates: out, truncated }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn views() -> Vec<PrefixView> {
        let v = |prefix, dynamic, strings: &[&str]| PrefixView {
            prefix,
            mode: if prefix == '\0' { SearchMode::Enabled } else { SearchMode::RequireManualPrefix },
            dynamic,
            description: format!("d{prefix}"),
            strings: strings.iter().map(|s| s.to_string()).collect(),
        };
        vec![v('\0', true, &["dirt", "diamond"]), v('@', true, &["minecraft", "mekanism"]), v('$', false, &[])]
    }

    fn insertions(c: &Completion) -> Vec<&str> {
        c.candidates.iter().map(|c| c.insertion.as_str()).collect()
    }

    #[test]
    fn empty_text_lists_prefixes() {
        let c = candidates("", 0, &views(), 100);
        assert_eq!(insertions(&c), ["@", "$"]);
    }

    #[test]
    fn prefixed_token_completes_dynamically() {
        let c = candidates("@me", 3, &views(), 100);
        assert_eq!(insertions(&c), ["@mekanism"]);
        assert_eq!(c.candidates[0].category, Category::Dynamic);
    }

    #[test]
    fn quoted_prefixed_token_keeps_quotes() {
        let c = candidates("@\"mi", 4, &views(), 100);
        assert_eq!(insertions(&c), ["@\"minecraft\""]);
    }

    #[test]
    fn plain_token_offers_enabled_index_strings() {
        let c = candidates("di", 2, &views(), 100);
        assert_eq!(insertions(&c), ["dirt", "diamond"]);
    }

    #[test]
    fn after_a_predicate_the_or_operator_is_offered() {
        let c = candidates("dirt ", 5, &views(), 100);
        assert!(insertions(&c).contains(&"|"));
        assert!(insertions(&c).contains(&"-"));
    }

    #[test]
    fn exact_match_is_not_offered_and_limit_truncates() {
        let c = candidates("@minecraft", 10, &views(), 100);
        assert!(c.candidates.is_empty());
        let c = candidates("di", 2, &views(), 1);
        assert_eq!(c.candidates.len(), 1);
        assert!(c.truncated);
    }

    #[test]
    fn disabled_prefix_is_not_listed() {
        let mut v = views();
        v[1].mode = SearchMode::Disabled;
        assert_eq!(insertions(&candidates("", 0, &v, 100)), ["$"]);
    }
}
