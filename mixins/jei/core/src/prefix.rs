//! Port of `ElementPrefixParser`'s prefix table and `parseToken`: which first character of a search token picks which
//! index (mod names `@`, tags `#`, tooltips `$`, creative tabs `%`, colours `^`, identifiers `&`).

/// `SearchMode`: how an index takes part in a search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchMode {
    Disabled,
    /// Searched only when its prefix is typed.
    RequireManualPrefix,
    Enabled,
}

/// `ElementSearchIndex`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SearchIndex {
    NamesAndAliases,
    ModNames,
    Tags,
    Tooltips,
    CreativeTabs,
    Colors,
    Identifiers,
}

/// One row of the prefix table: the key JEI's `PrefixInfo` has, minus the storage and string getters (ported with
/// the ingredient list).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrefixInfo {
    pub index: SearchIndex,
    pub id: &'static str,
    /// `'\0'` for the unprefixed names.
    pub prefix: char,
    pub supports_dynamic_completion: bool,
    /// The lang key of `Component.translatable(..)` (empty for the unprefixed).
    pub description_key: &'static str,
}

pub const PREFIXES: [PrefixInfo; 7] = [
    PrefixInfo { index: SearchIndex::NamesAndAliases, id: "unprefixed", prefix: '\0', supports_dynamic_completion: true, description_key: "" },
    PrefixInfo { index: SearchIndex::ModNames, id: "mod_names", prefix: '@', supports_dynamic_completion: true, description_key: "jei.search.completion.prefix.modName" },
    PrefixInfo { index: SearchIndex::Tags, id: "tags", prefix: '#', supports_dynamic_completion: true, description_key: "jei.search.completion.prefix.tag" },
    PrefixInfo { index: SearchIndex::Tooltips, id: "tooltips", prefix: '$', supports_dynamic_completion: false, description_key: "jei.search.completion.prefix.tooltip" },
    PrefixInfo { index: SearchIndex::CreativeTabs, id: "creative_tabs", prefix: '%', supports_dynamic_completion: true, description_key: "jei.search.completion.prefix.creativeTab" },
    PrefixInfo { index: SearchIndex::Colors, id: "colors", prefix: '^', supports_dynamic_completion: true, description_key: "jei.search.completion.prefix.color" },
    PrefixInfo { index: SearchIndex::Identifiers, id: "identifiers", prefix: '&', supports_dynamic_completion: true, description_key: "jei.search.completion.prefix.identifier" },
];

pub fn no_prefix() -> &'static PrefixInfo {
    &PREFIXES[0]
}

/// `ElementPrefixParser.parseToken`. `mode` gives each index's configured mode (the unprefixed one is always enabled).
/// `None` is JEI's empty Optional: no token, or a lone prefix character.
pub fn parse_token<'a>(token: &'a str, mode: impl Fn(SearchIndex) -> SearchMode) -> Option<(&'a str, &'static PrefixInfo)> {
    let first = token.chars().next()?;
    let info = PREFIXES[1..].iter().find(|p| p.prefix == first);
    match info {
        Some(info) if mode(info.index) != SearchMode::Disabled => {
            let rest = &token[first.len_utf8()..];
            if rest.is_empty() { None } else { Some((rest, info)) }
        }
        _ => Some((token, no_prefix())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on(_: SearchIndex) -> SearchMode {
        SearchMode::Enabled
    }

    #[test]
    fn prefix_picks_index() {
        let (rest, info) = parse_token("@mekanism", on).unwrap();
        assert_eq!((rest, info.index), ("mekanism", SearchIndex::ModNames));
        assert_eq!(parse_token("iron", on).unwrap().1.index, SearchIndex::NamesAndAliases);
    }

    #[test]
    fn lone_prefix_is_nothing() {
        assert!(parse_token("#", on).is_none());
        assert!(parse_token("", on).is_none());
    }

    #[test]
    fn disabled_prefix_is_plain_text() {
        let off = |i| if i == SearchIndex::Tags { SearchMode::Disabled } else { SearchMode::Enabled };
        let (rest, info) = parse_token("#ore", off).unwrap();
        assert_eq!((rest, info.index), ("#ore", SearchIndex::NamesAndAliases));
    }
}
