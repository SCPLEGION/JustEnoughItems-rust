//! Port of the game-free parts of `ListElementInfo`: the strings an ingredient is searched by (names and aliases, mod
//! names with ids/aliases/short names, tags, tooltip lines, colour names), built from values the glue reads from the game.
use crate::prefix::SearchIndex;
use std::collections::BTreeSet;

/// The `IIngredientFilterConfig` switches `ListElementInfo` reads.
#[derive(Clone, Copy, Debug)]
pub struct FilterConfig {
    pub search_ingredient_aliases: bool,
    pub search_mod_ids: bool,
    pub search_mod_aliases: bool,
    pub search_short_mod_names: bool,
}

impl Default for FilterConfig {
    /// JEI's defaults.
    fn default() -> Self {
        Self { search_ingredient_aliases: true, search_mod_ids: true, search_mod_aliases: true, search_short_mod_names: true }
    }
}

/// `ChatFormatting.stripFormatting`: drops each `§` and the formatting code after it.
pub fn strip_formatting(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\u{a7}' {
            if let Some(&n) = it.peek() {
                if n.is_ascii_digit() || matches!(n.to_ascii_lowercase(), 'a'..='f' | 'k'..='o' | 'r') {
                    it.next();
                    continue;
                }
            }
        }
        out.push(c);
    }
    out
}

/// `DisplayNameUtil.getLowercaseDisplayNameForSearch`.
pub fn lowercase_display_name(display_name: &str) -> String {
    strip_formatting(display_name).to_lowercase()
}

/// `MOD_NAME_SEPARATOR_PATTERN.split(name)` (`(?=[A-Z_-])|\s+`), with Java's rules: no empty leading piece from a
/// zero-width match, trailing empty pieces removed.
fn split_mod_name(name: &str) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut chars = name.chars().peekable();
    let mut index = 0usize;
    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            parts.push(std::mem::take(&mut cur));
            while chars.peek().is_some_and(|n| n.is_whitespace()) {
                chars.next();
            }
            index += 1;
            continue;
        }
        if (c.is_ascii_uppercase() || c == '_' || c == '-') && index > 0 {
            // zero-width split before this char
            if !cur.is_empty() {
                parts.push(std::mem::take(&mut cur));
            }
        }
        cur.push(c);
        index += 1;
    }
    parts.push(cur);
    while parts.last().is_some_and(|p| p.is_empty()) {
        parts.pop();
    }
    parts
}

fn combine_first_letters(words: &[String], count: usize) -> String {
    words.iter().map(|w| w.chars().take(count).collect::<String>()).collect()
}

/// `ListElementInfo.getShortModNames`.
pub fn short_mod_names(mod_name: &str) -> Vec<String> {
    let words = split_mod_name(mod_name);
    if words.len() <= 1 {
        return vec![];
    }
    vec![combine_first_letters(&words, 1), combine_first_letters(&words, 2)]
}

/// What the glue reads from the game for one ingredient.
#[derive(Clone, Debug, Default)]
pub struct IngredientData {
    /// `IIngredientHelper.getDisplayName`, formatting codes allowed.
    pub display_name: String,
    /// The ingredient's aliases (`IIngredientManager.getIngredientAliases`).
    pub aliases: Vec<String>,
    /// Namespace and path of `getIdentifier`.
    pub namespace: String,
    pub path: String,
    /// `getDisplayModId`.
    pub display_mod_id: String,
    /// Mod display name for the namespace, and for the display mod id.
    pub mod_name: String,
    pub display_mod_name: String,
    /// `modIdHelper.getModAliases` for each of the mod ids (namespace first).
    pub mod_aliases: Vec<Vec<String>>,
    /// `getTagStream` ids as `namespace:path`.
    pub tags: Vec<String>,
    /// The plain tooltip lines used for search.
    pub tooltip: Vec<String>,
    /// Names of the closest colour of each of the ingredient's colours.
    pub color_names: Vec<String>,
    /// Display names of the creative tabs holding the item.
    pub creative_tabs: Vec<String>,
}

/// `ListElementInfo`: the search strings of one ingredient.
#[derive(Clone, Debug)]
pub struct ListElementInfo {
    pub data: IngredientData,
    /// The display name first, then the aliases (all lowercase).
    names: Vec<String>,
    pub created_index: usize,
}

impl ListElementInfo {
    pub fn new(data: IngredientData, config: &FilterConfig, created_index: usize) -> Self {
        let first = lowercase_display_name(&data.display_name);
        let mut info = Self { data, names: vec![first], created_index };
        info.refresh_aliases(config);
        info
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }

    /// `refreshIngredientAliases`: names again from the display name and the config.
    pub fn refresh_aliases(&mut self, config: &FilterConfig) {
        self.names.truncate(1);
        if config.search_ingredient_aliases {
            self.names.extend(self.data.aliases.iter().map(|a| a.to_lowercase()));
        }
    }

    fn mod_ids(&self) -> Vec<&str> {
        let d = &self.data;
        if d.namespace == d.display_mod_id { vec![&d.namespace] } else { vec![&d.namespace, &d.display_mod_id] }
    }

    fn mod_names_list(&self) -> Vec<&str> {
        let d = &self.data;
        if d.namespace == d.display_mod_id { vec![&d.mod_name] } else { vec![&d.mod_name, &d.display_mod_name] }
    }

    pub fn mod_name_for_sorting(&self) -> &str {
        self.mod_names_list()[0]
    }

    /// `getModNames(config)`.
    pub fn mod_names(&self, config: &FilterConfig) -> BTreeSet<String> {
        let mut names: BTreeSet<String> = self.mod_names_list().iter().map(|s| s.to_string()).collect();
        if config.search_mod_ids {
            names.extend(self.mod_ids().iter().map(|s| s.to_string()));
        }
        if config.search_mod_aliases {
            for aliases in self.data.mod_aliases.iter().take(self.mod_ids().len()) {
                names.extend(aliases.iter().cloned());
            }
        }
        if config.search_short_mod_names {
            for n in self.mod_names_list() {
                names.extend(short_mod_names(n));
            }
        }
        names.into_iter().map(|n| n.to_lowercase().chars().filter(|c| !c.is_whitespace()).collect()).collect()
    }

    /// `getTooltipStrings`: lowercase, formatting stripped, minus the name, mod name, mod id and path.
    pub fn tooltip_strings(&self) -> BTreeSet<String> {
        let mut out: BTreeSet<String> = self
            .data
            .tooltip
            .iter()
            .map(|l| strip_formatting(l).to_lowercase().trim().to_owned())
            .filter(|l| !l.is_empty())
            .collect();
        out.remove(&self.names[0]);
        out.remove(&self.mod_names_list()[0].to_lowercase());
        out.remove(self.mod_ids()[0]);
        out.remove(&self.data.path);
        out
    }

    /// `getTagStrings`: the path of each tag id.
    pub fn tag_strings(&self) -> Vec<String> {
        self.data.tags.iter().map(|t| t.split_once(':').map_or(t.as_str(), |(_, p)| p).to_owned()).collect()
    }

    pub fn color_names(&self) -> Vec<String> {
        let mut seen = BTreeSet::new();
        self.data.color_names.iter().map(|c| c.to_lowercase()).filter(|c| seen.insert(c.clone())).collect()
    }

    pub fn creative_tab_strings(&self) -> BTreeSet<String> {
        self.data.creative_tabs.iter().map(|n| strip_formatting(n).to_lowercase().trim().to_owned()).filter(|n| !n.is_empty()).collect()
    }

    pub fn identifier(&self) -> String {
        format!("{}:{}", self.data.namespace, self.data.path)
    }

    /// The strings of one search index, as `ElementPrefixParser`'s getters give them.
    pub fn strings(&self, index: SearchIndex, config: &FilterConfig) -> Vec<String> {
        match index {
            SearchIndex::NamesAndAliases => self.names.clone(),
            SearchIndex::ModNames => self.mod_names(config).into_iter().collect(),
            SearchIndex::Tags => self.tag_strings(),
            SearchIndex::Tooltips => self.tooltip_strings().into_iter().collect(),
            SearchIndex::CreativeTabs => self.creative_tab_strings().into_iter().collect(),
            SearchIndex::Colors => self.color_names(),
            SearchIndex::Identifiers => vec![self.identifier()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prefix::SearchMode;
    use crate::search::ElementSearch;

    fn data(name: &str) -> IngredientData {
        IngredientData {
            display_name: name.into(),
            namespace: "mekanism".into(),
            path: "osmium_ingot".into(),
            display_mod_id: "mekanism".into(),
            mod_name: "Mekanism Additions".into(),
            display_mod_name: "Mekanism Additions".into(),
            tags: vec!["c:ingots/osmium".into()],
            tooltip: vec!["\u{a7}9Osmium Ingot".into(), "Used for gear".into(), "Mekanism Additions".into()],
            ..Default::default()
        }
    }

    #[test]
    fn strips_codes() {
        assert_eq!(strip_formatting("\u{a7}aGreen\u{a7}r plain \u{a7}"), "Green plain \u{a7}");
    }

    #[test]
    fn short_names() {
        assert_eq!(short_mod_names("Mekanism Additions"), vec!["MA", "MeAd"]);
        assert!(short_mod_names("Mekanism").is_empty());
        assert_eq!(short_mod_names("JustEnoughItems"), vec!["JEI", "JuEnIt"]);
    }

    #[test]
    fn mod_names_sanitized() {
        let i = ListElementInfo::new(data("Osmium Ingot"), &FilterConfig::default(), 0);
        let m = i.mod_names(&FilterConfig::default());
        assert!(m.contains("mekanismadditions") && m.contains("mekanism") && m.contains("ma"));
    }

    #[test]
    fn tooltip_drops_name_and_mod() {
        let i = ListElementInfo::new(data("Osmium Ingot"), &FilterConfig::default(), 0);
        assert_eq!(i.tooltip_strings().into_iter().collect::<Vec<_>>(), vec!["used for gear"]);
    }

    #[test]
    fn feeds_search() {
        let cfg = FilterConfig::default();
        let info = ListElementInfo::new(data("Osmium Ingot"), &cfg, 0);
        let mut s = ElementSearch::new(|_| SearchMode::Enabled);
        s.add(0, |i| info.strings(i, &cfg));
        assert_eq!(s.query("@mekanism #osmium"), vec![0]);
        assert_eq!(s.query("$gear"), vec![0]);
        assert_eq!(s.query("&mekanism:osmium_ingot"), vec![0]);
        assert!(s.query("@jei").is_empty());
    }
}
