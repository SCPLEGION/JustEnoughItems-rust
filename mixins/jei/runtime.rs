// JEI's glue inside Horizon's game crate (`crate::game::jei`). The logic lives in the `jei_core` crate; this module reads
// the game's items and recipes (`native_recipes`), draws the ingredient list beside container screens
// (`native_container_*`, API 87) and the recipe view, and hears clicks, keys and the wheel.

use super::native_hooks::{NativeRecipeKind, NativeRecipes};
use super::Game;
use horizon_client::hud::HudBatch;
use jei_core::bookmarks::BookmarkList;
use jei_core::grid::{self, GridConfig, Rect};
use jei_core::plugin::{CategoryDef, CustomRecipe, PluginError, Registry};
use jei_core::ingredient::{FilterConfig, IngredientData, ListElementInfo};
use jei_core::prefix::{default_mode, SearchIndex};
use jei_core::recipes::{Focus, RecipeId, RecipeManager, Role, Slot};
use jei_core::search::ElementSearch;
use jei_core::view::{Kind, SlotRole};
use std::cell::RefCell;
use std::sync::Arc;
use std::time::Instant;

const MARGIN: i32 = 4;
const NAV_H: i32 = 20;
const SEARCH_H: i32 = 20;
const TEXT: u32 = 0xFFFFFF;
const BG: u32 = 0xC0101010;
const SLOT_BG: u32 = 0xFF8B8B8B;
const HOVER: u32 = 0x80FFFFFF;
/// How long a slot shows one of its ingredients before the next (JEI cycles each second).
const CYCLE_MS: u128 = 1000;

/// Where a recipe of the manager comes from.
#[derive(Clone, Copy)]
enum Src {
    Native(usize),
    Custom(usize),
}

struct View {
    /// Indexes into `State::cats`.
    cats: Vec<usize>,
    cat: usize,
    page: usize,
    recipes: Vec<RecipeId>,
    focus: Focus,
}

#[derive(Default, Clone, Copy)]
struct Places {
    grid: Rect,
    nav: Rect,
    search: Rect,
    /// The recipe view's box and the slots drawn in it: (rect, item index, role of a click on it).
    panel: Rect,
    /// The bookmark list's grid.
    marks: Rect,
}

struct State {
    data: Option<Arc<NativeRecipes>>,
    infos: Vec<ListElementInfo>,
    search: ElementSearch,
    mgr: RecipeManager,
    /// Recipe id -> index into `data.recipes`.
    recipe_of: Vec<Src>,
    cats: Vec<CategoryDef>,
    bookmarks: BookmarkList,
    plugin_version: u64,
    /// The transfer button of the recipe shown: its rect and the recipe book id.
    transfer: Option<(Rect, i32)>,
    query: String,
    focused: bool,
    results: Vec<usize>,
    page: i32,
    view: Option<View>,
    places: Places,
    /// (rect, item id) of the hoverable stacks drawn this frame.
    hot: Vec<(Rect, String)>,
    started: Instant,
    cfg: FilterConfig,
}

impl State {
    fn new() -> Self {
        Self {
            data: None,
            infos: Vec::new(),
            search: ElementSearch::new(default_mode),
            mgr: RecipeManager::new(),
            recipe_of: Vec::new(),
            cats: Vec::new(),
            bookmarks: BookmarkList::new(),
            plugin_version: 0,
            transfer: None,
            query: String::new(),
            focused: false,
            results: Vec::new(),
            page: 0,
            view: None,
            places: Places::default(),
            hot: Vec::new(),
            started: Instant::now(),
            cfg: FilterConfig::default(),
        }
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::new());
    static PLUGINS: RefCell<Registry> = RefCell::new(Registry::default());
}

/// `IRecipeCategoryRegistration`: another mod's recipe category (call from its own glue, at start).
#[allow(dead_code)]
pub fn register_category(def: CategoryDef) -> Result<(), PluginError> {
    PLUGINS.with_borrow_mut(|p| p.register_category(def))
}
/// `IRecipeCatalystRegistration`.
#[allow(dead_code)]
pub fn add_catalyst(recipe_type: &str, item: &str) -> Result<(), PluginError> {
    PLUGINS.with_borrow_mut(|p| p.add_catalyst(recipe_type, item))
}
/// `IRecipeRegistration.addRecipes`.
#[allow(dead_code)]
pub fn add_recipe(recipe: CustomRecipe) -> Result<(), PluginError> {
    PLUGINS.with_borrow_mut(|p| p.add_recipe(recipe))
}

fn mod_name(namespace: &str) -> String {
    namespace
        .split('_')
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn kind_of(k: NativeRecipeKind) -> Kind {
    match k {
        NativeRecipeKind::CraftingShaped | NativeRecipeKind::CraftingShapeless => Kind::Crafting,
        NativeRecipeKind::Smelting => Kind::Smelting,
        NativeRecipeKind::Blasting => Kind::Blasting,
        NativeRecipeKind::Smoking => Kind::Smoking,
        NativeRecipeKind::Campfire => Kind::Campfire,
        NativeRecipeKind::Stonecutting => Kind::Stonecutting,
        NativeRecipeKind::Smithing => Kind::Smithing,
        NativeRecipeKind::Brewing => Kind::Brewing,
    }
}

/// Builds JEI's indexes from the game's items and recipes, once per set of them.
fn load(state: &mut State, data: Arc<NativeRecipes>) {
    let plugins = PLUGINS.with_borrow(Clone::clone);
    if plugins.version == state.plugin_version && state.data.as_ref().is_some_and(|d| Arc::ptr_eq(d, &data)) {
        return;
    }
    state.plugin_version = plugins.version;
    let cfg = state.cfg;
    state.infos.clear();
    state.search = ElementSearch::new(default_mode);
    for (i, item) in data.items.iter().enumerate() {
        let (ns, path) = item.id.split_once(':').unwrap_or(("minecraft", item.id.as_str()));
        let d = IngredientData {
            display_name: item.name.clone(),
            namespace: ns.to_owned(),
            path: path.to_owned(),
            display_mod_id: ns.to_owned(),
            mod_name: mod_name(ns),
            display_mod_name: mod_name(ns),
            mod_aliases: vec![vec![]],
            tags: item.tags.clone(),
            ..Default::default()
        };
        let info = ListElementInfo::new(d, &cfg, i);
        state.search.add(i, |idx: SearchIndex| info.strings(idx, &cfg));
        state.infos.push(info);
    }
    let mut mgr = RecipeManager::new();
    let mut cats: Vec<CategoryDef> = Kind::ALL.iter().map(|k| CategoryDef::vanilla(*k)).collect();
    cats.extend(plugins.categories.iter().cloned());
    for c in &cats {
        mgr.add_category(&c.recipe_type);
        mgr.add_crafting_station(&c.recipe_type, c.stations.clone());
    }
    let mut recipe_of = Vec::new();
    for (n, r) in data.recipes.iter().enumerate() {
        let kind = kind_of(r.kind);
        let mut slots: Vec<Slot> = r
            .slots
            .iter()
            .map(|s| Slot { role: Role::Input, ingredients: s.iter().filter_map(|x| data.items.get(x.item as usize)).map(|i| i.id.clone()).collect() })
            .collect();
        slots.push(Slot {
            role: Role::Output,
            ingredients: r.results.iter().filter_map(|x| data.items.get(x.item as usize)).map(|i| i.id.clone()).collect(),
        });
        if let Some(id) = mgr.add_recipe(kind.recipe_type(), slots) {
            if recipe_of.len() <= id as usize {
                recipe_of.resize(id as usize + 1, Src::Native(0));
            }
            recipe_of[id as usize] = Src::Native(n);
        }
    }
    for (n, r) in plugins.recipes.iter().enumerate() {
        let mut slots: Vec<Slot> = r.inputs.iter().map(|i| Slot { role: Role::Input, ingredients: i.clone() }).collect();
        slots.extend(r.outputs.iter().map(|o| Slot { role: Role::Output, ingredients: o.clone() }));
        if let Some(id) = mgr.add_recipe(&r.recipe_type, slots) {
            if recipe_of.len() <= id as usize {
                recipe_of.resize(id as usize + 1, Src::Native(0));
            }
            recipe_of[id as usize] = Src::Custom(n);
        }
    }
    state.cats = cats;
    state.mgr = mgr;
    state.recipe_of = recipe_of;
    state.data = Some(data);
    refilter(state);
}

fn refilter(state: &mut State) {
    state.results = state.search.query(&state.query);
    state.page = 0;
}

/// Every tick: the data, once there is a world.
pub(super) fn tick(game: &mut Game<'_>) {
    let version = PLUGINS.with_borrow(|p| p.version);
    let needs = STATE.with_borrow(|s| s.data.is_none() || s.plugin_version != version);
    if needs && let Some(d) = game.native_recipes() {
        STATE.with_borrow_mut(|s| load(s, d));
    }
}

fn rect_f(r: Rect) -> [f32; 4] {
    [r.x as f32, r.y as f32, r.w as f32, r.h as f32]
}

fn open_recipes(state: &mut State, item: &str, role: Role) {
    let focus = Focus { role, uid: item.to_owned() };
    let cats: Vec<usize> = state
        .mgr
        .categories(std::slice::from_ref(&focus), false, &|_| true)
        .iter()
        .filter_map(|t| state.cats.iter().position(|c| &c.recipe_type == t))
        .collect();
    if cats.is_empty() {
        return;
    }
    let recipes = state.mgr.recipes(&state.cats[cats[0]].recipe_type, std::slice::from_ref(&focus), false, &|_| true);
    state.view = Some(View { cats, cat: 0, page: 0, recipes, focus });
}

fn select_category(state: &mut State, cat: usize) {
    let Some(v) = state.view.as_mut() else { return };
    v.cat = cat;
    v.page = 0;
    v.recipes = state.mgr.recipes(&state.cats[v.cats[cat]].recipe_type, std::slice::from_ref(&v.focus), false, &|_| true);
}

fn text_w(game: &Game<'_>, t: &str) -> i32 {
    game.native_text_width(t) as i32
}

pub(super) fn container_draw(game: &Game<'_>, window: [f32; 4], batch: &mut HudBatch) {
    STATE.with_borrow_mut(|s| {
        s.hot.clear();
        if s.data.is_none() {
            return;
        }
        let (bw, bh) = (batch.width as i32, batch.height as i32);
        let left = (window[0] + window[2]) as i32 + MARGIN;
        let area = Rect::new(left, NAV_H, bw - left - MARGIN, bh - NAV_H - SEARCH_H - 2 * MARGIN);
        let bounds = grid::calculate_bounds(&GridConfig::ingredient_list(), area, false);
        let search = Rect::new(bounds.x.max(left), bh - SEARCH_H - MARGIN, bounds.w.max(80), SEARCH_H);
        let nav = Rect::new(bounds.x, 2, bounds.w, 16);
        s.places = Places { grid: bounds, nav, search, panel: Rect::EMPTY, marks: Rect::EMPTY };
        if bounds.is_empty() {
            return;
        }
        let cursor = game.native_cursor();
        let (cx, cy) = (cursor[0] as i32, cursor[1] as i32);
        let slots = grid::calculate_slots(bounds, &[], None, 0);
        let per_page = slots.len() as i32;
        let first = grid::first_item_index_for_valid_page(s.page * per_page, s.results.len() as i32, per_page);
        s.page = if per_page == 0 { 0 } else { first / per_page };
        for (n, slot) in slots.iter().enumerate() {
            let Some(&idx) = s.results.get(first as usize + n) else { break };
            let Some(item) = s.data.as_ref().and_then(|d| d.items.get(idx)) else { continue };
            let a = slot.area;
            game.native_item(batch, &item.id, [(a.x + 1) as f32, (a.y + 1) as f32], 16.0);
            if a.contains(cx, cy) {
                game.native_fill(batch, rect_f(Rect::new(a.x + 1, a.y + 1, 16, 16)), HOVER);
            }
            s.hot.push((a, item.id.clone()));
        }
        // The bookmark list, left of the window.
        let marks_area = Rect::new(MARGIN, NAV_H, window[0] as i32 - 2 * MARGIN, bh - NAV_H - 2 * MARGIN);
        let marks = grid::calculate_bounds(&GridConfig::bookmark_list(), marks_area, false);
        s.places.marks = marks;
        if !marks.is_empty() {
            let mark_slots = grid::calculate_slots(marks, &[], None, 0);
            for (slot, id) in mark_slots.iter().zip(s.bookmarks.items().to_vec()) {
                let a = slot.area;
                game.native_fill(batch, rect_f(Rect::new(a.x + 1, a.y + 1, 16, 16)), 0x40FFFF00);
                game.native_item(batch, &id, [(a.x + 1) as f32, (a.y + 1) as f32], 16.0);
                if a.contains(cx, cy) {
                    game.native_fill(batch, rect_f(Rect::new(a.x + 1, a.y + 1, 16, 16)), HOVER);
                }
                s.hot.push((a, id));
            }
        }
        // Navigation: `<  page/pages  >`.
        let pages = grid::page_count(s.results.len() as i32, per_page);
        let label = format!("{}/{}", s.page + 1, pages);
        let lw = text_w(game, &label);
        game.native_text(batch, "<", [(nav.x + 4) as f32, (nav.y + 4) as f32], TEXT, true);
        game.native_text(batch, ">", [(nav.x + nav.w - 10) as f32, (nav.y + 4) as f32], TEXT, true);
        game.native_text(batch, &label, [(nav.x + (nav.w - lw) / 2) as f32, (nav.y + 4) as f32], TEXT, true);
        // Search box.
        game.native_fill(batch, rect_f(search), 0xFF000000);
        game.native_fill(batch, rect_f(Rect::new(search.x + 1, search.y + 1, search.w - 2, search.h - 2)), if s.focused { 0xFF202020 } else { 0xFF101010 });
        let shown = if s.query.is_empty() && !s.focused { "Search...".to_owned() } else if s.focused && (s.started.elapsed().as_millis() / 500) % 2 == 0 { format!("{}_", s.query) } else { s.query.clone() };
        game.native_text(batch, &shown, [(search.x + 4) as f32, (search.y + 6) as f32], if s.query.is_empty() { 0x909090 } else { TEXT }, false);
        // Tooltip of the hovered stack.
        if s.view.is_none() {
            let hovered = s.hot.iter().find(|(r, _)| r.contains(cx, cy)).map(|(_, id)| id.clone());
            if let Some(id) = hovered {
                tooltip(game, batch, s, &id, cx, cy);
            }
        }
        if s.view.is_some() {
            draw_view(game, batch, s, cx, cy);
        }
    });
}

fn name_of(s: &State, id: &str) -> String {
    s.data.as_ref().and_then(|d| d.items.iter().find(|i| i.id == id)).map(|i| i.name.clone()).unwrap_or_else(|| id.to_owned())
}

fn tooltip(game: &Game<'_>, batch: &mut HudBatch, s: &State, id: &str, x: i32, y: i32) {
    let name = name_of(s, id);
    let w = text_w(game, &name).max(text_w(game, id));
    let (bx, by) = (x + 12, y - 12);
    game.native_fill(batch, rect_f(Rect::new(bx - 3, by - 3, w + 6, 26)), 0xF0100010);
    game.native_text(batch, &name, [bx as f32, by as f32], TEXT, true);
    game.native_text(batch, id, [bx as f32, (by + 11) as f32], 0x5555FF, true);
}

fn cycle(s: &State, list: &[String]) -> Option<String> {
    if list.is_empty() {
        return None;
    }
    Some(list[(s.started.elapsed().as_millis() / CYCLE_MS) as usize % list.len()].clone())
}

fn stacks(s: &State, r: &[super::native_hooks::NativeRecipeStack]) -> Vec<String> {
    let Some(d) = s.data.as_ref() else { return vec![] };
    r.iter().filter_map(|x| d.items.get(x.item as usize)).map(|i| i.id.clone()).collect()
}

fn draw_view(game: &Game<'_>, batch: &mut HudBatch, s: &mut State, cx: i32, cy: i32) {
    let Some(v) = s.view.as_ref() else { return };
    let Some(data) = s.data.clone() else { return };
    let cat = s.cats[v.cats[v.cat]].clone();
    let (bw, bh) = (batch.width as i32, batch.height as i32);
    let pages = v.recipes.len().max(1);
    let src = v.recipes.get(v.page).and_then(|id| s.recipe_of.get(*id as usize)).copied();
    let native = match src {
        Some(Src::Native(n)) => data.recipes.get(n),
        _ => None,
    };
    let custom = match src {
        Some(Src::Custom(n)) => PLUGINS.with_borrow(|p| p.recipes.get(n).cloned()),
        _ => None,
    };
    let (gw, gh) = native.map_or((3, 3), |r| (r.width as i32, r.height as i32));
    let inputs = native.map_or(0, |r| r.slots.len());
    let lay = cat.layout_for((gw, gh), inputs);
    let pw = (lay.width + 24).max(176);
    let ph = lay.height + 78;
    let panel = Rect::new((bw - pw) / 2, (bh - ph) / 2, pw, ph);
    s.places.panel = panel;
    game.native_fill(batch, rect_f(panel), BG);
    game.native_fill(batch, rect_f(Rect::new(panel.x + 1, panel.y + 1, panel.w - 2, panel.h - 2)), 0xFFC6C6C6);
    // Category tabs along the top, the title under them.
    for (i, ci) in v.cats.iter().enumerate() {
        let t = Rect::new(panel.x + 4 + i as i32 * 22, panel.y + 4, 20, 20);
        game.native_fill(batch, rect_f(t), if i == v.cat { 0xFFFFFFFF } else { 0xFF8B8B8B });
        game.native_item(batch, &s.cats[*ci].icon, [(t.x + 2) as f32, (t.y + 2) as f32], 16.0);
    }
    game.native_text(batch, &cat.title, [(panel.x + 6) as f32, (panel.y + 28) as f32], 0x404040, false);
    let origin = (panel.x + (pw - lay.width) / 2, panel.y + 42);
    let mut hot = Vec::new();
    s.transfer = None;
    if src.is_some() {
        if let Some((ax, ay)) = lay.arrow {
            game.native_text(batch, "->", [(origin.0 + ax) as f32, (origin.1 + ay + 4) as f32], 0x404040, false);
        }
        let result_list = native.map(|r| stacks(s, &r.results)).unwrap_or_default();
        for slot in &lay.slots {
            let rect = Rect::new(origin.0 + slot.x, origin.1 + slot.y, 18, 18);
            game.native_fill(batch, rect_f(rect), SLOT_BG);
            let list: Vec<String> = match (slot.role, native, custom.as_ref()) {
                (SlotRole::Input, Some(r), _) => {
                    // Brewing: the bottle fills three slots, the reagent the fourth.
                    let at = if cat.vanilla == Some(Kind::Brewing) { usize::from(slot.input >= 3) } else { slot.input };
                    r.slots.get(at).map(|x| stacks(s, x)).unwrap_or_default()
                }
                (SlotRole::Input, None, Some(c)) => c.inputs.get(slot.input).cloned().unwrap_or_default(),
                (SlotRole::Output, Some(_), _) => result_list.clone(),
                (SlotRole::Output, None, Some(c)) => c.outputs.get(slot.input).cloned().unwrap_or_default(),
                (SlotRole::Fuel, _, _) => vec!["minecraft:coal".to_owned()],
                _ => vec![],
            };
            if let Some(id) = cycle(s, &list) {
                game.native_item(batch, &id, [(rect.x + 1) as f32, (rect.y + 1) as f32], 16.0);
                if rect.contains(cx, cy) {
                    game.native_fill(batch, rect_f(Rect::new(rect.x + 1, rect.y + 1, 16, 16)), HOVER);
                }
                hot.push((rect, id));
            }
        }
        // Recipe transfer (`+`): the recipe book places crafting recipes in the open grid.
        if let Some(r) = native.filter(|r| r.book_id >= 0 && cat.vanilla == Some(Kind::Crafting)) {
            let b = Rect::new(origin.0 + lay.width - 13, origin.1 + lay.height - 13, 13, 13);
            game.native_fill(batch, rect_f(b), if b.contains(cx, cy) { 0xFF55FF55 } else { 0xFF339933 });
            game.native_text(batch, "+", [(b.x + 4) as f32, (b.y + 3) as f32], TEXT, true);
            s.transfer = Some((b, r.book_id));
        }
    }
    let label = format!("{}/{}", v.page + 1, pages);
    game.native_text(batch, "<", [(panel.x + 8) as f32, (panel.y + ph - 14) as f32], 0x404040, false);
    game.native_text(batch, ">", [(panel.x + pw - 14) as f32, (panel.y + ph - 14) as f32], 0x404040, false);
    game.native_text(batch, &label, [(panel.x + (pw - text_w(game, &label)) / 2) as f32, (panel.y + ph - 14) as f32], 0x404040, false);
    if let Some((_, id)) = hot.iter().find(|(r, _)| r.contains(cx, cy)) {
        let id = id.clone();
        tooltip(game, batch, s, &id, cx, cy);
    }
    s.hot = hot;
}

pub(super) fn container_click(game: &mut Game<'_>, window: [f32; 4], at: [f32; 2], button: u8, claimed: &mut bool) {
    let _ = window;
    let (x, y) = (at[0] as i32, at[1] as i32);
    STATE.with_borrow_mut(|s| {
        if s.data.is_none() {
            return;
        }
        let right = button == 1;
        if s.view.is_some() {
            let p = s.places.panel;
            if !p.contains(x, y) {
                s.view = None;
                *claimed = true;
                return;
            }
            *claimed = true;
            if let Some((b, book_id)) = s.transfer && b.contains(x, y) {
                // Left places the recipe once, right as many as the items allow (JEI: Shift).
                let _ = game.native_place_recipe(book_id, right);
                s.view = None;
                return;
            }
            // Tabs.
            let cats = s.view.as_ref().map_or(0, |v| v.cats.len());
            for i in 0..cats {
                if Rect::new(p.x + 4 + i as i32 * 22, p.y + 4, 20, 20).contains(x, y) {
                    select_category(s, i);
                    return;
                }
            }
            // Pages.
            if y >= p.y + p.h - 18 {
                if let Some(v) = s.view.as_mut() {
                    if x < p.x + p.w / 2 {
                        v.page = v.page.saturating_sub(1);
                    } else if v.page + 1 < v.recipes.len() {
                        v.page += 1;
                    }
                }
                return;
            }
            // A stack: left shows how it is made, right what it is used in.
            if let Some((_, id)) = s.hot.iter().find(|(r, _)| r.contains(x, y)).cloned() {
                open_recipes(s, &id, if right { Role::Input } else { Role::Output });
            }
            return;
        }
        let pl = s.places;
        if pl.search.contains(x, y) {
            s.focused = true;
            *claimed = true;
            if right {
                s.query.clear();
                refilter(s);
            }
            return;
        }
        s.focused = false;
        if pl.grid.is_empty() {
            return;
        }
        if pl.nav.contains(x, y) {
            let per = grid::calculate_slots(pl.grid, &[], None, 0).len() as i32;
            let pages = grid::page_count(s.results.len() as i32, per);
            if x < pl.nav.x + pl.nav.w / 2 {
                s.page = (s.page - 1).rem_euclid(pages);
            } else {
                s.page = (s.page + 1) % pages;
            }
            *claimed = true;
            return;
        }
        if let Some((_, id)) = s.hot.iter().find(|(r, _)| r.contains(x, y)).cloned() {
            open_recipes(s, &id, if right { Role::Input } else { Role::Output });
            *claimed = true;
        }
    });
}

pub(super) fn container_key(game: &mut Game<'_>, key: &str, text: Option<&str>, pressed: bool, claimed: &mut bool) {
    let hovered = if pressed { game.native_hovered_stack().map(|s| s.item.to_string()) } else { None };
    STATE.with_borrow_mut(|s| {
        if s.data.is_none() {
            return;
        }
        if s.focused {
            *claimed = true;
            if !pressed {
                return;
            }
            match key {
                "key.keyboard.backspace" => {
                    s.query.pop();
                    refilter(s);
                }
                "key.keyboard.enter" | "key.keyboard.keypad.enter" => s.focused = false,
                _ => {
                    if let Some(t) = text {
                        s.query.push_str(t);
                        s.query.truncate(50);
                        refilter(s);
                    }
                }
            }
            return;
        }
        if !pressed {
            return;
        }
        if key == "key.keyboard.a" {
            // `jei.key.bookmark` (A): bookmark the stack under the cursor, or remove it.
            let cursor = game.native_cursor();
            let (cx, cy) = (cursor[0] as i32, cursor[1] as i32);
            let over = s.hot.iter().find(|(r, _)| r.contains(cx, cy)).map(|(_, id)| id.clone());
            if let Some(id) = over.or(hovered) {
                s.bookmarks.toggle(&id);
                *claimed = true;
            }
            return;
        }
        // `jei.key.showRecipe` (R) and `jei.key.showUses` (U) on the stack under the cursor.
        let role = match key {
            "key.keyboard.r" => Role::Output,
            "key.keyboard.u" => Role::Input,
            _ => return,
        };
        let cursor = game.native_cursor();
        let (cx, cy) = (cursor[0] as i32, cursor[1] as i32);
        let over = s.hot.iter().find(|(r, _)| r.contains(cx, cy)).map(|(_, id)| id.clone());
        if let Some(id) = over.or(hovered) {
            open_recipes(s, &id, role);
            *claimed = s.view.is_some();
        }
    });
}

pub(super) fn container_wheel(game: &mut Game<'_>, at: [f32; 2], steps: f32, claimed: &mut bool) {
    let _ = game;
    let (x, y) = (at[0] as i32, at[1] as i32);
    STATE.with_borrow_mut(|s| {
        let pl = s.places;
        if s.data.is_none() || !(pl.grid.contains(x, y) || pl.nav.contains(x, y)) {
            return;
        }
        let per = grid::calculate_slots(pl.grid, &[], None, 0).len() as i32;
        let pages = grid::page_count(s.results.len() as i32, per);
        s.page = (s.page + if steps > 0.0 { -1 } else { 1 }).rem_euclid(pages);
        *claimed = true;
    });
}
