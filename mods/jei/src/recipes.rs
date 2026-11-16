//! The recipes JEI shows, read from the game's tables (`Host::recipes_of_type`) into `jei-core`'s recipe manager.
use jei_core::{
    plugin::CategoryDef,
    recipes::{RecipeManager, Role, Slot},
    view::Kind,
};
use mod_api::{ClientHost, Ingredient, ItemKey, ItemStack, Recipe, RecipeRule};
use std::collections::HashMap;

/// What a recipe's view draws: its category, the crafting grid, and the ingredients of each slot.
#[derive(Clone, Debug, Default)]
pub struct UiRecipe {
    pub kind: Option<Kind>,
    pub grid: (i32, i32),
    pub inputs: Vec<Vec<String>>,
    pub outputs: Vec<String>,
}

pub struct Recipes {
    pub mgr: RecipeManager,
    pub cats: Vec<CategoryDef>,
    /// Indexed by `RecipeId`.
    pub ui: Vec<UiRecipe>,
}

fn ids(i: &Ingredient) -> Vec<String> {
    if !i.order.is_empty() {
        return i.order.clone();
    }
    let mut v: Vec<String> = i.items.iter().cloned().collect();
    v.sort();
    v
}

fn opt(i: &Option<Ingredient>) -> Vec<String> {
    i.as_ref().map(ids).unwrap_or_default()
}

fn id_of(names: &HashMap<ItemKey, String>, s: &ItemStack) -> Option<String> {
    names.get(&s.item()).cloned()
}

/// A shaped recipe's grid, from its file: `pattern` rows and the `key` of each letter (empty cells keep their slot).
fn shaped(host: &dyn ClientHost, r: &Recipe) -> Option<((i32, i32), Vec<Vec<String>>)> {
    let pattern = r.json.get("pattern")?.as_array()?;
    let key = r.json.get("key")?.as_object()?;
    let rows: Vec<&str> = pattern.iter().filter_map(|p| p.as_str()).collect();
    let width = rows.iter().map(|p| p.chars().count()).max()? as i32;
    let mut slots = Vec::new();
    for row in &rows {
        let mut cells: Vec<char> = row.chars().collect();
        cells.resize(width as usize, ' ');
        for c in cells {
            let list = key
                .get(&c.to_string())
                .and_then(|v| host.recipe_ingredient(&v.to_string()).ok().flatten())
                .map(|i| ids(&i))
                .unwrap_or_default();
            slots.push(list);
        }
    }
    Some(((width, rows.len() as i32), slots))
}

pub fn load(host: &dyn ClientHost, names: &HashMap<ItemKey, String>) -> Recipes {
    let mut mgr = RecipeManager::new();
    let cats: Vec<CategoryDef> = Kind::ALL.iter().filter(|k| **k != Kind::Brewing).map(|k| CategoryDef::vanilla(*k)).collect();
    for c in &cats {
        mgr.add_category(&c.recipe_type);
        mgr.add_crafting_station(&c.recipe_type, c.stations.clone());
    }
    let mut ui: Vec<UiRecipe> = Vec::new();
    let mut add = |mgr: &mut RecipeManager, u: UiRecipe| {
        let Some(kind) = u.kind else { return };
        let mut slots: Vec<Slot> = u.inputs.iter().map(|i| Slot { role: Role::Input, ingredients: i.clone() }).collect();
        slots.push(Slot { role: Role::Output, ingredients: u.outputs.clone() });
        if let Some(id) = mgr.add_recipe(kind.recipe_type(), slots) {
            if ui.len() <= id as usize {
                ui.resize(id as usize + 1, UiRecipe::default());
            }
            ui[id as usize] = u;
        }
    };
    for kind in [Kind::Crafting, Kind::Smelting, Kind::Blasting, Kind::Smoking, Kind::Campfire, Kind::Stonecutting, Kind::Smithing] {
        for r in host.recipes_of_type(kind.recipe_type()).iter() {
            let first_result = r.results.first().and_then(|s| id_of(names, s));
            let u = match kind {
                Kind::Crafting => {
                    let Some(out) = first_result else { continue };
                    let (grid, inputs) = match shaped(host, r) {
                        Some(s) => s,
                        None => {
                            let inputs: Vec<Vec<String>> = r.ingredients.iter().map(ids).collect();
                            let n = inputs.len() as i32;
                            ((n.clamp(1, 3), (n + 2) / 3), inputs)
                        }
                    };
                    UiRecipe { kind: Some(kind), grid, inputs, outputs: vec![out] }
                }
                Kind::Stonecutting => match &r.rule {
                    Some(RecipeRule::Stonecutting { input, result }) => {
                        UiRecipe { kind: Some(kind), grid: (1, 1), inputs: vec![ids(input)], outputs: id_of(names, result).into_iter().collect() }
                    }
                    _ => continue,
                },
                Kind::Smithing => match &r.rule {
                    Some(RecipeRule::SmithingTransform { template, base, addition, result }) => UiRecipe {
                        kind: Some(kind),
                        grid: (3, 1),
                        inputs: vec![opt(template), ids(base), opt(addition)],
                        outputs: result.item.clone().into_iter().collect(),
                    },
                    Some(RecipeRule::SmithingTrim { template, base, addition, .. }) => UiRecipe {
                        kind: Some(kind),
                        grid: (3, 1),
                        inputs: vec![ids(template), ids(base), ids(addition)],
                        outputs: ids(base),
                    },
                    _ => continue,
                },
                _ => {
                    let (Some(i), Some(out)) = (r.ingredients.first(), first_result) else { continue };
                    UiRecipe { kind: Some(kind), grid: (1, 1), inputs: vec![ids(i)], outputs: vec![out] }
                }
            };
            add(&mut mgr, u);
        }
    }
    Recipes { mgr, cats, ui }
}
