//! The keyboard mappings of `InternalKeyMappings` (Java ids and default keys). Mouse mappings (`showRecipe2`, the
//! cheat clicks...) are the clicks the glue already handles and are not listed.

/// What a mapping does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    ToggleOverlay,
    FocusSearch,
    PreviousPage,
    NextPage,
    ToggleBookmarkOverlay,
    Bookmark,
    ShowRecipe,
    ShowUses,
    PreviousSearch,
    NextSearch,
    ToggleCheatMode,
    ToggleEditMode,
    RecipeBack,
    RecipeForward,
    PreviousRecipePage,
    NextRecipePage,
    PreviousCategory,
    NextCategory,
    CloseRecipeGui,
}

/// GLFW modifier bits: shift 1, control 2, alt 4.
pub const SHIFT: u32 = 1;
pub const CONTROL: u32 = 2;
pub const ALT: u32 = 4;

#[derive(Clone, Copy, Debug)]
pub struct Mapping {
    /// The Java id (`key.jei.toggleOverlay`); the registered id is its last part.
    pub id: &'static str,
    pub action: Action,
    /// The default key; `key.keyboard.unknown` is unbound.
    pub key: &'static str,
    pub modifiers: u32,
    /// Whether it needs the recipe screen open (`closeRecipeGui`, the recipe paging...).
    pub in_recipes: bool,
}

const fn m(id: &'static str, action: Action, key: &'static str, modifiers: u32, in_recipes: bool) -> Mapping {
    Mapping { id, action, key, modifiers, in_recipes }
}

const NONE: &str = "key.keyboard.unknown";

pub const MAPPINGS: &[Mapping] = &[
    m("key.jei.toggleOverlay", Action::ToggleOverlay, "key.keyboard.o", CONTROL, false),
    m("key.jei.focusSearch", Action::FocusSearch, "key.keyboard.f", CONTROL, false),
    m("key.jei.previousPage", Action::PreviousPage, NONE, 0, false),
    m("key.jei.nextPage", Action::NextPage, NONE, 0, false),
    m("key.jei.toggleBookmarkOverlay", Action::ToggleBookmarkOverlay, NONE, 0, false),
    m("key.jei.bookmark", Action::Bookmark, "key.keyboard.a", 0, false),
    m("key.jei.showRecipe", Action::ShowRecipe, "key.keyboard.r", 0, false),
    m("key.jei.showUses", Action::ShowUses, "key.keyboard.u", 0, false),
    m("key.jei.previousSearch", Action::PreviousSearch, "key.keyboard.up", ALT, false),
    m("key.jei.nextSearch", Action::NextSearch, "key.keyboard.down", ALT, false),
    m("key.jei.toggleCheatMode", Action::ToggleCheatMode, NONE, 0, false),
    m("key.jei.toggleEditMode", Action::ToggleEditMode, NONE, 0, false),
    m("key.jei.recipeBack", Action::RecipeBack, "key.keyboard.backspace", 0, true),
    m("key.jei.recipeForward", Action::RecipeForward, NONE, 0, true),
    m("key.jei.previousRecipePage", Action::PreviousRecipePage, "key.keyboard.page.up", 0, true),
    m("key.jei.nextRecipePage", Action::NextRecipePage, "key.keyboard.page.down", 0, true),
    m("key.jei.previousCategory", Action::PreviousCategory, "key.keyboard.page.up", SHIFT, true),
    m("key.jei.nextCategory", Action::NextCategory, "key.keyboard.page.down", SHIFT, true),
    m("key.jei.closeRecipeGui", Action::CloseRecipeGui, "key.keyboard.escape", 0, true),
];

/// The id a mapping is registered under: the Java id without its `key.jei.` prefix, dots as underscores.
pub fn registered_id(m: &Mapping) -> String {
    m.id.trim_start_matches("key.jei.").replace('.', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<String> = MAPPINGS.iter().map(registered_id).collect();
        ids.sort();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n);
    }

    #[test]
    fn category_keys_need_shift() {
        // `previousRecipePage` and `previousCategory` share PageUp and differ by Shift only.
        let page = MAPPINGS.iter().find(|m| m.action == Action::PreviousRecipePage).unwrap();
        let cat = MAPPINGS.iter().find(|m| m.action == Action::PreviousCategory).unwrap();
        assert_eq!(page.key, cat.key);
        assert_eq!((page.modifiers, cat.modifiers), (0, SHIFT));
    }
}
