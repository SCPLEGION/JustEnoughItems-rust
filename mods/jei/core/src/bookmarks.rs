//! Bookmarks (`BookmarkList`, the part without the game): an ordered list of ingredient uids; adding one that is
//! already there removes it (JEI's `A` toggles), a new one goes to the front, and the list saves as one uid per line.

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BookmarkList {
    items: Vec<String>,
}

impl BookmarkList {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn items(&self) -> &[String] {
        &self.items
    }
    pub fn contains(&self, uid: &str) -> bool {
        self.items.iter().any(|i| i == uid)
    }
    /// `BookmarkList.addToListWithoutNotification`/`remove`: toggles; returns whether it is bookmarked now.
    pub fn toggle(&mut self, uid: &str) -> bool {
        if let Some(i) = self.items.iter().position(|x| x == uid) {
            self.items.remove(i);
            false
        } else {
            self.items.insert(0, uid.to_owned());
            true
        }
    }
    pub fn remove(&mut self, uid: &str) -> bool {
        let before = self.items.len();
        self.items.retain(|i| i != uid);
        self.items.len() != before
    }
    pub fn save(&self) -> String {
        self.items.iter().map(|i| format!("{i}\n")).collect()
    }
    pub fn load(text: &str) -> Self {
        let mut items: Vec<String> = Vec::new();
        for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
            if !items.iter().any(|i| i == line) {
                items.push(line.to_owned());
            }
        }
        Self { items }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_and_order() {
        let mut b = BookmarkList::new();
        assert!(b.toggle("minecraft:stick"));
        assert!(b.toggle("minecraft:coal"));
        assert_eq!(b.items(), ["minecraft:coal", "minecraft:stick"]);
        assert!(!b.toggle("minecraft:coal"));
        assert_eq!(b.items(), ["minecraft:stick"]);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let mut b = BookmarkList::new();
        b.toggle("a:b");
        b.toggle("c:d");
        assert_eq!(BookmarkList::load(&b.save()), b);
        assert_eq!(BookmarkList::load("x:y\n\nx:y\n").items(), ["x:y"]);
    }
}
