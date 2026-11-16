//! `ClientConfig` and `jei-client.toml`: the option table (section, name, kind, default), reading and writing the file
//! with every value clamped to its range, as `ConfigSerializer` does.

use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Bool(bool),
    Int { default: i64, min: i64, max: i64 },
    Enum { default: &'static str, values: &'static [&'static str] },
    /// A list of enum names, written `["A", "B"]`; only names of `values` are kept, in order.
    List { default: &'static str, values: &'static [&'static str] },
}

#[derive(Clone, Copy, Debug)]
pub struct Opt {
    pub section: &'static str,
    pub name: &'static str,
    pub kind: Kind,
}

const fn b(section: &'static str, name: &'static str, default: bool) -> Opt {
    Opt { section, name, kind: Kind::Bool(default) }
}
const fn i(section: &'static str, name: &'static str, default: i64, min: i64, max: i64) -> Opt {
    Opt { section, name, kind: Kind::Int { default, min, max } }
}
const fn l(section: &'static str, name: &'static str, default: &'static str, values: &'static [&'static str]) -> Opt {
    Opt { section, name, kind: Kind::List { default, values } }
}
const fn e(section: &'static str, name: &'static str, default: &'static str, values: &'static [&'static str]) -> Opt {
    Opt { section, name, kind: Kind::Enum { default, values } }
}

/// The options of `ClientConfig` (Java names, so a file of the Java mod reads back).
pub const OPTIONS: &[Opt] = &[
    b("search", "searchCompletionEnabled", false),
    i("search", "maxSearchCompletionRows", 5, 1, 100),
    b("ingredientList", "overlaysEnabled", true),
    l("ingredientList", "ingredientSortStages", "MOD_NAME,INGREDIENT_TYPE,CREATIVE_MENU", &["MOD_NAME", "INGREDIENT_TYPE", "ALPHABETICAL", "CREATIVE_MENU", "TAG", "ARMOR", "MAX_DURABILITY"]),
    b("ingredientList", "toastReflowEnabled", true),
    b("ingredientList", "compactInventoryEffects", true),
    b("ingredientList", "hideIngredientsWhenNotSearching", false),
    i("ingredientList", "maxColumns", 100, 4, 100),
    i("ingredientList", "maxRows", 100, 1, 100),
    b("bookmarkList", "bookmarkOutputAsRecipe", true),
    b("bookmarkList", "dragToRearrangeBookmarksEnabled", true),
    b("bookmarkList", "bookmarkTooltipPreview", true),
    b("bookmarkList", "bookmarkTooltipIngredients", false),
    b("bookmarkList", "holdShiftToShowBookmarkTooltipFeatures", true),
    b("input", "guiResizeEnabled", true),
    i("input", "dragDelayInMilliseconds", 150, 0, 1000),
    b("input", "smoothScrollingEnabled", true),
    i("input", "smoothScrollRate", 9, 1, 50),
    b("input", "recipeSlotCyclingEnabled", true),
    b("recipes", "showTagRecipesEnabled", true),
    i("recipes", "recipeGuiHeight", 350, 175, i32::MAX as i64),
    i("recipes", "recipeGuiWidth", 198, 198, i32::MAX as i64),
    i("recipes", "maxRecipeGuiColumns", 2, 1, 10),
    e("recipes", "navigationMode", "PAGED", &["PAGED", "SCROLLED"]),
    b("recipes", "recipeSortingBookmarks", true),
    b("recipes", "recipeSortingCraftable", true),
    b("recipes", "enableRecipesGuiIngredientsSummary", false),
    b("tooltips", "showCreativeTabNamesEnabled", false),
    b("tooltips", "tagContentTooltipEnabled", true),
    b("tooltips", "hideSingleTagContentTooltipEnabled", true),
    b("lookups", "lookupFluidContentsEnabled", false),
    b("lookups", "lookupBlockTagsEnabled", true),
    b("lookups", "enabled", false),
    i("lookups", "maxRows", 2, 1, 7),
    i("lookups", "maxColumns", 9, 2, 100),
    i("lookups", "maxIngredients", 100, 10, 1000),
    e("lookups", "displaySide", "LEFT", &["LEFT", "RIGHT"]),
    e("cheating", "giveMode", "MOUSE_PICKUP", &["MOUSE_PICKUP", "INVENTORY"]),
    b("cheating", "cheatToHotbarUsingHotkeysEnabled", false),
    b("cheating", "showHiddenIngredients", false),
    b("advanced", "catchRenderErrorsEnabled", true),
    b("advanced", "recipeSyncWarningEnabled", true),
];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Config {
    values: BTreeMap<(String, String), String>,
}

fn find(section: &str, name: &str) -> Option<&'static Opt> {
    OPTIONS.iter().find(|o| o.section == section && o.name == name)
}

/// A raw value read from the file, brought into the option's range, or `None` when it is not valid for its kind.
fn normalize(kind: Kind, raw: &str) -> Option<String> {
    match kind {
        Kind::Bool(_) => matches!(raw, "true" | "false").then(|| raw.to_owned()),
        Kind::Int { min, max, .. } => raw.parse::<i64>().ok().map(|v| v.clamp(min, max).to_string()),
        Kind::Enum { values, .. } => {
            let v = raw.trim_matches('"');
            values.contains(&v).then(|| v.to_owned())
        }
        Kind::List { values, .. } => {
            // `["A", "B"]` or `A,B`, stored as `A,B`.
            let inner = raw.trim().trim_start_matches('[').trim_end_matches(']');
            let names: Vec<&str> = inner.split(',').map(|n| n.trim().trim_matches('"')).filter(|n| values.contains(n)).collect();
            Some(names.join(","))
        }
    }
}

impl Config {
    pub fn parse(text: &str) -> Self {
        let mut c = Self::default();
        let mut section = String::new();
        for line in text.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(s) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                section = s.trim().to_owned();
            } else if let Some((k, v)) = line.split_once('=')
                && let Some(opt) = find(&section, k.trim())
                && let Some(v) = normalize(opt.kind, v.trim())
            {
                c.values.insert((section.clone(), opt.name.to_owned()), v);
            }
        }
        c
    }
    fn raw(&self, section: &str, name: &str) -> String {
        let opt = find(section, name).unwrap_or_else(|| panic!("no JEI option {section}.{name}"));
        self.values.get(&(section.to_owned(), name.to_owned())).cloned().unwrap_or_else(|| match opt.kind {
            Kind::Bool(d) => d.to_string(),
            Kind::Int { default, .. } => default.to_string(),
            Kind::Enum { default, .. } | Kind::List { default, .. } => default.to_owned(),
        })
    }
    pub fn bool(&self, section: &str, name: &str) -> bool {
        self.raw(section, name) == "true"
    }
    pub fn int(&self, section: &str, name: &str) -> i64 {
        self.raw(section, name).parse().unwrap_or_default()
    }
    pub fn text(&self, section: &str, name: &str) -> String {
        self.raw(section, name)
    }
    /// A list option's names, in order.
    pub fn list(&self, section: &str, name: &str) -> Vec<String> {
        self.raw(section, name).split(',').filter(|n| !n.is_empty()).map(str::to_owned).collect()
    }
    /// Sets an option; false when the value is not valid for it (nothing changes).
    pub fn set(&mut self, section: &str, name: &str, value: &str) -> bool {
        let Some(opt) = find(section, name) else { return false };
        match normalize(opt.kind, value) {
            Some(v) => {
                self.values.insert((section.to_owned(), name.to_owned()), v);
                true
            }
            None => false,
        }
    }
    /// The file: every option in its section with its value, strings quoted.
    pub fn serialize(&self) -> String {
        let mut out = String::new();
        let mut section = "";
        for o in OPTIONS {
            if o.section != section {
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(&format!("[{}]\n", o.section));
                section = o.section;
            }
            let v = self.raw(o.section, o.name);
            match o.kind {
                Kind::Enum { .. } => out.push_str(&format!("{} = \"{v}\"\n", o.name)),
                Kind::List { .. } => {
                    let items: Vec<String> = v.split(',').filter(|n| !n.is_empty()).map(|n| format!("\"{n}\"")).collect();
                    out.push_str(&format!("{} = [{}]\n", o.name, items.join(", ")));
                }
                _ => out.push_str(&format!("{} = {v}\n", o.name)),
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_empty() {
        let c = Config::default();
        assert!(!c.bool("search", "searchCompletionEnabled"));
        assert_eq!(c.int("lookups", "maxIngredients"), 100);
        assert_eq!(c.text("recipes", "navigationMode"), "PAGED");
    }

    #[test]
    fn values_are_clamped_and_invalid_ones_ignored() {
        let c = Config::parse("[lookups]\nmaxIngredients = 5000\nmaxRows = 0\nenabled = maybe\n[recipes]\nnavigationMode = \"SIDEWAYS\"\n");
        assert_eq!(c.int("lookups", "maxIngredients"), 1000);
        assert_eq!(c.int("lookups", "maxRows"), 1);
        assert!(!c.bool("lookups", "enabled"));
        assert_eq!(c.text("recipes", "navigationMode"), "PAGED");
    }

    #[test]
    fn round_trips() {
        let mut c = Config::default();
        assert!(c.set("input", "smoothScrollRate", "20"));
        assert!(c.set("recipes", "navigationMode", "SCROLLED"));
        assert!(!c.set("input", "smoothScrollRate", "x"));
        assert!(!c.set("nope", "nope", "1"));
        let back = Config::parse(&c.serialize());
        assert_eq!(back.int("input", "smoothScrollRate"), 20);
        assert_eq!(back.text("recipes", "navigationMode"), "SCROLLED");
        assert_eq!(back, Config::parse(&back.serialize()));
    }

    #[test]
    fn list_options_keep_known_names_in_order() {
        let c = Config::parse("[ingredientList]\ningredientSortStages = [\"ALPHABETICAL\", \"NOPE\", \"MOD_NAME\"]\n");
        assert_eq!(c.list("ingredientList", "ingredientSortStages"), ["ALPHABETICAL", "MOD_NAME"]);
        assert!(c.serialize().contains("ingredientSortStages = [\"ALPHABETICAL\", \"MOD_NAME\"]"));
        assert_eq!(Config::default().list("ingredientList", "ingredientSortStages"), ["MOD_NAME", "INGREDIENT_TYPE", "CREATIVE_MENU"]);
    }

    #[test]
    fn unknown_keys_and_comments_are_skipped() {
        let c = Config::parse("# hi\n[search]\nunknown = 1\nsearchCompletionEnabled = true\n");
        assert!(c.bool("search", "searchCompletionEnabled"));
    }
}
