//! JEI on Horizon's own mod API: the ingredient list over container screens. All logic is `jei-core`.
use jei_core::{
    grid::{self, GridConfig, Rect},
    recipes::{Focus, RecipeId, Role},
    view::SlotRole,
    bookmarks::BookmarkList,
    highlight,
    ingredient::{FilterConfig, IngredientData, ListElementInfo},
    prefix::{default_mode, SearchIndex, SearchMode, PREFIXES},
    search::ElementSearch,
};
use mod_api::{
    self, ClientHost, DrawContext, Hooks, ItemKey, ItemStack, ScreenCharTypedPreEvent, ScreenKeyPressedPreEvent,
    ScreenMouseButtonPressedPreEvent, ScreenMouseScrolledPreEvent, ClientTickPostEvent, Flow, ScreenRenderPostEvent, ScreenView,
};
mod recipes;

use std::{cell::RefCell, collections::HashMap, time::Instant};

const MARGIN: i32 = 6;
const NAV_H: i32 = 20;
const SEARCH_H: i32 = 18;
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
    cheat_button: Rect,
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
    screen_size: [i32; 2],
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
}

fn options_rows(s: &State, bw: i32, bh: i32) -> (Rect, Vec<Rect>) {
    let w = 200;
    let n = MODE_ROWS.len() as i32 + 3;
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
    matches!(screen.id, "inventory" | "creative")
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
    s.view = Some(View { cats, cat: 0, page: 0, recipes, focus });
}

fn select_category(s: &mut State, cat: usize) {
    let (Some(r), Some(v)) = (s.recipes.as_ref(), s.view.as_mut()) else { return };
    v.cat = cat;
    v.page = 0;
    v.recipes = r.mgr.recipes(&r.cats[v.cats[cat]].recipe_type, std::slice::from_ref(&v.focus), false, &|_| true);
}

fn cycle(s: &State, list: &[String]) -> Option<String> {
    if list.is_empty() {
        return None;
    }
    let ms = s.started.map_or(0, |t| t.elapsed().as_millis());
    Some(list[(ms / CYCLE_MS) as usize % list.len()].clone())
}

/// `RecipesGui`: the panel over the screen with the category tabs, the recipe's layout and its page arrows.
fn draw_view(s: &mut State, draw: &mut dyn DrawContext, screen: &ScreenView<'_>) {
    let (Some(v), Some(r)) = (s.view.clone(), s.recipes.as_ref()) else { return };
    let cat = r.cats[v.cats[v.cat]].clone();
    let [bw, bh] = draw.size();
    let (bw, bh) = (bw as i32, bh as i32);
    let ui = v.recipes.get(v.page).and_then(|id| r.ui.get(*id as usize)).cloned();
    let lay = cat.layout_for(ui.as_ref().map_or((3, 3), |u| u.grid), ui.as_ref().map_or(0, |u| u.inputs.len()));
    let pw = (lay.width + 24).max(176);
    let ph = lay.height + 78;
    let panel = Rect::new((bw - pw) / 2, (bh - ph) / 2, pw, ph);
    s.panel = panel;
    let (cx, cy) = (screen.mouse[0] as i32, screen.mouse[1] as i32);
    draw.fill(rect_f(panel), BG);
    draw.fill(rect_f(Rect::new(panel.x + 1, panel.y + 1, panel.w - 2, panel.h - 2)), 0xFFC6C6C6);
    for (i, ci) in v.cats.iter().enumerate() {
        let t = Rect::new(panel.x + 4 + i as i32 * 22, panel.y + 4, 20, 20);
        draw.fill(rect_f(t), if i == v.cat { 0xFFFFFFFF } else { 0xFF8B8B8B });
        if let Some(&k) = s.index.get(&r.cats[*ci].icon) {
            draw.item(s.keys[k], [(t.x + 2) as f32, (t.y + 2) as f32], 16.0);
        }
    }
    draw.text(&cat.title, [(panel.x + 6) as f32, (panel.y + 28) as f32], 0xFF404040, false);
    let origin = (panel.x + (pw - lay.width) / 2, panel.y + 42);
    let mut hot = Vec::new();
    if let Some(u) = ui {
        if let Some((ax, ay)) = lay.arrow {
            draw.text("->", [(origin.0 + ax) as f32, (origin.1 + ay + 4) as f32], 0xFF404040, false);
        }
        for slot in &lay.slots {
            let rect = Rect::new(origin.0 + slot.x, origin.1 + slot.y, 18, 18);
            draw.fill(rect_f(rect), SLOT_BG);
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
    }
    let label = format!("{}/{}", v.page + 1, v.recipes.len().max(1));
    let lw = draw.text_width(&label) as i32;
    draw.text("<", [(panel.x + 8) as f32, (panel.y + ph - 14) as f32], 0xFF404040, false);
    draw.text(">", [(panel.x + pw - 14) as f32, (panel.y + ph - 14) as f32], 0xFF404040, false);
    draw.text(&label, [(panel.x + (pw - lw) / 2) as f32, (panel.y + ph - 14) as f32], 0xFF404040, false);
    if let Some(u) = v.recipes.get(v.page).and_then(|id| r.ui.get(*id as usize)).filter(|u| !u.note.is_empty()) {
        draw.text(&u.note, [(origin.0 + (lay.width - draw.text_width(&u.note) as i32) / 2) as f32, (origin.1 + lay.height + 4) as f32], 0xFF404040, false);
    }
    s.hot = hot;
}

/// A press inside the open view; true when it was the view's (always, inside the panel).
fn view_click(s: &mut State, x: i32, y: i32, right: bool) -> bool {
    let p = s.panel;
    if !p.contains(x, y) {
        s.view = None;
        s.history.clear();
        return true;
    }
    let cats = s.view.as_ref().map_or(0, |v| v.cats.len());
    for i in 0..cats {
        if Rect::new(p.x + 4 + i as i32 * 22, p.y + 4, 20, 20).contains(x, y) {
            select_category(s, i);
            return true;
        }
    }
    if y >= p.y + p.h - 18 {
        if let Some(v) = s.view.as_mut() {
            let n = v.recipes.len().max(1);
            v.page = if x < p.x + p.w / 2 { (v.page + n - 1) % n } else { (v.page + 1) % n };
        }
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

#[mod_api::horizon_mod]
impl Hooks for Mod {
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
            let [bw, bh] = draw.size();
            let (bw, bh) = (bw as i32, bh as i32);
            s.screen_size = [bw, bh];
            let left = (screen.rect[0] + screen.rect[2]) as i32 + MARGIN;
            let area = Rect::new(left, NAV_H, bw - left - MARGIN, bh - NAV_H - SEARCH_H - 2 * MARGIN);
            let bounds = grid::calculate_bounds(&GridConfig { max_columns: s.columns, max_rows: s.rows, ..GridConfig::ingredient_list() }, area, false);
            s.grid = bounds;
            s.nav = Rect::new(bounds.x, 2, bounds.w, 16);
            s.search_box = Rect::new(bounds.x.max(left), bh - SEARCH_H - MARGIN, bounds.w.max(80), SEARCH_H);
            if bounds.is_empty() {
                return;
            }
            let (cx, cy) = (screen.mouse[0] as i32, screen.mouse[1] as i32);
            let slots = grid::calculate_slots(bounds, &[], None, 0);
            let per_page = slots.len() as i32;
            let first = grid::first_item_index_for_valid_page(s.page * per_page, s.results.len() as i32, per_page);
            s.page = if per_page == 0 { 0 } else { first / per_page };
            let mut hot = Vec::new();
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
            let marks_area = Rect::new(MARGIN, NAV_H, screen.rect[0] as i32 - 2 * MARGIN, bh - NAV_H - 2 * MARGIN);
            let marks = grid::calculate_bounds(&GridConfig { max_columns: s.columns, max_rows: s.rows, ..GridConfig::bookmark_list() }, marks_area, false);
            s.marks = marks;
            if !marks.is_empty() {
                for (slot, id) in grid::calculate_slots(marks, &[], None, 0).iter().zip(s.bookmarks.items().to_vec()) {
                    let a = slot.area;
                    draw.fill([(a.x + 1) as f32, (a.y + 1) as f32, 16.0, 16.0], 0x40FFFF00);
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
            draw.text("<", [(nav.x + 4) as f32, (nav.y + 4) as f32], TEXT, true);
            draw.text(">", [(nav.x + nav.w - 10) as f32, (nav.y + 4) as f32], TEXT, true);
            draw.text(&label, [(nav.x + (nav.w - lw) / 2) as f32, (nav.y + 4) as f32], TEXT, true);
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
            let _ = PREFIXES.len() + usize::from(SearchMode::Enabled == SearchMode::Enabled);
            s.started.get_or_insert_with(Instant::now);
            s.cheat_button = Rect::new(search.x + search.w + 4, search.y + 2, 44, 14);
            draw.fill(rect_f(s.cheat_button), if s.cheat { 0xFF55AA55 } else { 0xFF555555 });
            draw.text("Cheat", [(s.cheat_button.x + 4) as f32, (s.cheat_button.y + 3) as f32], TEXT, false);
            s.options_button = Rect::new(s.cheat_button.x + s.cheat_button.w + 4, s.cheat_button.y, 44, 14);
            draw.fill(rect_f(s.options_button), 0xFF555555);
            draw.text("Options", [(s.options_button.x + 4) as f32, (s.options_button.y + 3) as f32], TEXT, false);
            if s.options {
                draw_options(s, draw);
            }
            draw_view(s, draw, screen);
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
                s.options = true;
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
            if s.cheat_button.contains(x, y) {
                s.cheat = !s.cheat;
                host.set_setting("jei", "cheat_mode", if s.cheat { "true" } else { "false" });
                claimed = true;
                return;
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
        if let Some((item, count)) = STATE.with_borrow_mut(|s| s.give.take()) {
            let _ = host.give(item, count);
        }
    }

    fn screen_mouse_scrolled_pre(&self, _: &dyn ClientHost, event: &mut ScreenMouseScrolledPreEvent<'_>) -> Flow {
        if !shows_over(event.screen) {
            return Flow::Continue;
        }
        let (x, y) = (event.screen.mouse[0] as i32, event.screen.mouse[1] as i32);
        STATE.with_borrow_mut(|s| {
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
            if !s.focused {
                match event.key {
                    "key.keyboard.backspace" if s.view.is_some() => {
                        // `HistoryList`: Backspace goes back to the earlier recipe view, then closes.
                        s.view = s.history.pop();
                        return Flow::Cancel;
                    }
                    "key.keyboard.escape" if s.view.is_some() => {
                        s.view = None;
                        s.history.clear();
                        return Flow::Cancel;
                    }
                    "key.keyboard.a" => {
                        // `jei.key.bookmark` (A): bookmark the ingredient under the cursor, or remove it.
                        let (x, y) = (event.screen.mouse[0] as i32, event.screen.mouse[1] as i32);
                        if let Some((_, id)) = s.hot.iter().find(|(r, _)| r.contains(x, y)).cloned() {
                            s.bookmarks.toggle(&id);
                            host.set_setting("jei", "bookmarks", &s.bookmarks.save());
                            return Flow::Cancel;
                        }
                    }
                    "key.keyboard.r" | "key.keyboard.u" => {
                        // `jei.key.showRecipe` (R) and `jei.key.showUses` (U) on the stack under the cursor.
                        let (x, y) = (event.screen.mouse[0] as i32, event.screen.mouse[1] as i32);
                        let over = s.hot.iter().find(|(r, _)| r.contains(x, y)).map(|(_, id)| id.clone()).or_else(|| {
                            host.screen_hovered_item().and_then(|k| s.keys.iter().position(|x| *x == k)).map(|i| s.ids[i].clone())
                        });
                        if let Some(id) = over {
                            open_recipes(s, &id, if event.key == "key.keyboard.r" { Role::Output } else { Role::Input });
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
