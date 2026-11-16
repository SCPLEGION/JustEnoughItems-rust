//! JEI on Horizon's own mod API: the ingredient list over container screens. All logic is `jei-core`.
use jei_core::{
    grid::{self, GridConfig, Rect},
    ingredient::{FilterConfig, IngredientData, ListElementInfo},
    prefix::{default_mode, SearchIndex, SearchMode, PREFIXES},
    search::ElementSearch,
};
use mod_api::{
    self, ClientHost, DrawContext, Hooks, ItemKey, ItemStack, ScreenCharTypedPreEvent, ScreenKeyPressedPreEvent,
    ScreenMouseButtonPressedPreEvent, ScreenMouseScrolledPreEvent, ClientTickPostEvent, Flow, ScreenRenderPostEvent, ScreenView,
};
use std::cell::RefCell;

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
}

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
    let mut search = ElementSearch::new(|i| default_mode(i));
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
        let info = ListElementInfo::new(data, &cfg, i);
        search.add(i, |idx: SearchIndex| info.strings(idx, &cfg));
        s.infos.push(info);
        s.keys.push(key);
        s.ids.push(id);
    }
    s.results = search.query("");
    s.search = Some(search);
    s.loaded = true;
}

fn refilter(s: &mut State) {
    if let Some(search) = &s.search {
        s.results = search.query(&s.query);
    }
    s.page = 0;
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
            if !s.loaded {
                load(host, s);
            }
            if !s.loaded {
                return;
            }
            let [bw, bh] = draw.size();
            let (bw, bh) = (bw as i32, bh as i32);
            let left = (screen.rect[0] + screen.rect[2]) as i32 + MARGIN;
            let area = Rect::new(left, NAV_H, bw - left - MARGIN, bh - NAV_H - SEARCH_H - 2 * MARGIN);
            let bounds = grid::calculate_bounds(&GridConfig { max_columns: 100, max_rows: 100, ..GridConfig::ingredient_list() }, area, false);
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
            for (n, slot) in slots.iter().enumerate() {
                let Some(&idx) = s.results.get(first as usize + n) else { break };
                let a = slot.area;
                draw.item(s.keys[idx], [(a.x + 1) as f32, (a.y + 1) as f32], 16.0);
                if a.contains(cx, cy) {
                    draw.fill([(a.x + 1) as f32, (a.y + 1) as f32, 16.0, 16.0], HOVER);
                }
            }
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
            draw.text(&shown, [(search.x + 4) as f32, (search.y + 6) as f32], if s.query.is_empty() { 0xFF909090 } else { TEXT }, false);
            let _ = PREFIXES.len() + usize::from(SearchMode::Enabled == SearchMode::Enabled);
        });
    }

    fn screen_mouse_button_pressed_pre(&self, _: &dyn ClientHost, event: &mut ScreenMouseButtonPressedPreEvent<'_>) -> Flow {
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
                    // Cheat mode: left gives a stack, right one (needs creative mode).
                    s.give = Some((s.keys[idx], if right { 1 } else { 64 }));
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

    fn screen_key_pressed_pre(&self, _: &dyn ClientHost, event: &mut ScreenKeyPressedPreEvent<'_>) -> Flow {
        if !shows_over(event.screen) {
            return Flow::Continue;
        }
        STATE.with_borrow_mut(|s| {
            if !s.focused {
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
