//! The recipes JEI shows, read from the game's tables (`Host::recipes_of_type`) into `jei-core`'s recipe manager.
use crate::jei_core::{
    plugin::CategoryDef,
    recipes::{RecipeManager, Role, Slot},
    view::{self, Kind, Layout},
};
use mod_api::{ClientHost, Ingredient, ItemKey, ItemStack, Recipe, RecipeRule};
use std::collections::HashMap;

/// What a recipe's view draws: its category, the crafting grid, and the ingredients of each slot.
#[derive(Clone, Debug, Default)]
pub struct UiRecipe {
    /// The recipe's id (`minecraft:oak_planks`), for the transfer button.
    pub id: String,
    /// The category's `IRecipeType` uid.
    pub recipe_type: String,
    /// A line under the layout (a fuel's smelt count, a compostable's chance).
    pub note: String,
    pub grid: (i32, i32),
    pub inputs: Vec<Vec<String>>,
    pub outputs: Vec<String>,
}

const FUEL: &str = "jei:smelting_fuel";
const COMPOST: &str = "jei:compostable";
const ANVIL: &str = "jei:anvil";
const GRINDSTONE: &str = "jei:grindstone";

/// `NumberFormat` with at most two fraction digits.
fn trim_float(v: f32) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// A text's width for a layout: the font is the draw context's, so an estimate (6 per character) before the first frame.
fn host_text_width(_: &dyn ClientHost, text: &str) -> i32 {
    text.chars().count() as i32 * 6
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
    let cats: Vec<CategoryDef> = Kind::ALL.iter().map(|k| CategoryDef::vanilla(*k)).collect();
    for c in &cats {
        mgr.add_category(&c.recipe_type);
        mgr.add_crafting_station(&c.recipe_type, c.stations.clone());
    }
    let mut ui: Vec<UiRecipe> = Vec::new();
    let mut add = |mgr: &mut RecipeManager, u: UiRecipe| {
        let mut slots: Vec<Slot> = u.inputs.iter().map(|i| Slot { role: Role::Input, ingredients: i.clone() }).collect();
        slots.push(Slot { role: Role::Output, ingredients: u.outputs.clone() });
        if let Some(id) = mgr.add_recipe(&u.recipe_type, slots) {
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
                    UiRecipe { id: r.id.clone(), recipe_type: kind.recipe_type().into(), note: String::new(), grid, inputs, outputs: vec![out] }
                }
                Kind::Stonecutting => match &r.rule {
                    Some(RecipeRule::Stonecutting { input, result }) => {
                        UiRecipe { id: String::new(), recipe_type: kind.recipe_type().into(), note: String::new(), grid: (1, 1), inputs: vec![ids(input)], outputs: id_of(names, result).into_iter().collect() }
                    }
                    _ => continue,
                },
                Kind::Smithing => match &r.rule {
                    Some(RecipeRule::SmithingTransform { template, base, addition, result }) => UiRecipe {
                        id: String::new(), recipe_type: kind.recipe_type().into(),
                        note: String::new(),
                        grid: (3, 1),
                        inputs: vec![opt(template), ids(base), opt(addition)],
                        outputs: result.item.clone().into_iter().collect(),
                    },
                    Some(RecipeRule::SmithingTrim { template, base, addition, .. }) => UiRecipe {
                        id: String::new(), recipe_type: kind.recipe_type().into(),
                        note: String::new(),
                        grid: (3, 1),
                        inputs: vec![ids(template), ids(base), ids(addition)],
                        outputs: ids(base),
                    },
                    _ => continue,
                },
                _ => {
                    let (Some(i), Some(out)) = (r.ingredients.first(), first_result) else { continue };
                    UiRecipe { id: String::new(), recipe_type: kind.recipe_type().into(), note: String::new(), grid: (1, 1), inputs: vec![ids(i)], outputs: vec![out] }
                }
            };
            add(&mut mgr, u);
        }
    }
    // Fuel (`FuelRecipeMaker`: grouped by burn time) and composting (sorted by chance).
    let mut cats = cats;
    let register = |mgr: &mut RecipeManager, cats: &mut Vec<CategoryDef>, def: CategoryDef| {
        mgr.add_category(&def.recipe_type);
        mgr.add_crafting_station(&def.recipe_type, def.stations.clone());
        cats.push(def);
    };
    let text_w = host_text_width(host, "Smelts 10000000 items");
    register(
        &mut mgr,
        &mut cats,
        CategoryDef::custom(FUEL, "Smelting Fuel", "minecraft:furnace", Layout { width: 18 + 20 + text_w, height: 34, slots: vec![view::input(0, 1, 17)], arrow: None }),
    );
    let mut fuels: Vec<(i32, Vec<String>)> = Vec::new();
    for (key, id) in names {
        if let Some(ticks) = host.item_fuel(*key, false).map(|f| (f.ticks as f32 * f.speed) as i32).filter(|t| *t > 0) {
            match fuels.iter_mut().find(|f| f.0 == ticks) {
                Some(f) => f.1.push(id.clone()),
                None => fuels.push((ticks, vec![id.clone()])),
            }
        }
    }
    fuels.sort_by_key(|f| f.0);
    for (ticks, mut items) in fuels {
        items.sort();
        let count = ticks as f32 / 200.0;
        let note = if count == 1.0 { "Smelts 1 item".to_owned() } else { format!("Smelts {} items", trim_float(count)) };
        add(&mut mgr, UiRecipe { id: String::new(), recipe_type: FUEL.into(), note, grid: (1, 1), inputs: vec![items], outputs: vec![] });
    }
    register(
        &mut mgr,
        &mut cats,
        CategoryDef::custom(COMPOST, "Composting", "minecraft:composter", Layout { width: 120, height: 18, slots: vec![view::input(0, 1, 1)], arrow: None }),
    );
    let mut compost: Vec<(f32, String)> = names.iter().filter_map(|(k, id)| host.item_compost_chance(*k).filter(|c| *c > 0.0).map(|c| (c, id.clone()))).collect();
    compost.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    for (chance, id) in compost {
        let note = format!("Chance: {}%", (chance * 100.0).floor() as i32);
        add(&mut mgr, UiRecipe { id: String::new(), recipe_type: COMPOST.into(), note, grid: (1, 1), inputs: vec![vec![id]], outputs: vec![] });
    }
    // The anvil (`AnvilRecipeMaker`): each repairable item with its materials, each enchantable one with books.
    register(
        &mut mgr,
        &mut cats,
        CategoryDef::custom(ANVIL, "Anvil", "minecraft:anvil", Layout { width: 125, height: 18, slots: vec![view::input(0, 0, 0), view::input(1, 49, 0), view::output(107, 0)], arrow: None }),
    );
    for a in host.anvil_recipes() {
        let id_list = |v: &[ItemStack]| -> Vec<String> { v.iter().filter_map(|s| id_of(names, s)).collect() };
        let (left, right, out) = (id_list(&a.left), id_list(&a.right), id_list(&a.results));
        if left.is_empty() || right.is_empty() || out.is_empty() {
            continue;
        }
        let note = a.costs.iter().min().map(|c| format!("Cost: {c}")).unwrap_or_default();
        add(&mut mgr, UiRecipe { id: String::new(), recipe_type: ANVIL.into(), note, grid: (1, 1), inputs: vec![left, right], outputs: out });
    }
    // Brewing (`BrewingRecipeMaker`): one recipe per ingredient item with the bottles it turns and what they become.
    // Potions share one item id, so the variants of a bottle are not told apart yet.
    let mut brewing: Vec<(String, Vec<String>, Vec<String>)> = Vec::new();
    for b in host.brewing_recipes() {
        let list = |v: &[ItemStack]| -> Vec<String> { v.iter().filter_map(|s| id_of(names, s)).collect() };
        let Some(ingredient) = list(&b.ingredient).into_iter().next() else { continue };
        let (inputs, outputs) = (list(&b.input), list(&b.output));
        let entry = match brewing.iter_mut().position(|e| e.0 == ingredient) {
            Some(i) => &mut brewing[i],
            None => {
                brewing.push((ingredient, Vec::new(), Vec::new()));
                brewing.last_mut().unwrap()
            }
        };
        for (src, dst) in [(inputs, &mut entry.1), (outputs, &mut entry.2)] {
            for id in src {
                if !dst.contains(&id) {
                    dst.push(id);
                }
            }
        }
    }
    for (ingredient, inputs, outputs) in brewing {
        add(&mut mgr, UiRecipe { id: String::new(), recipe_type: Kind::Brewing.recipe_type().into(), note: String::new(), grid: (1, 1), inputs: vec![inputs.clone(), inputs.clone(), inputs, vec![ingredient]], outputs });
    }
    // The grindstone (`GrindstoneRecipeMaker`): repairs and disenchanting, with the experience range.
    register(
        &mut mgr,
        &mut cats,
        CategoryDef::custom(GRINDSTONE, "Grindstone", "minecraft:grindstone", Layout { width: 116, height: 61, slots: vec![view::input(0, 18, 3), view::input(1, 18, 24), view::output(98, 19)], arrow: None }),
    );
    for g in host.grindstone_recipes() {
        let list = |v: &[ItemStack]| -> Vec<String> { v.iter().filter_map(|s| id_of(names, s)).collect() };
        let (top, bottom, out) = (list(&g.top), list(&g.bottom), list(&g.output));
        if top.is_empty() && bottom.is_empty() || out.is_empty() {
            continue;
        }
        let note = if g.xp_max > 0 { format!("XP: {}-{}", g.xp_min, g.xp_max) } else { String::new() };
        add(&mut mgr, UiRecipe { id: String::new(), recipe_type: GRINDSTONE.into(), note, grid: (1, 1), inputs: vec![top, bottom], outputs: out });
    }
    // Categories and recipes other mods registered through `api` (`IModPlugin`).
    let plugins = crate::api::snapshot();
    for def in plugins.categories {
        if !cats.iter().any(|c| c.recipe_type == def.recipe_type) {
            register(&mut mgr, &mut cats, def);
        }
    }
    for r in plugins.recipes {
        let id_of = |s: &String| crate::jei_core::plugin::split_ingredient(s).0.to_owned();
        let inputs: Vec<Vec<String>> = r.inputs.iter().map(|slot| slot.iter().map(id_of).collect()).collect();
        let outputs: Vec<String> = r.outputs.first().map(|slot| slot.iter().map(id_of).collect()).unwrap_or_default();
        add(&mut mgr, UiRecipe { id: String::new(), recipe_type: r.recipe_type, note: r.notes.join(" "), grid: (1, 1), inputs, outputs });
    }
    Recipes { mgr, cats, ui }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_floats() {
        assert_eq!(trim_float(1.0), "1");
        assert_eq!(trim_float(1.5), "1.5");
        assert_eq!(trim_float(0.125), "0.12"); // half-even, as `NumberFormat`
        assert_eq!(trim_float(7.5), "7.5");
    }
}
