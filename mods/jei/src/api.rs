//! The plugin API other mods call (`IModPlugin`: `registerCategories`, `registerRecipeCatalysts`, `registerRecipes`).
//! A mod lists `jei` under `[dependencies]` in `horizon-mod.toml` and calls these from its own hooks; every code mod
//! is linked into one binary, so the registry is a plain static the JEI mod reads when it builds its recipes.
pub use crate::jei_core::{
    plugin::{CategoryDef, CustomRecipe, PluginError},
    view::{self, Layout},
};
use crate::jei_core::plugin::Registry;
use mod_api::{ClientHost, DrawContext, ScreenView};
use std::sync::Mutex;

/// An ingredient type beyond items (`IIngredientType`, `IIngredientHelper`, `IIngredientRenderer`): fluids are one, and a
/// mod's chemicals can be another. Its ingredients are listed as `prefix` + a resource id (`fluid:minecraft:water`), which
/// is also the id recipes, bookmarks and the search use.
#[derive(Clone)]
pub struct IngredientType {
    /// `fluid:`: tells this type's ids from items' (an item's id has no prefix).
    pub prefix: &'static str,
    /// Every ingredient of the type, with the prefix, in registry order.
    pub list: fn(&dyn ClientHost) -> Vec<String>,
    /// Draws one ingredient in a `size` square at `at`.
    pub draw: fn(&mut dyn DrawContext, id: &str, at: [f32; 2], size: f32),
    /// Its display name.
    pub name: fn(&dyn ClientHost, id: &str) -> String,
}

static TYPES: Mutex<Vec<IngredientType>> = Mutex::new(Vec::new());

/// A place on a mod's own screen that takes a dragged ingredient (`IGhostIngredientHandler`): a filter slot, say.
/// While the player drags an ingredient out of the list over `screen`, `targets` says where it can be dropped (GUI
/// pixels, x0 y0 x1 y1) and a release over one calls `accept`, which must queue the change itself.
#[derive(Clone)]
pub struct GhostTarget {
    /// The screen's id (`ScreenView::id`); JEI draws its list over it from then on.
    pub screen: &'static str,
    pub targets: fn(&ScreenView<'_>, ingredient: &str) -> Vec<[i32; 4]>,
    /// Called with the mouse position of the release and the target's index; true when it took the ingredient.
    pub accept: fn(&ScreenView<'_>, target: usize, ingredient: &str) -> bool,
}

static GHOSTS: Mutex<Vec<GhostTarget>> = Mutex::new(Vec::new());

/// Registers the drop targets of a screen; a screen id registered twice keeps the first.
pub fn register_ghost_target(target: GhostTarget) {
    let mut ghosts = GHOSTS.lock().unwrap_or_else(|e| e.into_inner());
    if !ghosts.iter().any(|g| g.screen == target.screen) {
        ghosts.push(target);
    }
}

pub(crate) fn ghost_target(screen: &str) -> Option<GhostTarget> {
    GHOSTS.lock().unwrap_or_else(|e| e.into_inner()).iter().find(|g| g.screen == screen).cloned()
}

static REGISTRY: Mutex<Registry> = Mutex::new(Registry { categories: Vec::new(), recipes: Vec::new(), version: 0 });

fn with<T>(f: impl FnOnce(&mut Registry) -> T) -> T {
    f(&mut REGISTRY.lock().unwrap_or_else(|e| e.into_inner()))
}

/// Adds a recipe category (`IRecipeCategory`); its `recipe_type` must be new.
pub fn register_category(def: CategoryDef) -> Result<(), PluginError> {
    with(|r| r.register_category(def))
}

/// Adds an item that makes a registered category's recipes (`IRecipeCatalysts`).
pub fn add_catalyst(recipe_type: &str, item: &str) -> Result<(), PluginError> {
    with(|r| r.add_catalyst(recipe_type, item))
}

/// Adds a recipe to a registered category.
pub fn add_recipe(recipe: CustomRecipe) -> Result<(), PluginError> {
    with(|r| r.add_recipe(recipe))
}

/// Registers an ingredient type; a prefix already registered is kept as it was.
pub fn register_ingredient_type(ty: IngredientType) {
    let mut types = TYPES.lock().unwrap_or_else(|e| e.into_inner());
    if !types.iter().any(|t| t.prefix == ty.prefix) {
        types.push(ty);
        drop(types);
        with(|r| r.version += 1);
    }
}

pub(crate) fn ingredient_types() -> Vec<IngredientType> {
    TYPES.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Raised by every change, so the viewer rebuilds.
pub fn version() -> u64 {
    with(|r| r.version)
}

pub(crate) fn snapshot() -> Registry {
    with(|r| r.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> Layout {
        Layout { width: 100, height: 40, slots: vec![view::input(0, 1, 10), view::output(80, 10)], arrow: Some((40, 12)) }
    }

    #[test]
    fn registers_and_rejects() {
        let before = version();
        register_category(CategoryDef::custom("apitest:crusher", "Crusher", "minecraft:stone", layout())).unwrap();
        assert_eq!(register_category(CategoryDef::custom("apitest:crusher", "Crusher", "minecraft:stone", layout())), Err(PluginError::DuplicateCategory("apitest:crusher".into())));
        add_catalyst("apitest:crusher", "minecraft:furnace").unwrap();
        assert_eq!(add_catalyst("apitest:none", "minecraft:furnace"), Err(PluginError::UnknownCategory("apitest:none".into())));
        let r = CustomRecipe { recipe_type: "apitest:crusher".into(), inputs: vec![vec!["minecraft:stone".into()]], outputs: vec![vec!["minecraft:gravel".into()]], notes: vec![] };
        add_recipe(r).unwrap();
        assert!(version() > before);
        let snap = snapshot();
        assert!(snap.categories.iter().any(|c| c.recipe_type == "apitest:crusher" && c.stations == ["minecraft:stone", "minecraft:furnace"]));
    }

    #[test]
    fn ghost_targets_register_once_per_screen() {
        fn none(_: &ScreenView<'_>, _: &str) -> Vec<[i32; 4]> {
            Vec::new()
        }
        fn take(_: &ScreenView<'_>, _: usize, _: &str) -> bool {
            true
        }
        assert!(ghost_target("apitest:filter").is_none());
        register_ghost_target(GhostTarget { screen: "apitest:filter", targets: none, accept: take });
        assert!(ghost_target("apitest:filter").is_some());
    }
}
