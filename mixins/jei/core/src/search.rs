//! Port of `ElementSearch` (per-index string storage and token lookup) and the query evaluation of
//! `IngredientFilter.getIngredientListUncached`: text is lowercased, tokenized, split at `|` into groups; inside a group
//! the positive tokens are intersected and the `-` tokens subtracted; the groups' results are united.
//!
//! JEI stores keys in Lucene and matches them with a literal substring wildcard (`*token*`, case-sensitive). The keys
//! are held here in a plain map and matched with `str::contains`, which gives the same sets.
use crate::prefix::{PREFIXES, PrefixInfo, SearchIndex, SearchMode, parse_token};
use crate::tokenizer::{self, Token};
use std::collections::{HashMap, HashSet};

/// An element's id in the list (its position in the registry).
pub type ElementId = usize;

#[derive(Default)]
struct Storage {
    /// key -> elements with that key (JEI's `LimitedStringStorage` shares the set per key too).
    by_key: HashMap<String, Vec<ElementId>>,
}

impl Storage {
    fn put(&mut self, key: &str, element: ElementId) {
        let key = key.trim();
        if key.is_empty() {
            return;
        }
        let list = self.by_key.entry(key.to_owned()).or_default();
        if list.last() != Some(&element) && !list.contains(&element) {
            list.push(element);
        }
    }
    fn search(&self, token: &str, out: &mut HashSet<ElementId>) {
        for (key, elements) in &self.by_key {
            if key.contains(token) {
                out.extend(elements);
            }
        }
    }
}

/// One parsed token: the text to look for and the index it picked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenInfo {
    pub token: String,
    pub prefix: &'static PrefixInfo,
}

pub struct ElementSearch {
    modes: HashMap<SearchIndex, SearchMode>,
    storages: HashMap<SearchIndex, Storage>,
    all: Vec<ElementId>,
}

impl ElementSearch {
    /// `mode` is each index's configured `SearchMode` (the unprefixed names are always searched).
    pub fn new(mode: impl Fn(SearchIndex) -> SearchMode) -> Self {
        let modes = PREFIXES
            .iter()
            .map(|p| (p.index, if p.index == SearchIndex::NamesAndAliases { SearchMode::Enabled } else { mode(p.index) }))
            .collect();
        Self { modes, storages: HashMap::new(), all: Vec::new() }
    }

    pub fn mode(&self, index: SearchIndex) -> SearchMode {
        self.modes.get(&index).copied().unwrap_or(SearchMode::Disabled)
    }

    /// `ElementSearch.add`: `strings(index)` are the element's keys for that index. Disabled indexes store nothing.
    pub fn add(&mut self, element: ElementId, strings: impl Fn(SearchIndex) -> Vec<String>) {
        self.all.push(element);
        for p in PREFIXES.iter() {
            if self.mode(p.index) == SearchMode::Disabled {
                continue;
            }
            let storage = self.storages.entry(p.index).or_default();
            for s in strings(p.index) {
                storage.put(&s, element);
            }
        }
    }

    pub fn all_elements(&self) -> &[ElementId] {
        &self.all
    }

    /// `ElementPrefixParser.parseToken` with this search's modes.
    pub fn parse_token(&self, token: &str) -> Option<TokenInfo> {
        parse_token(token, |i| self.mode(i)).map(|(t, p)| TokenInfo { token: t.to_owned(), prefix: p })
    }

    /// `ElementSearch.getSearchResults`.
    pub fn results(&self, info: &TokenInfo) -> HashSet<ElementId> {
        let mut out = HashSet::new();
        if info.token.is_empty() {
            return out;
        }
        let combined = |out: &mut HashSet<ElementId>| {
            // `CombinedSearchables`: every index that is `Enabled` (not "require manual prefix").
            for p in PREFIXES.iter() {
                if self.mode(p.index) == SearchMode::Enabled {
                    if let Some(s) = self.storages.get(&p.index) {
                        s.search(&info.token, out);
                    }
                }
            }
        };
        if info.prefix.index == SearchIndex::NamesAndAliases || self.mode(info.prefix.index) == SearchMode::Disabled {
            combined(&mut out);
        } else if let Some(s) = self.storages.get(&info.prefix.index) {
            s.search(&info.token, &mut out);
        }
        out
    }

    /// `IngredientFilter.getIngredientListUncached` up to the sort: the matching elements, each once, in the order the
    /// groups produced them. The caller drops invisible elements and sorts. An empty query is every element.
    pub fn query(&self, filter_text: &str) -> Vec<ElementId> {
        let text = filter_text.to_lowercase();
        let groups: Vec<_> = tokenizer::split_by_operators(tokenizer::tokenize(&text))
            .iter()
            .map(|g| self.parse_group(g))
            .filter(|(s, r)| !(s.is_empty() && r.is_empty()))
            .collect();
        if groups.is_empty() {
            return self.all.clone();
        }
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for (search, remove) in &groups {
            let mut found: Vec<ElementId> = self.group_results(search, remove).into_iter().collect();
            found.sort_unstable(); // a stable order; JEI's is hash order and sorted afterwards anyway
            for e in found {
                if seen.insert(e) {
                    out.push(e);
                }
            }
        }
        out
    }

    fn parse_group(&self, tokens: &[Token]) -> (Vec<TokenInfo>, Vec<TokenInfo>) {
        let (mut search, mut remove) = (Vec::new(), Vec::new());
        for t in tokens {
            if t.is_empty() {
                continue;
            }
            if let Some(info) = self.parse_token(&t.text) {
                if t.exclusion { remove.push(info) } else { search.push(info) }
            }
        }
        (search, remove)
    }

    fn group_results(&self, search: &[TokenInfo], remove: &[TokenInfo]) -> HashSet<ElementId> {
        let sets: Vec<HashSet<ElementId>> = search.iter().map(|t| self.results(t)).collect();
        let mut results = match sets.iter().min_by_key(|s| s.len()) {
            None => HashSet::new(),
            Some(smallest) => {
                let mut r = smallest.clone();
                for s in &sets {
                    r.retain(|e| s.contains(e));
                }
                r
            }
        };
        if results.is_empty() && !remove.is_empty() {
            results.extend(self.all.iter().copied());
        }
        if !results.is_empty() {
            for t in remove {
                for e in self.results(t) {
                    results.remove(&e);
                }
                if results.is_empty() {
                    break;
                }
            }
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(i: SearchIndex, names: &[&str], mods: &[&str], e: usize) -> Vec<String> {
        match i {
            SearchIndex::NamesAndAliases => vec![names[e].to_string()],
            SearchIndex::ModNames => vec![mods[e].to_string()],
            _ => vec![],
        }
    }

    fn build(mode: impl Fn(SearchIndex) -> SearchMode) -> ElementSearch {
        let n = ["iron ingot", "gold ingot", "iron ore", "osmium ingot"];
        let m = ["minecraft", "minecraft", "minecraft", "mekanism"];
        let mut s = ElementSearch::new(mode);
        for e in 0..4 {
            s.add(e, |i| names(i, &n, &m, e));
        }
        s
    }

    #[test]
    fn substring_and_intersection() {
        let s = build(|_| SearchMode::Enabled);
        assert_eq!(s.query("ingot"), vec![0, 1, 3]);
        assert_eq!(s.query("iron ingot"), vec![0]);
        assert_eq!(s.query("IRON"), vec![0, 2]);
    }

    #[test]
    fn exclusion_or_and_empty() {
        let s = build(|_| SearchMode::Enabled);
        assert_eq!(s.query("ingot -iron"), vec![1, 3]);
        assert_eq!(s.query("-iron"), vec![1, 3]);
        assert_eq!(s.query("gold | osmium"), vec![1, 3]);
        assert_eq!(s.query(""), vec![0, 1, 2, 3]);
        assert!(s.query("nothing").is_empty());
    }

    #[test]
    fn prefix_and_modes() {
        let s = build(|_| SearchMode::Enabled);
        assert_eq!(s.query("@mekanism"), vec![3]);
        assert_eq!(s.query("@mekanism ingot"), vec![3]);
        // Mod names on "require manual prefix": plain text no longer finds them, `@` does.
        let s = build(|i| if i == SearchIndex::ModNames { SearchMode::RequireManualPrefix } else { SearchMode::Enabled });
        assert!(s.query("mekanism").is_empty());
        assert_eq!(s.query("@mekanism"), vec![3]);
        // Disabled: `@x` is plain text and searches the names.
        let s = build(|i| if i == SearchIndex::ModNames { SearchMode::Disabled } else { SearchMode::Enabled });
        assert!(s.query("@mekanism").is_empty());
    }
}
