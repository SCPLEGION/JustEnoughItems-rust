//! JEI on Horizon's own mod API: the ingredient list over container screens. All logic is `jei-core`.
use jei_core::{
    grid::{self, GridConfig, Rect},
    overlay_layout, recipe_gui,
    recipes::{Focus, RecipeId, Role},
    view::SlotRole,
    bookmarks::BookmarkList,
    highlight, keys,
    completion::{self, Candidate, PrefixView},
    config::Config,
    history::LookupHistory,
    ingredient::{FilterConfig, IngredientData, ListElementInfo},
    prefix::{default_mode, SearchIndex, SearchMode, PREFIXES},
    search::ElementSearch,
};
use mod_api::{
    self, ClientHost, DrawContext, Hooks, ItemKey, ItemStack, ScreenCharTypedPreEvent, ScreenKeyPressedPreEvent,
    ScreenMouseButtonPressedPreEvent, ScreenMouseScrolledPreEvent, ClientTickPostEvent, Flow, ScreenRenderPostEvent, ScreenView, Text,
};
pub mod api;
mod recipes;
mod ui;

use std::{cell::RefCell, collections::HashMap, time::Instant};

const PAGE_NAV_BG: u32 = 0x30000000;
const TEXT: u32 = 0xFFFFFFFF;
const HOVER: u32 = 0x80FFFFFF;

#[derive(Default)]
struct State {
    loaded: bool,
    keys: Vec<ItemKey>,
    ids: Vec<String>,
    infos: Vec<ListElementInfo>,
    search: Option<ElementSearch>,
    results: Vec<usize>,
    query: String,
    focused: bool,
    page: i32,
    /// The list's grid and the search box, from the last frame.
    grid: Rect,
    search_box: Rect,
    nav: Rect,
    /// A give the click asked for, done in the next client tick (a screen hook has no mutable host).
    give: Option<(ItemKey, u32)>,
    recipes: Option<recipes::Recipes>,
    index: HashMap<String, usize>,
    view: Option<View>,
    history: Vec<View>,
    /// The ingredients drawn this frame and where, for clicks.
    hot: Vec<(Rect, String)>,
    panel: Rect,
    started: Option<Instant>,
    /// Cheat mode: a click on a listed item gives it (`jei.cheatMode`).
    cheat: bool,
    settings_loaded: bool,
    bookmarks: BookmarkList,
    /// Hidden ingredients (`EditModeConfig`): an item id, or `*` and an id for the item's variants.
    hidden: Vec<String>,
    /// Hide ingredients mode (per session).
    edit: bool,
    modes: Vec<(SearchIndex, SearchMode)>,
    options: bool,
    options_button: Rect,
    columns: i32,
    rows: i32,
    marks: Rect,
    /// The "+" of the recipe shown: its rect and the recipe id.
    hits: Vec<(Rect, Hit)>,
    per_page: i32,
    /// Recipe columns of the recipe screen, the step of the scrolled mode.
    recipe_columns: i32,
    tab_page: Option<usize>,
    /// A transfer the click asked for (recipe, all sets), done in the next client tick.
    pending_transfer: Option<(String, bool)>,
    screen_size: [i32; 2],
    /// The item keys by id, kept to rebuild the recipes when a plugin registers more.
    names: HashMap<ItemKey, String>,
    plugin_version: u64,
    /// `jei-client.toml`, kept as one setting.
    config: Config,
    /// The recipes looked up, newest first (`LookupHistory`), saved when `lookups_dirty`.
    lookups: LookupHistory,
    lookups_dirty: bool,
    config_dirty: bool,
    bookmarks_dirty: bool,
    lookup_rects: Vec<(Rect, String)>,
    /// The search indexes as the completion reads them.
    views: Vec<PrefixView>,
}

/// The option rows of the options screen.
const MODE_ROWS: [(&str, SearchIndex); 5] = [
    ("Mod name search", SearchIndex::ModNames),
    ("Tag search", SearchIndex::Tags),
    ("Tooltip search", SearchIndex::Tooltips),
    ("Color search", SearchIndex::Colors),
    ("Creative tab search", SearchIndex::CreativeTabs),
];

fn parse_mode(v: &str, default: SearchMode) -> SearchMode {
    match v {
        "ENABLED" => SearchMode::Enabled,
        "REQUIRE_PREFIX" => SearchMode::RequireManualPrefix,
        "DISABLED" => SearchMode::Disabled,
        _ => default,
    }
}

fn mode_name(m: SearchMode) -> &'static str {
    match m {
        SearchMode::Enabled => "ENABLED",
        SearchMode::RequireManualPrefix => "REQUIRE_PREFIX",
        SearchMode::Disabled => "DISABLED",
    }
}

fn next_mode(m: SearchMode) -> SearchMode {
    match m {
        SearchMode::Enabled => SearchMode::RequireManualPrefix,
        SearchMode::RequireManualPrefix => SearchMode::Disabled,
        SearchMode::Disabled => SearchMode::Enabled,
    }
}

/// `IngredientVisibility`: hidden by the player, singly or with all variants of the item (`HideMode.WILDCARD`).
fn is_hidden(hidden: &[String], id: &str) -> bool {
    hidden.iter().any(|h| h == id || h.strip_prefix('*').is_some_and(|w| w == id))
}

fn lines(text: Option<String>) -> Vec<String> {
    text.unwrap_or_default().lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_owned).collect()
}

fn save_lines(host: &dyn ClientHost, key: &str, list: &[String]) {
    host.set_setting("jei", key, &list.iter().map(|l| format!("{l}\n")).collect::<String>());
}

/// The index over the elements, with the modes now set.
fn build_search(s: &mut State) {
    let cfg = FilterConfig::default();
    let modes = s.modes.clone();
    let mut search = ElementSearch::new(|i| modes.iter().find(|(k, _)| *k == i).map_or(SearchMode::Disabled, |(_, m)| *m));
    for (i, info) in s.infos.iter().enumerate() {
        search.add(i, |idx: SearchIndex| info.strings(idx, &cfg));
    }
    s.search = Some(search);
    s.views = PREFIXES
        .iter()
        .map(|p| {
            let mut strings: Vec<String> = s.infos.iter().flat_map(|i| i.strings(p.index, &cfg)).collect();
            strings.sort();
            strings.dedup();
            PrefixView {
                prefix: p.prefix,
                mode: modes.iter().find(|(k, _)| *k == p.index).map_or(SearchMode::Disabled, |(_, m)| *m),
                dynamic: p.supports_dynamic_completion,
                description: p.description_key.rsplit('.').next().unwrap_or("").to_owned(),
                strings,
            }
        })
        .collect();
}

/// The completion candidates for the search text (`SearchCompletion`), empty when the option is off.
fn suggestions(s: &State) -> Vec<Candidate> {
    if !s.focused || !s.config.bool("search", "searchCompletionEnabled") {
        return Vec::new();
    }
    let rows = s.config.int("search", "maxSearchCompletionRows") as usize;
    let cursor = s.query.encode_utf16().count();
    completion::candidates(&s.query, cursor, &s.views, rows).candidates.into_iter().take(rows).collect()
}

/// Replaces the token under the cursor with a candidate (Tab); a leading `-` stays.
fn accept(s: &mut State, c: &Candidate) {
    let start = s.query.rfind(char::is_whitespace).map_or(0, |i| i + 1);
    let token = &s.query[start..];
    let keep = if token.starts_with('-') && c.insertion != "-" { start + 1 } else { start };
    s.query.truncate(keep);
    s.query.push_str(&c.insertion);
    s.query.truncate(50);
    refilter(s);
}

fn options_rows(s: &State, bw: i32, bh: i32) -> (Rect, Vec<Rect>) {
    let w = 200;
    let n = MODE_ROWS.len() as i32 + 4;
    let h = 14 + n * 20;
    let panel = Rect::new((bw - w) / 2, (bh - h) / 2, w, h);
    let _ = s;
    (panel, (0..n).map(|i| Rect::new(panel.x + 6, panel.y + 8 + i * 20, w - 12, 18)).collect())
}

fn draw_options(s: &State, draw: &mut dyn DrawContext) {
    let [bw, bh] = draw.size();
    let (panel, rows) = options_rows(s, bw as i32, bh as i32);
    draw.fill(rect_f(panel), BG);
    draw.fill(rect_f(Rect::new(panel.x + 1, panel.y + 1, panel.w - 2, panel.h - 2)), 0xFFC6C6C6);
    let n = MODE_ROWS.len();
    for (i, r) in rows.iter().enumerate() {
        draw.fill(rect_f(*r), SLOT_BG);
        let label = if i < n {
            let mode = s.modes.iter().find(|(k, _)| *k == MODE_ROWS[i].1).map_or(SearchMode::Disabled, |(_, m)| *m);
            format!("{}: {}", MODE_ROWS[i].0, mode_name(mode))
        } else if i == n {
            format!("Max columns: {} (click +1, right -1)", s.columns)
        } else if i == n + 1 {
            format!("Max rows: {} (click +1, right -1)", s.rows)
        } else if i == n + 3 {
            format!("Search completion: {} (Tab accepts)", if s.config.bool("search", "searchCompletionEnabled") { "ON" } else { "OFF" })
        } else {
            format!("Hide Ingredients Mode: {} (Ctrl+click hides)", if s.edit { "ON" } else { "OFF" })
        };
        draw.text(&label, [(r.x + 4) as f32, (r.y + 5) as f32], TEXT, true);
    }
}

fn options_click(host: &dyn ClientHost, s: &mut State, x: i32, y: i32, right: bool) {
    let [bw, bh] = s.screen_size;
    let (panel, rows) = options_rows(s, bw, bh);
    if !panel.contains(x, y) {
        s.options = false;
        return;
    }
    let n = MODE_ROWS.len();
    for (i, r) in rows.iter().enumerate() {
        if !r.contains(x, y) {
            continue;
        }
        if i < n {
            let idx = MODE_ROWS[i].1;
            if let Some(entry) = s.modes.iter_mut().find(|(k, _)| *k == idx) {
                entry.1 = next_mode(entry.1);
                let id = PREFIXES.iter().find(|p| p.index == idx).map_or("", |p| p.id);
                host.set_setting("jei", &format!("search_{id}"), mode_name(entry.1));
            }
            build_search(s);
            refilter(s);
        } else if i == n {
            s.columns = (s.columns + if right { -1 } else { 1 }).clamp(2, 100);
            host.set_setting("jei", "max_columns", &s.columns.to_string());
        } else if i == n + 1 {
            s.rows = (s.rows + if right { -1 } else { 1 }).clamp(1, 100);
            host.set_setting("jei", "max_rows", &s.rows.to_string());
        } else if i == n + 3 {
            let on = !s.config.bool("search", "searchCompletionEnabled");
            s.config.set("search", "searchCompletionEnabled", if on { "true" } else { "false" });
            host.set_setting("jei", "config", &s.config.serialize());
        } else {
            s.edit = !s.edit;
            refilter(s);
        }
    }
}

/// An open recipe view: the categories with recipes for the focus, the one shown, and its recipes (`RecipesGui`).
#[derive(Clone)]
struct View {
    cats: Vec<usize>,
    cat: usize,
    page: usize,
    /// The first recipe shown in the scrolled navigation mode.
    first: usize,
    recipes: Vec<RecipeId>,
    focus: Focus,
}

const BG: u32 = 0xFF000000;
const SLOT_BG: u32 = 0xFF8B8B8B;
const CYCLE_MS: u128 = 1000;

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// The screens the list shows over: the inventory screen draws every container, and the creative one.
fn shows_over(screen: &ScreenView<'_>) -> bool {
    // `overlaysEnabled`: off hides the whole list.
    matches!(screen.id, "inventory" | "creative") && STATE.with(|c| c.try_borrow().is_ok_and(|s| s.config.bool("ingredientList", "overlaysEnabled")))
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

fn load(host: &dyn ClientHost, s: &mut State) {
    let Some(entries) = host.registry_entries("minecraft:item") else { return };
    let cfg = FilterConfig::default();
    let mut names: HashMap<ItemKey, String> = HashMap::new();
    // The creative inventory's own tooltip lines and tab titles, for search (`$tooltip`, `%tab`) and the hover.
    let mut extra: HashMap<ItemKey, (Vec<String>, Vec<String>)> = HashMap::new();
    for c in host.creative_inventory_stacks() {
        let e = extra.entry(c.stack.item()).or_default();
        if e.0.is_empty() {
            e.0 = c.tooltip.clone();
        }
        for tab in c.tabs {
            if !e.1.contains(&tab) {
                e.1.push(tab);
            }
        }
    }
    for id in entries {
        let Some(key) = host.item_key(&id) else { continue };
        let id = id.to_string();
        let name = ItemStack::new(key, 1).map(|st| host.hover_name(&st)).unwrap_or_else(|| id.clone());
        let (ns, path) = id.split_once(':').unwrap_or(("minecraft", &id));
        let data = IngredientData {
            display_name: name,
            namespace: ns.to_owned(),
            path: path.to_owned(),
            display_mod_id: ns.to_owned(),
            mod_name: mod_name(ns),
            display_mod_name: mod_name(ns),
            mod_aliases: vec![vec![]],
            tooltip: extra.get(&key).map(|e| e.0.clone()).unwrap_or_default(),
            creative_tabs: extra.get(&key).map(|e| e.1.clone()).unwrap_or_default(),
            ..Default::default()
        };
        let i = s.infos.len();
        s.infos.push(ListElementInfo::new(data, &cfg, i));
        s.keys.push(key);
        s.index.insert(id.clone(), s.keys.len() - 1);
        names.insert(key, id.clone());
        s.ids.push(id);
    }
    s.recipes = Some(recipes::load(host, &names));
    s.plugin_version = api::version();
    s.names = names;
    build_search(s);
    s.loaded = true;
    refilter(s);
}

fn refilter(s: &mut State) {
    if let Some(search) = &s.search {
        s.results = search.query(&s.query);
    }
    if !s.edit {
        let (hidden, ids) = (&s.hidden, &s.ids);
        s.results.retain(|&i| ids.get(i).is_none_or(|id| !is_hidden(hidden, id)));
    }
    s.page = 0;
}

fn open_recipes(s: &mut State, item: &str, role: Role) {
    let Some(r) = s.recipes.as_ref() else { return };
    let focus = Focus { role, uid: item.to_owned() };
    s.lookups.add(item);
    s.lookups_dirty = true;
    s.tab_page = None;
    let cats: Vec<usize> = r
        .mgr
        .categories(std::slice::from_ref(&focus), false, &|_| true)
        .iter()
        .filter_map(|t| r.cats.iter().position(|c| &c.recipe_type == t))
        .collect();
    if cats.is_empty() {
        return;
    }
    let recipes = r.mgr.recipes(&r.cats[cats[0]].recipe_type, std::slice::from_ref(&focus), false, &|_| true);
    if let Some(old) = s.view.take() {
        s.history.push(old);
    }
    s.view = Some(View { cats, cat: 0, page: 0, first: 0, recipes, focus });
}

fn select_category(s: &mut State, cat: usize) {
    let (Some(r), Some(v)) = (s.recipes.as_ref(), s.view.as_mut()) else { return };
    v.cat = cat;
    v.page = 0;
    v.first = 0;
    s.tab_page = None;
    v.recipes = r.mgr.recipes(&r.cats[v.cats[cat]].recipe_type, std::slice::from_ref(&v.focus), false, &|_| true);
}

fn cycle(s: &State, list: &[String]) -> Option<String> {
    if list.is_empty() {
        return None;
    }
    // `recipeSlotCyclingEnabled`: off keeps the first ingredient.
    if !s.config.bool("input", "recipeSlotCyclingEnabled") {
        return Some(list[0].clone());
    }
    let ms = s.started.map_or(0, |t| t.elapsed().as_millis());
    Some(list[(ms / CYCLE_MS) as usize % list.len()].clone())
}

/// What a press on the recipe screen's widgets does.
#[derive(Clone, Debug)]
enum Hit {
    PreviousCategory,
    NextCategory,
    PreviousPage,
    NextPage,
    Tab(usize),
    TabsPrevious,
    TabsNext,
    Transfer(String),
    /// The bookmark button of a recipe, with the ingredient it bookmarks.
    Bookmark(String),
    /// An option button of the left tab: 0 bookmarks first, 1 craftable first.
    SortOption(usize),
}

const NOTE_COLOR: u32 = 0xFF808080;

/// Where a category's text widget sits in its layout (`setPosition(x, 0)`): the fuel and composting categories.
fn note_x(recipe_type: &str) -> i32 {
    match recipe_type {
        "jei:smelting_fuel" => 20,
        "jei:compostable" => 24,
        _ => 0,
    }
}

/// Scrolls the recipe list by `dir` rows of `step` recipes, stopping at both ends.
fn scroll_recipes(v: &mut View, dir: i32, step: i32, per: i32) {
    let max = (v.recipes.len() as i32 - per).max(0);
    v.first = (v.first as i32 + dir * step).clamp(0, max) as usize;
}

/// `RecipesGui`: the panel over the screen (`gui_background`), the category tabs above it, the category and page
/// arrows with the title, and the recipes of the page in `RecipeGuiGrid` cells, each with its side buttons.
fn draw_view(s: &mut State, draw: &mut dyn DrawContext, screen: &ScreenView<'_>) {
    let (Some(v), Some(r)) = (s.view.clone(), s.recipes.as_ref()) else { return };
    let cat = r.cats[v.cats[v.cat]].clone();
    let [bw, bh] = draw.size();
    let (bw, bh) = (bw as i32, bh as i32);
    let ui_at = |i: usize| v.recipes.get(i).and_then(|id| r.ui.get(*id as usize)).cloned();
    let (cx, cy) = (screen.mouse[0] as i32, screen.mouse[1] as i32);

    // The stations and option tabs reach left of the panel and narrow what the screen may take (`getLeftSideExtraWidth`).
    let first_ideal = recipe_gui::ideal_area(bw, bh, recipe_gui::MIN_GUI_WIDTH, recipe_gui::DEFAULT_GUI_HEIGHT, 0);
    let (first_options, _) = recipe_gui::option_buttons(first_ideal, 2);
    let extra = recipe_gui::left_extra_width(recipe_gui::stations(cat.stations.len(), first_ideal, first_options).as_ref(), first_options);
    let ideal = recipe_gui::ideal_area(bw, bh, recipe_gui::MIN_GUI_WIDTH, recipe_gui::DEFAULT_GUI_HEIGHT, extra);
    let nav = recipe_gui::navigation(ideal);
    let first = ui_at(0);
    let layout_of = |u: Option<&recipes::UiRecipe>| cat.layout_for(u.map_or((3, 3), |u| u.grid), u.map_or(0, |u| u.inputs.len()));
    let lay0 = layout_of(first.as_ref());
    let border = recipe_gui::RECIPE_BORDER;
    let transfers = |u: &recipes::UiRecipe| !u.id.is_empty() && u.recipe_type == "minecraft:crafting";
    // Side buttons: the transfer plus (crafting only) and the bookmark (`RecipeBookmarkButton`).
    let buttons = 1 + i32::from(first.as_ref().is_some_and(transfers));
    let total_w = recipe_gui::total_width(lay0.width, lay0.height, border, buttons);
    let cell_size = (lay0.width + 2 * border, lay0.height + 2 * border);
    let max_columns = s.config.int("recipes", "maxRecipeGuiColumns") as i32;
    let area = recipe_gui::area_to_fit(ideal, bw, if first.is_some() { total_w } else { 0 });
    let avail = recipe_gui::layouts_area(area, nav.header_height);
    let grid = recipe_gui::RecipeGrid::calculate((avail.w, avail.h), (total_w, cell_size.1), max_columns);
    let per = grid.recipes_per_page();
    s.per_page = per;
    s.recipe_columns = grid.columns;
    let scrolled = s.config.text("recipes", "navigationMode") == "SCROLLED";
    let pages = grid::page_count(v.recipes.len() as i32, per).max(1);
    let page = (v.page as i32).min(pages - 1);
    // `RecipeNavigationMode.SCROLLED`: the list moves a row at a time instead of a page.
    let first_recipe = if scrolled { (v.first as i32).min((v.recipes.len() as i32 - per).max(0)) } else { page * per };
    s.panel = area;
    s.hits.clear();

    ui::gui_background(draw, area);
    // `RecipeOptionButtons` and `CraftingStations`: the tabs left of the panel.
    let (options_tab, option_rects) = recipe_gui::option_buttons(area, 2);
    ui::side_tab(draw, options_tab, "recipe_options_tab_v2");
    for (i, rect) in option_rects.iter().enumerate() {
        let (icon, key) = if i == 0 { ("icons/bookmarks_first", "recipeSortingBookmarks") } else { ("icons/craftable_first", "recipeSortingCraftable") };
        ui::icon_button(draw, *rect, icon, s.config.bool("recipes", key), rect.contains(cx, cy));
        s.hits.push((*rect, Hit::SortOption(i)));
    }
    let mut station_hot = Vec::new();
    if let Some(st) = recipe_gui::stations(cat.stations.len(), area, options_tab) {
        ui::side_tab(draw, st.tab, "catalyst_tab_v2");
        ui::side_tab(draw, st.inner, "recipe_catalyst_slot_background_v2");
        for (rect, id) in st.slots.iter().zip(cat.stations.iter()) {
            if let Some(&k) = s.index.get(id) {
                draw.item(s.keys[k], [rect.x as f32, rect.y as f32], 16.0);
            }
            if rect.contains(cx, cy) {
                draw.fill(rect_f(*rect), HOVER);
            }
            station_hot.push((*rect, id.clone()));
        }
    }
    // The tabs, over the panel's top edge.
    let selected = v.cat;
    if let Some(mut t) = recipe_gui::tabs(ideal, v.cats.len(), selected) {
        if let Some(p) = s.tab_page.filter(|p| *p < t.page_count)
            && let Some(moved) = recipe_gui::tabs(ideal, v.cats.len(), p * t.per_page)
        {
            t = moved;
        }
        for (i, rect) in t.rects.iter().enumerate() {
            let index = t.page * t.per_page + i;
            ui::tab(draw, *rect, index == selected);
            if let Some(&k) = s.index.get(&r.cats[v.cats[index]].icon) {
                draw.item(s.keys[k], [(rect.x + 4) as f32, (rect.y + 5) as f32], 16.0);
            }
            s.hits.push((*rect, Hit::Tab(index)));
        }
        if t.page_count > 1 {
            let n = t.navigation;
            let (prev, next) = overlay_layout::nav_buttons(n);
            for (rect, left) in [(prev, true), (next, false)] {
                ui::button(draw, rect, rect.contains(cx, cy));
                ui::arrow(draw, rect, left);
            }
            let label = format!("{}/{}", t.page + 1, t.page_count);
            let lw = draw.text_width(&label) as i32;
            draw.text(&label, [(n.x + (n.w - lw) / 2) as f32, (n.y + (n.h - 9) / 2) as f32], TEXT, true);
            s.hits.push((prev, Hit::TabsPrevious));
            s.hits.push((next, Hit::TabsNext));
            s.tab_page = Some(t.page);
        }
    }

    // The arrows and the title between them.
    let between = |a: Rect, b: Rect| Rect::new(a.x + a.w, a.y, b.x - (a.x + a.w), a.h);
    let bar = between(nav.previous_category, nav.next_category);
    draw.fill(rect_f(bar), PAGE_NAV_BG);
    let mut title = cat.title.clone();
    while draw.text_width(&title) as i32 > nav.title.w && title.pop().is_some() {}
    let tw = draw.text_width(&title) as i32;
    draw.text(&title, [(nav.title.x + (nav.title.w - tw) / 2) as f32, (nav.title.y + (nav.title.h - 9) / 2) as f32], TEXT, true);
    let mut arrows = vec![(nav.previous_category, true, Hit::PreviousCategory), (nav.next_category, false, Hit::NextCategory)];
    draw.fill(rect_f(between(nav.previous_page, nav.next_page)), PAGE_NAV_BG);
    let label = recipe_gui::page_string(first_recipe, per, v.recipes.len() as i32);
    let lw = draw.text_width(&label) as i32;
    draw.text(&label, [(nav.page_area.x + (nav.page_area.w - lw) / 2) as f32, (nav.page_area.y + (nav.page_area.h - 9) / 2) as f32], TEXT, true);
    arrows.push((nav.previous_page, true, Hit::PreviousPage));
    arrows.push((nav.next_page, false, Hit::NextPage));
    for (rect, left, hit) in arrows {
        ui::button(draw, rect, rect.contains(cx, cy));
        ui::arrow(draw, rect, left);
        s.hits.push((rect, hit));
    }

    // The recipes of the page.
    let mut hot = Vec::new();
    for slot_index in 0..per {
        let idx = (first_recipe + slot_index) as usize;
        let Some(u) = ui_at(idx) else { break };
        let lay = layout_of(Some(&u));
        let cell = grid.recipe_area(slot_index, avail, cell_size, total_w);
        let origin = (cell.x + border, cell.y + border);
        ui::recipe_background(draw, Rect::new(cell.x, cell.y, lay.width + 2 * border, lay.height + 2 * border));
        if let Some((ax, ay)) = lay.arrow {
            ui::recipe_arrow(draw, origin.0 + ax, origin.1 + ay);
        }
        for slot in &lay.slots {
            let rect = Rect::new(origin.0 + slot.x, origin.1 + slot.y, 18, 18);
            ui::slot(draw, rect);
            let list: Vec<String> = match slot.role {
                SlotRole::Input => u.inputs.get(slot.input).cloned().unwrap_or_default(),
                SlotRole::Output => u.outputs.clone(),
                SlotRole::Fuel => vec!["minecraft:coal".to_owned()],
            };
            if let Some(id) = cycle(s, &list) {
                if let Some(&k) = s.index.get(&id) {
                    draw.item(s.keys[k], [(rect.x + 1) as f32, (rect.y + 1) as f32], 16.0);
                }
                if rect.contains(cx, cy) {
                    draw.fill([(rect.x + 1) as f32, (rect.y + 1) as f32, 16.0, 16.0], HOVER);
                }
                hot.push((rect, id));
            }
        }
        if !u.note.is_empty() {
            // The text widget: centred in the box from `x` to the layout's right edge, over its whole height.
            let x = note_x(&u.recipe_type);
            let tw = draw.text_width(&u.note) as i32;
            draw.text(&u.note, [(origin.0 + x + (lay.width - x - tw) / 2) as f32, (origin.1 + (lay.height - 9) / 2) as f32], NOTE_COLOR, false);
        }
        if transfers(&u) {
            // Recipe transfer: the first side button, right of the layout (`getSideButtonArea(0)`).
            let b = recipe_gui::side_button(lay.width, lay.height, border, 0);
            let rect = Rect::new(origin.0 + b.x, origin.1 + b.y, b.w, b.h);
            ui::button(draw, rect, rect.contains(cx, cy));
            ui::plus(draw, rect);
            s.hits.push((rect, Hit::Transfer(u.id.clone())));
        }
        // The bookmark button bookmarks the recipe's output (`bookmarkOutputAsRecipe` is not stored per recipe yet).
        if let Some(out) = u.outputs.first().or_else(|| u.inputs.first().and_then(|i| i.first())) {
            let b = recipe_gui::side_button(lay.width, lay.height, border, i32::from(transfers(&u)));
            let rect = Rect::new(origin.0 + b.x, origin.1 + b.y, b.w, b.h);
            ui::bookmark(draw, rect, s.bookmarks.contains(out), rect.contains(cx, cy));
            s.hits.push((rect, Hit::Bookmark(out.clone())));
        }
    }
    hot.extend(station_hot);
    s.hot = hot;
    // `LookupHistory`: the recipes looked up lately, under the panel; a click opens that item's recipes.
    s.lookup_rects.clear();
    // Shown when `lookups.enabled`, in at most `maxRows` rows of `maxColumns` (and what fits under the panel).
    let per_row = (((area.w - 4) / 18).max(1) as usize).min(s.config.int("lookups", "maxColumns") as usize);
    let rows = if s.config.bool("lookups", "enabled") { s.config.int("lookups", "maxRows") as usize } else { 0 };
    for (i, id) in s.lookups.items().iter().take(per_row * rows).enumerate() {
        let Some(&k) = s.index.get(id) else { continue };
        let rect = Rect::new(area.x + 2 + (i % per_row) as i32 * 18, area.y + area.h + 4 + (i / per_row) as i32 * 18, 18, 18);
        ui::slot(draw, rect);
        draw.item(s.keys[k], [(rect.x + 1) as f32, (rect.y + 1) as f32], 16.0);
        s.lookup_rects.push((rect, id.clone()));
    }
}

/// A press while the view is open; true when the view took it (always: it is modal).
fn view_click(s: &mut State, x: i32, y: i32, right: bool) -> bool {
    if let Some((_, id)) = s.lookup_rects.iter().find(|(r, _)| r.contains(x, y)).cloned() {
        open_recipes(s, &id, Role::Output);
        return true;
    }
    if let Some((_, hit)) = s.hits.iter().find(|(r, _)| r.contains(x, y)).cloned() {
        let (cats, cat, recipes) = s.view.as_ref().map_or((0, 0, 0), |v| (v.cats.len(), v.cat, v.recipes.len()));
        let per = s.per_page.max(1);
        let pages = grid::page_count(recipes as i32, per).max(1) as usize;
        match hit {
            Hit::Transfer(id) => {
                // Left places the recipe once, right as many as the items allow (JEI: Shift).
                s.pending_transfer = Some((id, right));
                s.view = None;
                s.history.clear();
            }
            Hit::Bookmark(id) => {
                s.bookmarks.toggle(&id);
                s.bookmarks_dirty = true;
            }
            Hit::Tab(i) => select_category(s, i),
            Hit::SortOption(i) => {
                let key = if i == 0 { "recipeSortingBookmarks" } else { "recipeSortingCraftable" };
                let on = !s.config.bool("recipes", key);
                s.config.set("recipes", key, if on { "true" } else { "false" });
                s.config_dirty = true;
            }
            Hit::PreviousCategory | Hit::NextCategory if cats > 0 => {
                let next = if matches!(hit, Hit::NextCategory) { (cat + 1) % cats } else { (cat + cats - 1) % cats };
                select_category(s, next);
            }
            Hit::PreviousPage | Hit::NextPage => {
                let (scrolled, step) = (s.config.text("recipes", "navigationMode") == "SCROLLED", s.recipe_columns.max(1));
                if let Some(v) = s.view.as_mut() {
                    if scrolled {
                        scroll_recipes(v, if matches!(hit, Hit::NextPage) { 1 } else { -1 }, step, s.per_page);
                    } else {
                        v.page = if matches!(hit, Hit::NextPage) { (v.page + 1) % pages } else { (v.page + pages - 1) % pages };
                    }
                }
            }
            Hit::TabsPrevious | Hit::TabsNext => {
                let per_tabs = ((recipe_gui::ideal_area(s.screen_size[0], s.screen_size[1], recipe_gui::MIN_GUI_WIDTH, recipe_gui::DEFAULT_GUI_HEIGHT, 0).w - 4) / recipe_gui::TAB_WIDTH).max(1) as usize;
                let tab_pages = cats.div_ceil(per_tabs).max(1);
                let page = s.tab_page.unwrap_or(0);
                s.tab_page = Some(if matches!(hit, Hit::TabsNext) { (page + 1) % tab_pages } else { (page + tab_pages - 1) % tab_pages });
            }
            _ => {}
        }
        return true;
    }
    if !s.panel.contains(x, y) {
        s.view = None;
        s.history.clear();
        return true;
    }
    if let Some((_, id)) = s.hot.iter().find(|(r, _)| r.contains(x, y)).cloned() {
        // A click on an ingredient: left shows how it is made, right what it is used in (`jei.key.showRecipe/showUses`).
        open_recipes(s, &id, if right { Role::Input } else { Role::Output });
    }
    true
}

fn rect_f(r: Rect) -> [f32; 4] {
    [r.x as f32, r.y as f32, r.w as f32, r.h as f32]
}

#[derive(Default)]
struct Mod;

/// The action of the key just pressed, from the keys the player has bound (`host.mod_key`).
fn key_action(host: &dyn ClientHost, key: &str, modifiers: u32, in_recipes: bool) -> Option<keys::Action> {
    keys::MAPPINGS.iter().filter(|m| m.in_recipes == in_recipes || !m.in_recipes).find_map(|m| {
        let (bound, mods) = host.mod_key("jei", &keys::registered_id(m))?;
        let mods = mods.bits & (keys::SHIFT | keys::CONTROL | keys::ALT);
        (bound == key && mods == modifiers & (keys::SHIFT | keys::CONTROL | keys::ALT)).then_some(m.action)
    })
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn register(&self, r: &mut dyn mod_api::Registrar, _: &mod_api::RegistrationContext) {
        // `InternalKeyMappings`: each mapping on Options > Controls under its Java label.
        for m in keys::MAPPINGS {
            let mut reg = mod_api::KeyRegistration::new(&keys::registered_id(m), m.key).label(m.id);
            reg.modifiers = mod_api::ClientModifiers::new(m.modifiers);
            r.register_key_mapping(reg);
        }
    }

    fn screen_render_post(&self, host: &dyn ClientHost, event: &mut ScreenRenderPostEvent<'_>) {
        if !shows_over(event.screen) {
            return;
        }
        let screen = event.screen;
        let draw: &mut dyn DrawContext = &mut *event.draw;
        STATE.with_borrow_mut(|s| {
            if !s.settings_loaded {
                s.cheat = host.setting("jei", "cheat_mode").is_some_and(|v| v == "true");
                s.bookmarks = BookmarkList::load(&host.setting("jei", "bookmarks").unwrap_or_default());
                s.hidden = lines(host.setting("jei", "hidden"));
                s.modes = PREFIXES
                    .iter()
                    .map(|p| (p.index, host.setting("jei", &format!("search_{}", p.id)).map_or(default_mode(p.index), |v| parse_mode(&v, default_mode(p.index)))))
                    .collect();
                s.columns = host.setting("jei", "max_columns").and_then(|v| v.parse().ok()).unwrap_or(9).clamp(2, 100);
                s.rows = host.setting("jei", "max_rows").and_then(|v| v.parse().ok()).unwrap_or(16).clamp(1, 100);
                s.config = Config::parse(&host.setting("jei", "config").unwrap_or_default());
                s.lookups = LookupHistory::load(&host.setting("jei", "lookups").unwrap_or_default(), s.config.int("lookups", "maxIngredients") as usize);
                s.settings_loaded = true;
            }
            if !s.loaded {
                load(host, s);
            }
            if !s.settings_loaded {
                s.cheat = host.setting("jei", "cheat_mode").is_some_and(|v| v == "true");
                s.settings_loaded = true;
            }
            if !s.loaded {
                return;
            }
            if s.plugin_version != api::version() {
                s.recipes = Some(recipes::load(host, &s.names));
                s.plugin_version = api::version();
                s.view = None;
                s.history.clear();
            }
            let [bw, bh] = draw.size();
            let (bw, bh) = (bw as i32, bh as i32);
            s.screen_size = [bw, bh];
            let gui_right = (screen.rect[0] + screen.rect[2]) as i32;
            let area = overlay_layout::available_grid_area(gui_right, bw, bh);
            let bounds = grid::calculate_bounds(&GridConfig { max_columns: s.columns, max_rows: s.rows, ..GridConfig::ingredient_list() }, area, false);
            s.grid = bounds;
            let areas = overlay_layout::areas(gui_right, bw, bh, bounds);
            s.nav = areas.navigation;
            s.search_box = areas.search;
            s.options_button = areas.config_button;
            if bounds.is_empty() {
                return;
            }
            ui::gui_background(draw, areas.background);
            let (cx, cy) = (screen.mouse[0] as i32, screen.mouse[1] as i32);
            let slots = grid::calculate_slots(bounds, &[], None, 0);
            let per_page = slots.len() as i32;
            let first = grid::first_item_index_for_valid_page(s.page * per_page, s.results.len() as i32, per_page);
            s.page = if per_page == 0 { 0 } else { first / per_page };
            let mut hot = Vec::new();
            for slot in &slots {
                ui::slot(draw, slot.area);
            }
            for (n, slot) in slots.iter().enumerate() {
                let Some(&idx) = s.results.get(first as usize + n) else { break };
                let a = slot.area;
                draw.item(s.keys[idx], [(a.x + 1) as f32, (a.y + 1) as f32], 16.0);
                hot.push((a, s.ids[idx].clone()));
                if s.edit && is_hidden(&s.hidden, &s.ids[idx]) {
                    draw.fill([(a.x + 1) as f32, (a.y + 1) as f32, 16.0, 16.0], 0x80FF0000);
                }
                if a.contains(cx, cy) {
                    draw.fill([(a.x + 1) as f32, (a.y + 1) as f32, 16.0, 16.0], HOVER);
                }
            }
            // The bookmark list, left of the window.
            let marks_area = overlay_layout::available_grid_area_in(Rect::new(0, 0, screen.rect[0] as i32, bh), false);
            let marks = grid::calculate_bounds(&GridConfig { max_columns: s.columns, max_rows: s.rows, ..GridConfig::bookmark_list() }, marks_area, false);
            s.marks = marks;
            if !marks.is_empty() {
                let slot_bg = marks.expand_by(overlay_layout::INNER_PADDING);
                let nav = Rect::new(slot_bg.x, slot_bg.y - (overlay_layout::NAVIGATION_HEIGHT + overlay_layout::INNER_PADDING), slot_bg.w, overlay_layout::NAVIGATION_HEIGHT);
                let panel = Rect::new(slot_bg.x.min(nav.x), nav.y, slot_bg.w, slot_bg.y + slot_bg.h - nav.y).expand_by(overlay_layout::BORDER_PADDING);
                ui::gui_background(draw, panel);
                for slot in grid::calculate_slots(marks, &[], None, 0) {
                    ui::slot(draw, slot.area);
                }
                for (slot, id) in grid::calculate_slots(marks, &[], None, 0).iter().zip(s.bookmarks.items().to_vec()) {
                    let a = slot.area;
                    if let Some(&k) = s.index.get(&id) {
                        draw.item(s.keys[k], [(a.x + 1) as f32, (a.y + 1) as f32], 16.0);
                    }
                    if a.contains(cx, cy) {
                        draw.fill([(a.x + 1) as f32, (a.y + 1) as f32, 16.0, 16.0], HOVER);
                    }
                    hot.push((a, id));
                }
            }
            s.hot = hot;
            let pages = grid::page_count(s.results.len() as i32, per_page);
            let label = format!("{}/{}", s.page + 1, pages);
            let lw = draw.text_width(&label) as i32;
            let nav = s.nav;
            let (back, next) = overlay_layout::nav_buttons(nav);
            draw.fill(rect_f(Rect::new(back.x + back.w, nav.y, next.x - (back.x + back.w), nav.h)), PAGE_NAV_BG);
            for (b, left) in [(back, true), (next, false)] {
                ui::button(draw, b, b.contains(cx, cy));
                ui::arrow(draw, b, left);
            }
            draw.text(&label, [(nav.x + (nav.w - lw) / 2) as f32, (nav.y + (nav.h - 9) / 2) as f32], TEXT, true);
            let search = s.search_box;
            draw.fill(rect_f(search), 0xFF000000);
            draw.fill(rect_f(Rect::new(search.x + 1, search.y + 1, search.w - 2, search.h - 2)), if s.focused { 0xFF202020 } else { 0xFF101010 });
            let shown = if s.query.is_empty() && !s.focused { "Search...".to_owned() } else { s.query.clone() };
            if s.query.is_empty() {
                draw.text(&shown, [(search.x + 4) as f32, (search.y + 6) as f32], 0xFF909090, false);
            } else {
                // `SearchSyntaxHighlighter`: prefixes, operators and content in their colours; red when nothing matches.
                let prefixes: Vec<char> = PREFIXES.iter().filter(|p| p.prefix != '\0' && s.modes.iter().any(|(k, m)| *k == p.index && *m != SearchMode::Disabled)).map(|p| p.prefix).collect();
                let colors = highlight::colors(&s.query, s.results.is_empty(), &prefixes);
                let chars: Vec<char> = shown.chars().collect();
                let color_at = |u: usize| colors.get(u).copied().unwrap_or(highlight::DEFAULT_CONTENT_COLOR);
                let (mut x, mut i, mut unit) = ((search.x + 4) as f32, 0, 0);
                while i < chars.len() {
                    let color = color_at(unit);
                    let (mut j, mut u) = (i, unit);
                    while j < chars.len() && color_at(u) == color {
                        u += chars[j].len_utf16();
                        j += 1;
                    }
                    let seg: String = chars[i..j].iter().collect();
                    draw.text(&seg, [x, (search.y + 6) as f32], color, false);
                    x += draw.text_width(&seg);
                    i = j;
                    unit = u;
                }
            }
            let cands = suggestions(s);
            for (i, c) in cands.iter().enumerate() {
                let r = Rect::new(search.x, search.y - (cands.len() - i) as i32 * 11, search.w.max(120), 11);
                draw.fill(rect_f(r), 0xF0101010);
                draw.text(&c.display, [(r.x + 3) as f32, (r.y + 2) as f32], if i == 0 { 0xFFFFFF55 } else { TEXT }, false);
                if !c.description.is_empty() {
                    draw.text(&c.description, [(r.x + 3 + draw.text_width(&c.display) as i32 + 8) as f32, (r.y + 2) as f32], 0xFF909090, false);
                }
            }
            s.started.get_or_insert_with(Instant::now);
            // The config button (`config_button.png`; the cheat icon, red, while cheating).
            let cfg = s.options_button;
            ui::button(draw, cfg, cfg.contains(cx, cy));
            ui::cog(draw, cfg, s.cheat);
            if s.options {
                draw_options(s, draw);
            }
            draw_view(s, draw, screen);
            // The tooltip of the ingredient under the cursor: its name and the mod it is from.
            if !s.options
                && let Some((_, id)) = s.hot.iter().find(|(r, _)| r.contains(cx, cy))
                && let Some(&k) = s.index.get(id)
            {
                let name = ItemStack::new(s.keys[k], 1).map_or_else(|| id.clone(), |st| host.hover_name(&st));
                let ns = id.split_once(':').map_or("minecraft", |p| p.0);
                let mut lines = vec![Text::literal(name)];
                lines.extend(s.infos[k].data.tooltip.iter().map(|l| Text::literal(l.clone())));
                lines.push(Text::literal(mod_name(ns)));
                let _ = draw.gui_tooltip(&lines, [cx as f32, cy as f32]);
            }
        });
    }

    fn screen_mouse_button_pressed_pre(&self, host: &dyn ClientHost, event: &mut ScreenMouseButtonPressedPreEvent<'_>) -> Flow {
        if !shows_over(event.screen) {
            return Flow::Continue;
        }
        let (x, y) = (event.screen.mouse[0] as i32, event.screen.mouse[1] as i32);
        let right = event.button == 1;
        let mut claimed = false;
        STATE.with_borrow_mut(|s| {
            if !s.loaded {
                return;
            }
            if s.options {
                options_click(host, s, x, y, right);
                claimed = true;
                return;
            }
            if s.options_button.contains(x, y) {
                if event.modifiers & 1 != 0 {
                    // Shift on the config button toggles cheat mode (`toggleCheatModeConfigButton`).
                    s.cheat = !s.cheat;
                    host.set_setting("jei", "cheat_mode", if s.cheat { "true" } else { "false" });
                } else {
                    s.options = true;
                }
                claimed = true;
                return;
            }
            if s.view.is_some() {
                claimed = view_click(s, x, y, right);
                return;
            }
            if let Some((_, id)) = s.hot.iter().find(|(r, _)| r.contains(x, y)).cloned() {
                if s.edit && event.modifiers & 2 != 0 && s.grid.contains(x, y) {
                    // `key.jei.toggleHideIngredient` (Ctrl+left) and `toggleWildcardHideIngredient` (Ctrl+right).
                    let entry = if right { format!("*{id}") } else { id };
                    match s.hidden.iter().position(|h| *h == entry) {
                        Some(at) => {
                            s.hidden.remove(at);
                        }
                        None => s.hidden.push(entry),
                    }
                    save_lines(host, "hidden", &s.hidden);
                    refilter(s);
                    claimed = true;
                    return;
                }
                if s.marks.contains(x, y) {
                    open_recipes(s, &id, if right { Role::Input } else { Role::Output });
                    claimed = true;
                    return;
                }
            }
            if s.search_box.contains(x, y) {
                s.focused = true;
                if right {
                    s.query.clear();
                    refilter(s);
                }
                claimed = true;
                return;
            }
            s.focused = false;
            let per = grid::calculate_slots(s.grid, &[], None, 0).len() as i32;
            if s.nav.contains(x, y) {
                let pages = grid::page_count(s.results.len() as i32, per);
                s.page = if x < s.nav.x + s.nav.w / 2 { (s.page - 1).rem_euclid(pages) } else { (s.page + 1) % pages };
                claimed = true;
                return;
            }
            let first = grid::first_item_index_for_valid_page(s.page * per, s.results.len() as i32, per);
            for (n, slot) in grid::calculate_slots(s.grid, &[], None, 0).iter().enumerate() {
                if slot.area.contains(x, y)
                    && let Some(&idx) = s.results.get(first as usize + n)
                {
                    if s.cheat {
                        // Cheat mode: left gives a stack, right one (needs creative mode).
                        s.give = Some((s.keys[idx], if right { 1 } else { 64 }));
                    } else {
                        // Left: how it is made; right: what it is used in.
                        let id = s.ids[idx].clone();
                        open_recipes(s, &id, if right { Role::Input } else { Role::Output });
                    }
                    claimed = true;
                }
            }
        });
        if claimed { Flow::Cancel } else { Flow::Continue }
    }

    fn client_tick_post(&self, host: &mut dyn ClientHost, _: &mut ClientTickPostEvent) {
        let (give, transfer) = STATE.with_borrow_mut(|s| (s.give.take(), s.pending_transfer.take()));
        STATE.with_borrow_mut(|s| {
            if std::mem::take(&mut s.bookmarks_dirty) {
                host.set_setting("jei", "bookmarks", &s.bookmarks.save());
            }
            if std::mem::take(&mut s.config_dirty) {
                host.set_setting("jei", "config", &s.config.serialize());
            }
            if std::mem::take(&mut s.lookups_dirty) {
                host.set_setting("jei", "lookups", &s.lookups.save());
            }
        });
        if let Some((item, count)) = give {
            let _ = host.give(item, count);
        }
        if let Some((id, all)) = transfer {
            host.transfer_recipe_id(&id, all);
        }
    }

    fn screen_mouse_scrolled_pre(&self, _: &dyn ClientHost, event: &mut ScreenMouseScrolledPreEvent<'_>) -> Flow {
        if !shows_over(event.screen) {
            return Flow::Continue;
        }
        let (x, y) = (event.screen.mouse[0] as i32, event.screen.mouse[1] as i32);
        STATE.with_borrow_mut(|s| {
            if s.view.is_some() {
                // `RecipeNavigationInputHandler`: the wheel over the recipe screen turns its pages.
                if s.panel.contains(x, y) {
                    let pages = grid::page_count(s.view.as_ref().map_or(0, |v| v.recipes.len()) as i32, s.per_page.max(1)).max(1) as usize;
                    let (scrolled, step) = (s.config.text("recipes", "navigationMode") == "SCROLLED", s.recipe_columns.max(1));
                    let per = s.per_page;
                    if let Some(v) = s.view.as_mut() {
                        if scrolled {
                            scroll_recipes(v, if event.scroll[1] > 0.0 { -1 } else { 1 }, step, per);
                        } else {
                            v.page = if event.scroll[1] > 0.0 { (v.page + pages - 1) % pages } else { (v.page + 1) % pages };
                        }
                    }
                    return Flow::Cancel;
                }
                return Flow::Continue;
            }
            if !s.loaded || !(s.grid.contains(x, y) || s.nav.contains(x, y)) {
                return Flow::Continue;
            }
            let per = grid::calculate_slots(s.grid, &[], None, 0).len() as i32;
            let pages = grid::page_count(s.results.len() as i32, per);
            s.page = (s.page + if event.scroll[1] > 0.0 { -1 } else { 1 }).rem_euclid(pages);
            Flow::Cancel
        })
    }

    fn screen_key_pressed_pre(&self, host: &dyn ClientHost, event: &mut ScreenKeyPressedPreEvent<'_>) -> Flow {
        if !shows_over(event.screen) {
            return Flow::Continue;
        }
        STATE.with_borrow_mut(|s| {
            if !s.loaded {
                return Flow::Continue;
            }
            let action = key_action(host, event.key, event.modifiers, s.view.is_some());
            if !s.focused || action == Some(keys::Action::FocusSearch) {
                use keys::Action;
                match action {
                    Some(Action::FocusSearch) => {
                        s.focused = true;
                        return Flow::Cancel;
                    }
                    Some(Action::ToggleOverlay) => {
                        let on = !s.config.bool("ingredientList", "overlaysEnabled");
                        s.config.set("ingredientList", "overlaysEnabled", if on { "true" } else { "false" });
                        s.config_dirty = true;
                        return Flow::Cancel;
                    }
                    Some(Action::ToggleCheatMode) => {
                        s.cheat = !s.cheat;
                        return Flow::Cancel;
                    }
                    Some(Action::PreviousPage | Action::NextPage) if s.view.is_none() => {
                        let per = (grid::calculate_slots(s.grid, &[], None, 0).len() as i32).max(1);
                        let pages = grid::page_count(s.results.len() as i32, per).max(1);
                        s.page = (s.page + if action == Some(Action::NextPage) { 1 } else { pages - 1 }).rem_euclid(pages);
                        return Flow::Cancel;
                    }
                    Some(Action::RecipeBack) if s.view.is_some() => {
                        // `HistoryList`: goes back to the earlier recipe view, then closes.
                        s.view = s.history.pop();
                        return Flow::Cancel;
                    }
                    Some(Action::CloseRecipeGui) if s.view.is_some() => {
                        s.view = None;
                        s.history.clear();
                        return Flow::Cancel;
                    }
                    Some(a @ (Action::PreviousRecipePage | Action::NextRecipePage)) if s.view.is_some() => {
                        let (scrolled, step) = (s.config.text("recipes", "navigationMode") == "SCROLLED", s.recipe_columns.max(1));
                        let per = s.per_page.max(1);
                        let pages = grid::page_count(s.view.as_ref().map_or(0, |v| v.recipes.len()) as i32, per).max(1) as usize;
                        let next = a == Action::NextRecipePage;
                        if let Some(v) = s.view.as_mut() {
                            if scrolled {
                                scroll_recipes(v, if next { 1 } else { -1 }, step, per);
                            } else {
                                v.page = if next { (v.page + 1) % pages } else { (v.page + pages - 1) % pages };
                            }
                        }
                        return Flow::Cancel;
                    }
                    Some(a @ (Action::PreviousCategory | Action::NextCategory)) if s.view.is_some() => {
                        let cats = s.view.as_ref().map_or(0, |v| v.cats.len());
                        if cats > 0 {
                            let cat = s.view.as_ref().map_or(0, |v| v.cat);
                            select_category(s, if a == Action::NextCategory { (cat + 1) % cats } else { (cat + cats - 1) % cats });
                        }
                        return Flow::Cancel;
                    }
                    _ => {}
                }
                match action {
                    Some(Action::Bookmark) => {
                        // `jei.key.bookmark` (A): bookmark the ingredient under the cursor, or remove it.
                        let (x, y) = (event.screen.mouse[0] as i32, event.screen.mouse[1] as i32);
                        if let Some((_, id)) = s.hot.iter().find(|(r, _)| r.contains(x, y)).cloned() {
                            s.bookmarks.toggle(&id);
                            host.set_setting("jei", "bookmarks", &s.bookmarks.save());
                            return Flow::Cancel;
                        }
                    }
                    Some(Action::ShowRecipe | Action::ShowUses) => {
                        // `jei.key.showRecipe` (R) and `jei.key.showUses` (U) on the stack under the cursor.
                        let (x, y) = (event.screen.mouse[0] as i32, event.screen.mouse[1] as i32);
                        let over = s.hot.iter().find(|(r, _)| r.contains(x, y)).map(|(_, id)| id.clone()).or_else(|| {
                            host.screen_hovered_stack().map(|st| st.item()).and_then(|k| s.keys.iter().position(|x| *x == k)).map(|i| s.ids[i].clone())
                        });
                        if let Some(id) = over {
                            open_recipes(s, &id, if action == Some(Action::ShowRecipe) { Role::Output } else { Role::Input });
                            return if s.view.is_some() { Flow::Cancel } else { Flow::Continue };
                        }
                    }
                    _ => {}
                }
                return Flow::Continue;
            }
            // Typing in the box must not reach the screen (`e` closes the inventory).
            match event.key {
                "key.keyboard.backspace" => {
                    s.query.pop();
                    refilter(s);
                }
                "key.keyboard.tab" => {
                    if let Some(c) = suggestions(s).first().cloned() {
                        accept(s, &c);
                    }
                }
                "key.keyboard.enter" | "key.keyboard.keypad.enter" | "key.keyboard.escape" => s.focused = false,
                _ => {}
            }
            Flow::Cancel
        })
    }

    fn screen_char_typed_pre(&self, _: &dyn ClientHost, event: &mut ScreenCharTypedPreEvent<'_>) -> Flow {
        if !shows_over(event.screen) {
            return Flow::Continue;
        }
        STATE.with_borrow_mut(|s| {
            if s.focused && !event.c.is_control() {
                s.query.push(event.c);
                s.query.truncate(50);
                refilter(s);
                Flow::Cancel
            } else {
                Flow::Continue
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_round_trip() {
        for m in [SearchMode::Enabled, SearchMode::RequireManualPrefix, SearchMode::Disabled] {
            assert_eq!(parse_mode(mode_name(m), SearchMode::Enabled), m);
        }
        assert_eq!(parse_mode("junk", SearchMode::Disabled), SearchMode::Disabled);
        assert_eq!(next_mode(next_mode(next_mode(SearchMode::Enabled))), SearchMode::Enabled);
    }

    #[test]
    fn hidden_matches_item_or_wildcard() {
        let hidden = vec!["minecraft:dirt".to_owned(), "*minecraft:stone".to_owned()];
        assert!(is_hidden(&hidden, "minecraft:dirt"));
        assert!(is_hidden(&hidden, "minecraft:stone"));
        assert!(!is_hidden(&hidden, "minecraft:sand"));
        assert!(!is_hidden(&hidden, "*minecraft:dirt"));
    }

    #[test]
    fn lines_skip_blanks() {
        assert_eq!(lines(Some(" a \n\n b\n".into())), ["a", "b"]);
        assert!(lines(None).is_empty());
    }

    fn completing(query: &str) -> State {
        let mut s = State::default();
        s.query = query.into();
        s.focused = true;
        s.config.set("search", "searchCompletionEnabled", "true");
        s.views = vec![PrefixView {
            prefix: '@',
            mode: SearchMode::RequireManualPrefix,
            dynamic: true,
            description: "modName".into(),
            strings: vec!["mekanism".into(), "minecraft".into()],
        }];
        s
    }

    #[test]
    fn completion_is_off_without_the_option_or_focus() {
        let mut s = completing("@m");
        s.focused = false;
        assert!(suggestions(&s).is_empty());
        let mut s = completing("@m");
        s.config.set("search", "searchCompletionEnabled", "false");
        assert!(suggestions(&s).is_empty());
    }

    #[test]
    fn tab_replaces_the_last_token() {
        let mut s = completing("dirt @me");
        let c = suggestions(&s).first().cloned().expect("a candidate");
        assert_eq!(c.insertion, "@mekanism");
        accept(&mut s, &c);
        assert_eq!(s.query, "dirt @mekanism");
    }

    #[test]
    fn tab_keeps_the_negation() {
        let mut s = completing("-@mi");
        let c = suggestions(&s).first().cloned().expect("a candidate");
        accept(&mut s, &c);
        assert_eq!(s.query, "-@minecraft");
    }

    #[test]
    fn lookups_are_bounded_and_newest_first() {
        let mut h = LookupHistory::new(10);
        for i in 0..12 {
            h.add(&format!("minecraft:i{i}"));
        }
        h.add("minecraft:i5");
        assert_eq!(h.items().len(), 10);
        assert_eq!(h.items()[0], "minecraft:i5");
    }
}
