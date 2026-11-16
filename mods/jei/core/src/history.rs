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
}
