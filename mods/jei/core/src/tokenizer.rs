//! Port of `mezz.jei.gui.search.SearchTokenizer` and `Token`: splits search text into tokens with quotes (`"a b"`),
//! escapes (`\x`), exclusion (`-x`) and the or-operator (`|`). Positions are in UTF-16 units like Java's `char`s, so the
//! highlighter and completion overlay ported later index the text exactly as JEI does.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub text: String,
    pub exclusion: bool,
    pub operator: bool,
    pub start: usize,
    pub end: usize,
}

impl Token {
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
    pub fn operator(start: usize) -> Token {
        Token { text: "|".into(), exclusion: false, operator: true, start, end: start + 1 }
    }
}

/// `Character.isWhitespace` for the code points a search box can hold.
fn is_whitespace(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | '\u{1c}'..='\u{1f}')
        || (c.is_whitespace() && !matches!(c, '\u{a0}' | '\u{2007}' | '\u{202f}'))
}

fn is_prefix(text: &str) -> bool {
    let mut it = text.chars();
    match (it.next(), it.next()) {
        (Some(c), None) => !c.is_alphanumeric() && c != '-' && c != '|',
        _ => false,
    }
}

fn add_token(tokens: &mut Vec<Token>, content: &str, exclusion: bool, start: usize, end: usize) {
    let text = content.trim();
    if !text.is_empty() {
        tokens.push(Token { text: text.to_owned(), exclusion, operator: false, start, end });
    }
}

/// `SearchTokenizer.tokenize`.
pub fn tokenize(filter_text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    if filter_text.is_empty() {
        return tokens;
    }
    let mut current = String::new();
    let (mut inside_quotes, mut exclusion, mut escaped) = (false, false, false);
    let (mut token_start, mut token_end): (Option<usize>, usize) = (None, 0);
    // `i` counts UTF-16 units, as Java's `charAt` index does.
    let mut i = 0usize;
    for c in filter_text.chars() {
        let width = c.len_utf16();
        let at = i;
        i += width;
        if escaped {
            current.push(c);
            token_end = at + width;
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            token_start.get_or_insert(at);
            token_end = at + 1;
            continue;
        }
        if c == '"' {
            if inside_quotes {
                add_token(&mut tokens, &current, exclusion, token_start.unwrap_or(usize::MAX), at + 1);
                current.clear();
                inside_quotes = false;
                exclusion = false;
                token_start = None;
                token_end = 0;
            } else if !current.is_empty() && !is_prefix(&current) {
                add_token(&mut tokens, &current, exclusion, token_start.unwrap_or(usize::MAX), token_end);
                current.clear();
                exclusion = false;
                token_start = None;
                token_end = 0;
            } else {
                inside_quotes = true;
                token_start.get_or_insert(at);
                token_end = at + 1;
            }
            continue;
        }
        if !inside_quotes && is_whitespace(c) {
            if !current.is_empty() {
                add_token(&mut tokens, &current, exclusion, token_start.unwrap_or(usize::MAX), token_end);
                current.clear();
            }
            exclusion = false;
            token_start = None;
            token_end = 0;
            continue;
        }
        if !inside_quotes && c == '|' {
            if !current.is_empty() {
                add_token(&mut tokens, &current, exclusion, token_start.unwrap_or(usize::MAX), token_end);
                current.clear();
            }
            tokens.push(Token::operator(at));
            exclusion = false;
            token_start = None;
            token_end = 0;
            continue;
        }
        if !inside_quotes && current.is_empty() && c == '-' {
            exclusion = true;
            token_start = Some(at);
            token_end = at + 1;
            continue;
        }
        token_start.get_or_insert(at);
        token_end = at + width;
        current.push(c);
    }
    if !current.is_empty() || inside_quotes {
        add_token(&mut tokens, &current, exclusion, token_start.unwrap_or(usize::MAX), token_end);
    }
    tokens
}

/// `SearchTokenizer.splitByOperators`: the groups between `|` tokens (always at least one, maybe empty).
pub fn split_by_operators(tokens: Vec<Token>) -> Vec<Vec<Token>> {
    let mut groups = Vec::new();
    let mut current = Vec::new();
    for token in tokens {
        if token.operator {
            groups.push(std::mem::take(&mut current));
        } else {
            current.push(token);
        }
    }
    groups.push(current);
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(s: &str) -> Vec<(String, bool)> {
        tokenize(s).into_iter().map(|t| (t.text, t.exclusion)).collect()
    }

    #[test]
    fn words_and_exclusion() {
        assert_eq!(texts("iron -ingot"), vec![("iron".into(), false), ("ingot".into(), true)]);
        assert!(tokenize("").is_empty());
    }

    #[test]
    fn quotes_keep_spaces_and_positions() {
        let t = tokenize("\"iron ingot\" gold");
        assert_eq!(t[0].text, "iron ingot");
        assert_eq!((t[0].start, t[0].end), (0, 12));
        assert_eq!(t[1].text, "gold");
    }

    #[test]
    fn prefix_followed_by_quote_opens_quote() {
        // `@"my mod"`: a one-character prefix keeps the quote part of the same token.
        assert_eq!(texts("@\"my mod\""), vec![("@my mod".into(), false)]);
    }

    #[test]
    fn word_before_quote_is_flushed() {
        assert_eq!(texts("ab\"cd\""), vec![("ab".into(), false), ("cd".into(), false)]);
    }

    #[test]
    fn operator_splits_groups() {
        let g = split_by_operators(tokenize("a b | c"));
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].len(), 2);
        assert_eq!(g[1][0].text, "c");
        assert_eq!(split_by_operators(vec![]).len(), 1);
    }

    #[test]
    fn escapes() {
        assert_eq!(texts("a\\ b"), vec![("a b".into(), false)]);
        assert_eq!(texts("\\-x"), vec![("-x".into(), false)]);
    }

    #[test]
    fn utf16_positions() {
        let t = tokenize("\u{1F600}a b");
        assert_eq!((t[0].start, t[0].end), (0, 3));
        assert_eq!((t[1].start, t[1].end), (4, 5));
    }
}
