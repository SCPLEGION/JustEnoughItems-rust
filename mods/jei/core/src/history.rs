//! `LookupHistory`: the ingredients whose recipes were looked up, newest first, bounded (`lookups.maxIngredients`).

/// The default of `maxIngredients` (range 10 to 1000).
pub const DEFAULT_MAX: usize = 100;

#[derive(Clone, Debug)]
pub struct LookupHistory {
    items: Vec<String>,
    max: usize,
}

impl Default for LookupHistory {
    fn default() -> Self {
        Self { items: Vec::new(), max: DEFAULT_MAX }
    }
}

impl LookupHistory {
    pub fn new(max: usize) -> Self {
        Self { items: Vec::new(), max: max.clamp(10, 1000) }
    }
    /// `add`: moves an existing entry to the front, drops the oldest past the limit.
    pub fn add(&mut self, item: &str) {
        self.items.retain(|i| i != item);
        self.items.insert(0, item.to_owned());
        self.items.truncate(self.max);
    }
    pub fn set_max(&mut self, max: usize) {
        self.max = max.clamp(10, 1000);
        self.items.truncate(self.max);
    }
    pub fn items(&self) -> &[String] {
        &self.items
    }
    pub fn save(&self) -> String {
        self.items.iter().map(|i| format!("{i}\n")).collect()
    }
    pub fn load(text: &str, max: usize) -> Self {
        let mut h = Self::new(max);
        for line in text.lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().into_iter().rev() {
            h.add(line);
        }
        h
    }
}

/// The search strings entered lately (`previousSearch` / `nextSearch`): an entry is added when the search box loses
/// focus, a repeated one moves to the end, and the oldest drops past `DEFAULT_MAX`.
#[derive(Clone, Debug, Default)]
pub struct SearchHistory {
    items: Vec<String>,
    /// The entry shown by `previous`/`next`; `items.len()` is the live, unsaved text.
    cursor: usize,
}

impl SearchHistory {
    pub fn add(&mut self, text: &str) {
        if !text.is_empty() {
            self.items.retain(|i| i != text);
            self.items.push(text.to_owned());
            if self.items.len() > DEFAULT_MAX {
                self.items.remove(0);
            }
        }
        self.cursor = self.items.len();
    }
    /// The entry before the current one, or `None` at the oldest.
    pub fn previous(&mut self) -> Option<&str> {
        self.cursor = self.cursor.checked_sub(1)?;
        self.items.get(self.cursor).map(String::as_str)
    }
    /// The entry after the current one; an empty string past the newest.
    pub fn next(&mut self) -> Option<&str> {
        if self.cursor >= self.items.len() {
            return None;
        }
        self.cursor += 1;
        Some(self.items.get(self.cursor).map_or("", String::as_str))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newest_first_and_deduplicated() {
        let mut h = LookupHistory::default();
        h.add("a");
        h.add("b");
        h.add("a");
        assert_eq!(h.items(), ["a", "b"]);
    }

    #[test]
    fn bounded() {
        let mut h = LookupHistory::new(10);
        for i in 0..15 {
            h.add(&format!("i{i}"));
        }
        assert_eq!(h.items().len(), 10);
        assert_eq!(h.items()[0], "i14");
        h.set_max(10_000);
        assert_eq!(h.items().len(), 10);
    }

    #[test]
    fn saves_and_loads_in_order() {
        let mut h = LookupHistory::default();
        h.add("x");
        h.add("y");
        let back = LookupHistory::load(&h.save(), DEFAULT_MAX);
        assert_eq!(back.items(), h.items());
    }

    #[test]
    fn search_history_walks_back_and_forward() {
        let mut h = SearchHistory::default();
        h.add("a");
        h.add("b");
        h.add("a");
        assert_eq!(h.previous(), Some("a"));
        assert_eq!(h.previous(), Some("b"));
        assert_eq!(h.previous(), None);
        assert_eq!(h.next(), Some("a"));
        assert_eq!(h.next(), Some(""));
        assert_eq!(h.next(), None);
    }
}
