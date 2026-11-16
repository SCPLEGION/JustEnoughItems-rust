//! The look of JEI's own widgets. The sprites of the Java mod ship in `assets/jei/textures/gui/sprites` and are drawn
//! with `DrawContext::sprite` and `gui_nine_slice` as `jei:gui/sprites/<name>`; where the game has not loaded them (they
//! are probed once) the same widgets are rebuilt from the sprites' pixels with fills.
use jei_core::grid::Rect;
use mod_api::DrawContext;

fn f(r: Rect) -> [f32; 4] {
    [r.x as f32, r.y as f32, r.w as f32, r.h as f32]
}

fn fill(draw: &mut dyn DrawContext, x: i32, y: i32, w: i32, h: i32, color: u32) {
    if w > 0 && h > 0 {
        draw.fill(f(Rect::new(x, y, w, h)), color);
    }
}

/// `gui_background_v2`: black outline, a two pixel white bevel top and left, dark grey bottom and right.
fn gui_background_fill(draw: &mut dyn DrawContext, r: Rect) {
    fill(draw, r.x, r.y, r.w, r.h, 0xFF000000);
    fill(draw, r.x + 1, r.y + 1, r.w - 2, r.h - 2, 0xFFFFFFFF);
    fill(draw, r.x + 3, r.y + 3, r.w - 4, r.h - 4, 0xFF555555);
    fill(draw, r.x + 3, r.y + 3, r.w - 6, r.h - 6, 0xFFC6C6C6);
}

/// `single_recipe_background_v2`: a one pixel frame, lighter top and left, darker bottom and right.
fn recipe_background_fill(draw: &mut dyn DrawContext, r: Rect) {
    fill(draw, r.x, r.y, r.w, r.h, 0xFF999999);
    fill(draw, r.x + 1, r.y + 1, r.w - 2, r.h - 2, 0xFFD8D8D8);
    fill(draw, r.x + 2, r.y + 2, r.w - 3, r.h - 3, 0xFFB3B3B3);
    fill(draw, r.x + 2, r.y + 2, r.w - 5, r.h - 5, 0xFFC6C6C6);
}

/// `slot.png` and `ingredient_list_slot_background_v2`: dark top and left, white bottom and right.
fn slot_fill(draw: &mut dyn DrawContext, r: Rect) {
    fill(draw, r.x, r.y, r.w, r.h, 0xFF373737);
    fill(draw, r.x + 1, r.y + 1, r.w - 1, r.h - 1, 0xFFFFFFFF);
    fill(draw, r.x + 1, r.y + 1, r.w - 2, r.h - 2, 0xFF8B8B8B);
}

/// `button_enabled_v2` (and its highlight when hovered): black frame, light top and left, dark bottom and right.
fn button_fill(draw: &mut dyn DrawContext, r: Rect, hovered: bool) {
    fill(draw, r.x, r.y, r.w, r.h, 0xFF000000);
    fill(draw, r.x + 1, r.y + 1, r.w - 2, r.h - 2, 0xFFAAAAAA);
    fill(draw, r.x + 2, r.y + 2, r.w - 4, r.h - 4, 0xFF6E6E6E);
    fill(draw, r.x + 2, r.y + r.h - 3, r.w - 3, 2, 0xFF565656);
    fill(draw, r.x + r.w - 3, r.y + 2, 2, r.h - 3, 0xFF565656);
    if hovered {
        fill(draw, r.x + 1, r.y + 1, r.w - 2, r.h - 2, 0x80FFFFFF);
    }
}

/// `arrow_previous_v2` and `arrow_next_v2`: a 4 by 7 triangle centred in the button.
fn arrow_fill(draw: &mut dyn DrawContext, r: Rect, left: bool) {
    let (x0, y0) = (r.x + (r.w - 4) / 2, r.y + (r.h - 7) / 2);
    for i in 0..4 {
        let h = 2 * i + 1;
        let col = if left { i } else { 3 - i };
        fill(draw, x0 + col, y0 + 3 - i, 1, h, 0xFFFFFFFF);
    }
}

/// `recipe_transfer.png`: a plus 7 by 7 in the middle.
fn plus_fill(draw: &mut dyn DrawContext, r: Rect) {
    let (x, y) = (r.x + (r.w - 7) / 2, r.y + (r.h - 7) / 2);
    fill(draw, x, y + 3, 7, 1, 0xFFFFFFFF);
    fill(draw, x + 3, y, 1, 7, 0xFFFFFFFF);
}

/// `recipe_arrow.png`: the 22 by 16 grey arrow of a recipe.
fn recipe_arrow_fill(draw: &mut dyn DrawContext, x: i32, y: i32) {
    fill(draw, x, y + 6, 14, 4, 0xFF8B8B8B);
    for i in 0..8 {
        fill(draw, x + 14 + i, y + i, 1, 16 - 2 * i, 0xFF8B8B8B);
    }
}

/// A recipe tab (`tab_selected` and `tab`): the selected one is the panel's colour, the others darker.
fn tab_fill(draw: &mut dyn DrawContext, r: Rect, selected: bool) {
    if selected {
        fill(draw, r.x, r.y, r.w, r.h, 0xFF000000);
        fill(draw, r.x + 1, r.y + 1, r.w - 2, r.h - 1, 0xFFFFFFFF);
        fill(draw, r.x + 3, r.y + 3, r.w - 6, r.h - 3, 0xFFC6C6C6);
    } else {
        fill(draw, r.x, r.y + 2, r.w, r.h - 2, 0xFF000000);
        fill(draw, r.x + 1, r.y + 3, r.w - 2, r.h - 3, 0xFFAAAAAA);
        fill(draw, r.x + 2, r.y + 4, r.w - 3, r.h - 4, 0xFF8B8B8B);
    }
}

/// `config_button.png`: a cog in `color`, 12 by 12 in the middle.
fn cog_fill(draw: &mut dyn DrawContext, r: Rect, color: u32) {
    let (x, y) = (r.x + (r.w - 12) / 2, r.y + (r.h - 12) / 2);
    fill(draw, x + 3, y + 3, 6, 6, color);
    fill(draw, x + 4, y, 4, 12, color);
    fill(draw, x, y + 4, 12, 4, color);
    fill(draw, x + 1, y + 1, 2, 2, color);
    fill(draw, x + 9, y + 1, 2, 2, color);
    fill(draw, x + 1, y + 9, 2, 2, color);
    fill(draw, x + 9, y + 9, 2, 2, color);
    fill(draw, x + 5, y + 5, 2, 2, 0xFF6E6E6E);
}

use std::cell::Cell;

thread_local! {
    /// Whether the mod's own sprites are in the GUI atlas (probed on the first draw).
    static HAVE_SPRITES: Cell<Option<bool>> = const { Cell::new(None) };
}

fn name(sprite: &str) -> String {
    format!("jei:gui/sprites/{sprite}")
}

fn have(draw: &mut dyn DrawContext) -> bool {
    HAVE_SPRITES.with(|c| match c.get() {
        Some(v) => v,
        None => {
            // A sprite drawn into an empty box: nothing shows, and the answer says whether it exists.
            let v = draw.sprite(&name("gui_background_v2"), [0.0, 0.0, 1.0, 1.0], [0.0, 0.0, 0.0, 0.0], None);
            c.set(Some(v));
            v
        }
    })
}

/// The whole sprite `sprite` (`w` by `h` pixels) at `(x, y)`.
fn blit(draw: &mut dyn DrawContext, sprite: &str, x: i32, y: i32, w: i32, h: i32) {
    draw.sprite(&name(sprite), [0.0, 0.0, w as f32, h as f32], [x as f32, y as f32, w as f32, h as f32], None);
}

/// `gui_background_v2`: the panel of the recipe screen and the overlays.
pub fn gui_background(draw: &mut dyn DrawContext, r: Rect) {
    if have(draw) {
        draw.gui_nine_slice(&name("gui_background_v2"), f(r), 0xFFFFFFFF);
    } else {
        gui_background_fill(draw, r);
    }
}

/// `single_recipe_background_v2`: behind each recipe.
pub fn recipe_background(draw: &mut dyn DrawContext, r: Rect) {
    if have(draw) {
        draw.gui_nine_slice(&name("single_recipe_background_v2"), f(r), 0xFFFFFFFF);
    } else {
        recipe_background_fill(draw, r);
    }
}

/// `slot.png` (a recipe's slot) and the overlays' slot backgrounds (the same 18 by 18 look).
pub fn slot(draw: &mut dyn DrawContext, r: Rect) {
    if have(draw) {
        blit(draw, "slot", r.x, r.y, 18, 18);
    } else {
        slot_fill(draw, r);
    }
}

/// `button_enabled_v2`, with `button_highlight_v2` over it when hovered.
pub fn button(draw: &mut dyn DrawContext, r: Rect, hovered: bool) {
    if have(draw) {
        draw.gui_nine_slice(&name("button_enabled_v2"), f(r), 0xFFFFFFFF);
        if hovered {
            draw.gui_nine_slice(&name("button_highlight_v2"), f(r), 0xFFFFFFFF);
        }
    } else {
        button_fill(draw, r, hovered);
    }
}

/// `arrow_previous_v2` and `arrow_next_v2` (9 by 7) centred in the button.
pub fn arrow(draw: &mut dyn DrawContext, r: Rect, left: bool) {
    if have(draw) {
        let sprite = if left { "icons/arrow_previous_v2" } else { "icons/arrow_next_v2" };
        blit(draw, sprite, r.x + (r.w - 9) / 2, r.y + (r.h - 7) / 2, 9, 7);
    } else {
        arrow_fill(draw, r, left);
    }
}

/// `recipe_transfer.png` (7 by 7) centred in the button.
pub fn plus(draw: &mut dyn DrawContext, r: Rect) {
    if have(draw) {
        blit(draw, "icons/recipe_transfer", r.x + (r.w - 7) / 2, r.y + (r.h - 7) / 2, 7, 7);
    } else {
        plus_fill(draw, r);
    }
}

/// `recipe_arrow.png` (22 by 16).
pub fn recipe_arrow(draw: &mut dyn DrawContext, x: i32, y: i32) {
    if have(draw) {
        blit(draw, "recipe_arrow", x, y, 22, 16);
    } else {
        recipe_arrow_fill(draw, x, y);
    }
}

/// `tab_selected` and `tab_unselected` (24 by 24).
pub fn tab(draw: &mut dyn DrawContext, r: Rect, selected: bool) {
    if have(draw) {
        blit(draw, if selected { "tab_selected" } else { "tab_unselected" }, r.x, r.y, 24, 24);
    } else {
        tab_fill(draw, r, selected);
    }
}

/// `config_button.png` and `config_button_cheat.png` (16 by 16) in the middle of the button.
pub fn cog(draw: &mut dyn DrawContext, r: Rect, cheating: bool) {
    if have(draw) {
        blit(draw, if cheating { "icons/config_button_cheat" } else { "icons/config_button" }, r.x + (r.w - 16) / 2, r.y + (r.h - 16) / 2, 16, 16);
    } else {
        cog_fill(draw, r, if cheating { 0xFFFF5555 } else { 0xFFDDDDDD });
    }
}

/// The side tabs of the recipe screen: `catalyst_tab_v2`, `recipe_catalyst_slot_background_v2` and `recipe_options_tab_v2`.
pub fn side_tab(draw: &mut dyn DrawContext, r: Rect, sprite: &str) {
    if have(draw) {
        draw.gui_nine_slice(&name(sprite), f(r), 0xFFFFFFFF);
    } else {
        gui_background_fill(draw, r);
    }
}

/// A 16 by 16 icon button: `button_enabled_v2`, `button_pressed_v2` while on, the icon over it.
pub fn icon_button(draw: &mut dyn DrawContext, r: Rect, icon: &str, on: bool, hovered: bool) {
    if have(draw) {
        draw.gui_nine_slice(&name(if on { "button_pressed_v2" } else { "button_enabled_v2" }), f(r), 0xFFFFFFFF);
        blit(draw, icon, r.x, r.y, 16, 16);
        if hovered {
            draw.gui_nine_slice(&name(if on { "button_pressed_highlight_v2" } else { "button_highlight_v2" }), f(r), 0xFFFFFFFF);
        }
    } else {
        button_fill(draw, r, hovered || on);
    }
}
