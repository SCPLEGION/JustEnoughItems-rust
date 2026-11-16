//! Port of `RecipeManagerInternal` and the `collect` maps: recipes registered per recipe type, found by focus
//! (an ingredient in a role), categories ordered and hidden, crafting stations (catalysts) of a type.
//!
//! Ingredients are identified by their uid string (`IIngredientHelper.getUid` with the type, e.g. `item:minecraft:stick`);
//! the recipe payloads themselves stay with the category that draws them, so a recipe here is an id plus its slots.
use std::collections::{HashMap, HashSet};

/// `RecipeIngredientRole`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    Input,
    Output,
    /// A recipe's catalyst: it takes part but is not consumed.
    Catalyst,
    RenderOnly,
    /// A category's crafting station (a block that makes this kind of recipe).
    CraftingStation,
}

/// `IRecipeType`: the uid of a recipe type (`minecraft:crafting`).
pub type RecipeType = String;

/// One slot of a recipe: its role and the ingredients it cycles through.
#[derive(Clone, Debug)]
pub struct Slot {
    pub role: Role,
    pub ingredients: Vec<String>,
}

pub type RecipeId = u32;

#[derive(Clone, Debug)]
struct Entry {
    id: RecipeId,
    slots: Vec<Slot>,
}

/// `IFocus`: an ingredient in a role (`Input`, `Output` or `CraftingStation`;
/// JEI maps a catalyst focus to the crafting-station role).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Focus {
    pub role: Role,
    pub uid: String,
}

#[derive(Default)]
struct TypeData {
    recipes: Vec<Entry>,
    hidden: HashSet<RecipeId>,
    stations: Vec<Vec<String>>,
}

#[derive(Default)]
pub struct RecipeManager {
    /// Categories in registration order; `order` below is the display order.
    types: Vec<RecipeType>,
    data: HashMap<RecipeType, TypeData>,
    /// (type, role, uid) -> recipe ids; a map per role as in `RecipeIngredientRoleMap`.
    by_focus: HashMap<(Role, String), HashMap<RecipeType, Vec<RecipeId>>>,
    hidden_types: HashSet<RecipeType>,
    next_id: RecipeId,
}

impl RecipeManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// `registerRecipeCategories`: a category is known by its type; order of registration is the default order.
    pub fn add_category(&mut self, recipe_type: &str) {
        if !self.data.contains_key(recipe_type) {
            self.types.push(recipe_type.to_owned());
            self.data.insert(recipe_type.to_owned(), TypeData::default());
        }
    }

    /// `registerRecipeCatalysts`: the ingredients that craft a type's recipes (one entry may hold several uids).
    pub fn add_crafting_station(&mut self, recipe_type: &str, uids: Vec<String>) {
        let Some(data) = self.data.get_mut(recipe_type) else { return };
        for uid in &uids {
            self.by_focus.entry((Role::CraftingStation, uid.clone())).or_default().entry(recipe_type.to_owned()).or_default();
        }
        data.stations.push(uids);
    }

    /// `addRecipes`: returns the recipe's id, or `None` for a type without a category.
    pub fn add_recipe(&mut self, recipe_type: &str, slots: Vec<Slot>) -> Option<RecipeId> {
        let data = self.data.get_mut(recipe_type)?;
        let id = self.next_id;
        self.next_id += 1;
        for slot in &slots {
            let role = match slot.role {
                Role::Input | Role::Output | Role::CraftingStation => slot.role,
                Role::Catalyst => Role::CraftingStation,
                Role::RenderOnly => continue,
            };
            for uid in &slot.ingredients {
                let list = self.by_focus.entry((role, uid.clone())).or_default().entry(recipe_type.to_owned()).or_default();
                if !list.contains(&id) {
                    list.push(id);
                }
            }
        }
        data.recipes.push(Entry { id, slots });
        Some(id)
    }

    pub fn hide_recipes(&mut self, recipe_type: &str, ids: &[RecipeId]) {
        if let Some(d) = self.data.get_mut(recipe_type) {
            d.hidden.extend(ids);
        }
    }
    pub fn unhide_recipes(&mut self, recipe_type: &str, ids: &[RecipeId]) {
        if let Some(d) = self.data.get_mut(recipe_type) {
            for id in ids {
                d.hidden.remove(id);
            }
        }
    }
    pub fn hide_category(&mut self, recipe_type: &str) {
        self.hidden_types.insert(recipe_type.to_owned());
    }
    pub fn unhide_category(&mut self, recipe_type: &str) {
        self.hidden_types.remove(recipe_type);
    }

    fn focus_role(role: Role) -> Role {
        if role == Role::Catalyst { Role::CraftingStation } else { role }
    }

    /// The recipes of a type, optionally narrowed by focuses (all must match: `FocusGroup`), hidden ones only on request.
    /// `visible` says whether an ingredient uid is shown (`IIngredientVisibility`, recipe context).
    pub fn recipes(&self, recipe_type: &str, focuses: &[Focus], include_hidden: bool, visible: &dyn Fn(&str) -> bool) -> Vec<RecipeId> {
        if !include_hidden && self.hidden_types.contains(recipe_type) {
            return vec![];
        }
        let Some(data) = self.data.get(recipe_type) else { return vec![] };
        let mut candidates: Vec<RecipeId> = data.recipes.iter().map(|e| e.id).collect();
        for f in focuses {
            let role = Self::focus_role(f.role);
            let mut matching: Vec<RecipeId> =
                self.by_focus.get(&(role, f.uid.clone())).and_then(|m| m.get(recipe_type)).cloned().unwrap_or_default();
            // `InternalRecipeManagerPlugin.getRecipes`: a crafting station of the type shows all the type's recipes.
            if self.by_focus.get(&(Role::CraftingStation, f.uid.clone())).is_some_and(|m| m.contains_key(recipe_type))
                && role == Role::CraftingStation
            {
                matching = data.recipes.iter().map(|e| e.id).collect();
            }
            let set: HashSet<RecipeId> = matching.into_iter().collect();
            candidates.retain(|id| set.contains(id));
        }
        if include_hidden {
            return candidates;
        }
        let by_id: HashMap<RecipeId, &Entry> = data.recipes.iter().map(|e| (e.id, e)).collect();
        candidates
            .into_iter()
            .filter(|id| !data.hidden.contains(id))
            .filter(|id| {
                by_id[id].slots.iter().filter(|s| matches!(s.role, Role::Input | Role::Output)).all(|s| s.ingredients.is_empty() || s.ingredients.iter().any(|u| visible(u)))
            })
            .collect()
    }

    /// `getCraftingStations`: the stations of a type that still have a visible ingredient (or all, when hidden are wanted).
    pub fn crafting_stations(&self, recipe_type: &str, include_hidden: bool, visible: &dyn Fn(&str) -> bool) -> Vec<Vec<String>> {
        let Some(data) = self.data.get(recipe_type) else { return vec![] };
        data.stations
            .iter()
            .filter(|s| include_hidden || s.iter().any(|u| visible(u)))
            .map(|s| if include_hidden { s.clone() } else { s.iter().filter(|u| visible(u)).cloned().collect() })
            .collect()
    }

    /// `isCategoryHidden`.
    pub fn is_category_hidden(&self, recipe_type: &str, focuses: &[Focus], visible: &dyn Fn(&str) -> bool) -> bool {
        if self.hidden_types.contains(recipe_type) {
            return true;
        }
        if !self.crafting_stations(recipe_type, true, &|_| true).is_empty() && self.crafting_stations(recipe_type, false, visible).is_empty() {
            return true;
        }
        self.recipes(recipe_type, focuses, false, visible).is_empty()
    }

    /// `getRecipeCategories(focuses, includeHidden)`: visible categories in display order. With a focus, only the
    /// types that have a recipe for every focus (the plugin's `getRecipeTypes`), else all types.
    pub fn categories(&self, focuses: &[Focus], include_hidden: bool, visible: &dyn Fn(&str) -> bool) -> Vec<RecipeType> {
        self.types
            .iter()
            .filter(|t| include_hidden || !self.is_category_hidden(t, focuses, visible))
            .filter(|t| focuses.is_empty() || !self.recipes(t, focuses, true, visible).is_empty())
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(role: Role, u: &[&str]) -> Slot {
        Slot { role, ingredients: u.iter().map(|s| s.to_string()).collect() }
    }
    fn all(_: &str) -> bool {
        true
    }
    fn focus(role: Role, uid: &str) -> Focus {
        Focus { role, uid: uid.into() }
    }

    fn manager() -> RecipeManager {
        let mut m = RecipeManager::new();
        m.add_category("smelting");
        m.add_category("crafting");
        m.add_category("empty");
        m.add_recipe("smelting", vec![slot(Role::Input, &["ore"]), slot(Role::Output, &["ingot"])]);
        m.add_recipe("crafting", vec![slot(Role::Input, &["ingot", "plank"]), slot(Role::Output, &["block"])]);
        m.add_recipe("crafting", vec![slot(Role::Input, &["stick"]), slot(Role::Output, &["torch"])]);
        m.add_crafting_station("smelting", vec!["furnace".into()]);
        m
    }

    #[test]
    fn focus_lookup_by_role() {
        let m = manager();
        assert_eq!(m.recipes("crafting", &[focus(Role::Input, "ingot")], false, &all), vec![1]);
        assert_eq!(m.recipes("crafting", &[focus(Role::Output, "torch")], false, &all), vec![2]);
        assert_eq!(m.categories(&[focus(Role::Input, "ingot")], false, &all), vec!["crafting"]);
        assert_eq!(m.categories(&[focus(Role::Output, "ingot")], false, &all), vec!["smelting"]);
    }

    #[test]
    fn station_focus_shows_all_recipes_of_type() {
        let m = manager();
        assert_eq!(m.recipes("smelting", &[focus(Role::Catalyst, "furnace")], false, &all), vec![0]);
        assert_eq!(m.categories(&[focus(Role::CraftingStation, "furnace")], false, &all), vec!["smelting"]);
    }

    #[test]
    fn empty_categories_hidden_unless_asked() {
        let m = manager();
        assert_eq!(m.categories(&[], false, &all), vec!["smelting", "crafting"]);
        assert_eq!(m.categories(&[], true, &all), vec!["smelting", "crafting", "empty"]);
    }

    #[test]
    fn hidden_recipes_and_ingredients() {
        let mut m = manager();
        m.hide_recipes("crafting", &[2]);
        assert_eq!(m.recipes("crafting", &[], false, &all), vec![1]);
        assert_eq!(m.recipes("crafting", &[], true, &all), vec![1, 2]);
        // A recipe whose output is hidden is not shown.
        assert!(m.recipes("smelting", &[], false, &|u| u != "ingot").is_empty());
        // A hidden station hides its category.
        assert!(m.is_category_hidden("smelting", &[], &|u| u != "furnace"));
        m.hide_category("crafting");
        assert_eq!(m.categories(&[], false, &all), vec!["smelting"]);
    }
}
