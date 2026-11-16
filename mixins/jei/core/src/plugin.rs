//! The plugin surface for other mods (`IModPlugin`, `IRecipeCategoryRegistration`, `IRecipeRegistration`,
//! `IRecipeCatalystRegistration`) in data form: a mod registers a category (its size, slots and arrow), its crafting
//! stations and its recipes (the ingredients of each slot). JEI's glue draws them like the vanilla categories.
use crate::view::{self, Kind, Layout, SlotRole};

/// A recipe category: a vanilla one (laid out by `view::layout`) or a mod's own with a fixed layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CategoryDef {
    /// `IRecipeType` uid, `mekanism:crusher`.
    pub recipe_type: String,
    pub title: String,
    /// The tab icon item, `mekanism:crusher`.
    pub icon: String,
    /// Crafting stations (`registerRecipeCatalysts`): items that make this category's recipes.
    pub stations: Vec<String>,
    pub layout: Layout,
    pub vanilla: Option<Kind>,
}

impl CategoryDef {
    pub fn vanilla(kind: Kind) -> Self {
        Self {
            recipe_type: kind.recipe_type().into(),
            title: kind.title_en().into(),
            icon: kind.icon().into(),
            stations: vec![kind.icon().into()],
            layout: view::layout(kind, (3, 3), 9),
            vanilla: Some(kind),
        }
    }
    pub fn custom(recipe_type: &str, title: &str, icon: &str, layout: Layout) -> Self {
        Self { recipe_type: recipe_type.into(), title: title.into(), icon: icon.into(), stations: vec![icon.into()], layout, vanilla: None }
    }
    /// The layout of one recipe: vanilla ones depend on the recipe's grid and input count.
    pub fn layout_for(&self, grid: (i32, i32), inputs: usize) -> Layout {
        match self.vanilla {
            Some(k) => view::layout(k, grid, inputs),
            None => self.layout.clone(),
        }
    }
}

/// A recipe of a mod's category: the ingredients (item ids) each input slot cycles through, and each output slot's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomRecipe {
    pub recipe_type: String,
    pub inputs: Vec<Vec<String>>,
    pub outputs: Vec<Vec<String>>,
    /// A line of text under the layout for each step of the slots' cycle (the anvil's level cost); may be empty.
    pub notes: Vec<String>,
}

/// An ingredient string is an item id, optionally followed by `\u{1}` and its components (`[minecraft:damage=5]`).
pub fn split_ingredient(s: &str) -> (&str, Option<&str>) {
    match s.split_once('\u{1}') {
        Some((id, comps)) => (id, Some(comps)),
        None => (s, None),
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PluginError {
    DuplicateCategory(String),
    UnknownCategory(String),
    /// The recipe has more input or output slots than the category's layout.
    TooManySlots(String),
}

#[derive(Clone, Debug, Default)]
pub struct Registry {
    pub categories: Vec<CategoryDef>,
    pub recipes: Vec<CustomRecipe>,
    /// Raised by each change, so a viewer rebuilds its indexes.
    pub version: u64,
}

impl Registry {
    pub fn register_category(&mut self, def: CategoryDef) -> Result<(), PluginError> {
        if self.categories.iter().any(|c| c.recipe_type == def.recipe_type) {
            return Err(PluginError::DuplicateCategory(def.recipe_type));
        }
        self.categories.push(def);
        self.version += 1;
        Ok(())
    }
    pub fn add_catalyst(&mut self, recipe_type: &str, item: &str) -> Result<(), PluginError> {
        let c = self.categories.iter_mut().find(|c| c.recipe_type == recipe_type).ok_or_else(|| PluginError::UnknownCategory(recipe_type.into()))?;
        if !c.stations.iter().any(|s| s == item) {
            c.stations.push(item.into());
            self.version += 1;
        }
        Ok(())
    }
    pub fn add_recipe(&mut self, recipe: CustomRecipe) -> Result<(), PluginError> {
        let c = self.categories.iter().find(|c| c.recipe_type == recipe.recipe_type).ok_or_else(|| PluginError::UnknownCategory(recipe.recipe_type.clone()))?;
        let count = |role| c.layout.slots.iter().filter(|s| s.role == role).count();
        // A category without output slots takes invisible outputs (JEI's `addInvisibleIngredients`): found, not drawn.
        if c.vanilla.is_none() && (recipe.inputs.len() > count(SlotRole::Input) || (count(SlotRole::Output) > 0 && recipe.outputs.len() > count(SlotRole::Output))) {
            return Err(PluginError::TooManySlots(recipe.recipe_type));
        }
        self.recipes.push(recipe);
        self.version += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crusher() -> CategoryDef {
        CategoryDef::custom(
            "mekanism:crusher",
            "Crusher",
            "mekanism:crusher",
            Layout { width: 100, height: 40, slots: vec![view::input(0, 1, 10), view::output(80, 10)], arrow: Some((40, 12)) },
        )
    }

    #[test]
    fn invisible_outputs_are_accepted() {
        let mut r = Registry::default();
        let layout = Layout { width: 170, height: 125, slots: vec![view::input(0, 77, 1)], arrow: None };
        r.register_category(CategoryDef::custom("jei:information", "Information", "minecraft:book", layout)).unwrap();
        let info = CustomRecipe { recipe_type: "jei:information".into(), inputs: vec![vec!["minecraft:stick".into()]], outputs: vec![vec!["minecraft:stick".into()]], notes: vec!["text".into()] };
        r.add_recipe(info).unwrap();
    }

    #[test]
    fn registers_and_validates() {
        let mut r = Registry::default();
        r.register_category(crusher()).unwrap();
        assert_eq!(r.register_category(crusher()), Err(PluginError::DuplicateCategory("mekanism:crusher".into())));
        let ok = CustomRecipe { recipe_type: "mekanism:crusher".into(), inputs: vec![vec!["minecraft:cobblestone".into()]], outputs: vec![vec!["minecraft:gravel".into()]], notes: vec![] };
        r.add_recipe(ok.clone()).unwrap();
        let too_many = CustomRecipe { inputs: vec![vec![], vec![]], ..ok.clone() };
        assert!(matches!(r.add_recipe(too_many), Err(PluginError::TooManySlots(_))));
        let other = CustomRecipe { recipe_type: "x:y".into(), ..ok };
        assert!(matches!(r.add_recipe(other), Err(PluginError::UnknownCategory(_))));
        r.add_catalyst("mekanism:crusher", "mekanism:advanced_crusher").unwrap();
        assert_eq!(r.categories[0].stations.len(), 2);
    }

    #[test]
    fn splits_ingredient_components() {
        assert_eq!(split_ingredient("minecraft:stick"), ("minecraft:stick", None));
        assert_eq!(split_ingredient("minecraft:bow\u{1}[minecraft:damage=5]"), ("minecraft:bow", Some("[minecraft:damage=5]")));
    }

    #[test]
    fn vanilla_layout_follows_recipe() {
        let c = CategoryDef::vanilla(Kind::Crafting);
        assert_eq!(c.layout_for((2, 2), 4).slots.len(), 5);
    }
}
