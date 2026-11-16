//! `SearchSyntaxHighlighter`: the colour of each character of the search text.

use crate::jei_core::tokenizer::{tokenize, Token};

pub const PREFIX_COLOR: u32 = 0xFF55FFFF;
pub const PREFIXED_CONTENT_COLOR: u32 = 0xFF55FF55;
pub const OPERATOR_COLOR: u32 = 0xFFFFAA00;
pub const DEFAULT_CONTENT_COLOR: u32 = 0xFFFFFFFF;
pub const NO_MATCH_COLOR: u32 = 0xFFFF0000;

/// One colour per UTF-16 unit of `text` (the positions the tokenizer uses). `no_results` colours it all red;
/// `prefixes` are the prefix characters of the indexes that are not disabled.
pub fn colors(text: &str, no_results: bool, prefixes: &[char]) -> Vec<u32> {
    let units: Vec<u16> = text.encode_utf16().collect();
    if no_results {
        return vec![NO_MATCH_COLOR; units.len()];
    }
    let mut colors = vec![DEFAULT_CONTENT_COLOR; units.len()];
    for token in tokenize(text) {
        color_token(&units, &token, &mut colors, prefixes);
    }
    colors
}

fn color_token(units: &[u16], token: &Token, colors: &mut [u32], prefixes: &[char]) {
    let (start, end) = (token.start.min(units.len()), token.end.min(units.len()));
    if token.operator {
        colors[start..end].fill(OPERATOR_COLOR);
        return;
    }
    let mut pos = start;
    if pos < end && units[pos] == u16::from(b'-') {
        colors[pos] = OPERATOR_COLOR;
        pos += 1;
    }
    if pos < end && char::from_u32(u32::from(units[pos])).is_some_and(|c| prefixes.contains(&c)) {
        colors[pos] = PREFIX_COLOR;
        pos += 1;
        let color = if pos < end && units[pos] == u16::from(b'"') { DEFAULT_CONTENT_COLOR } else { PREFIXED_CONTENT_COLOR };
        colors[pos.min(end)..end].fill(color);
        return;
    }
    colors[pos.min(end)..end].fill(DEFAULT_CONTENT_COLOR);
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: [char; 2] = ['@', '#'];

    #[test]
    fn colors_prefix_operator_and_content() {
        let c = colors("-@mek | dirt", false, &P);
        assert_eq!(c[0], OPERATOR_COLOR);
        assert_eq!(c[1], PREFIX_COLOR);
        assert_eq!(c[2], PREFIXED_CONTENT_COLOR);
        assert_eq!(c[6], OPERATOR_COLOR);
        assert_eq!(c[8], DEFAULT_CONTENT_COLOR);
    }

    #[test]
    fn no_results_is_all_red() {
        assert!(colors("zzz", true, &P).iter().all(|&c| c == NO_MATCH_COLOR));
    }

    #[test]
    fn disabled_prefix_is_plain_text() {
        let c = colors("#ore", false, &['@']);
        assert!(c.iter().all(|&x| x == DEFAULT_CONTENT_COLOR));
    }

    #[test]
    fn quoted_prefixed_content_is_default() {
        let c = colors("@\"a b\"", false, &P);
        assert_eq!(c[0], PREFIX_COLOR);
        assert_eq!(c[1], DEFAULT_CONTENT_COLOR);
    }
}
