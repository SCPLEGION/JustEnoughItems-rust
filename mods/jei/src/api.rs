//! The plugin API other mods call (`IModPlugin`: `registerCategories`, `registerRecipeCatalysts`, `registerRecipes`).
//! A mod lists `jei` under `[dependencies]` in `horizon-mod.toml` and calls these from its own hooks; every code mod
//! is linked into one binary, so the registry is a plain static the JEI mod reads when it builds its recipes.
pub use crate::jei_core::{
    plugin::{CategoryDef, CustomRecipe, PluginError},
    view::{self, Layout},
};
use crate::jei_core::plugin::Registry;
use std::sync::Mutex;

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
}
