//! The look of JEI's own widgets, drawn with fills: the game gives a mod no sprite drawing, so each texture of
//! `assets/jei/textures/gui/sprites` is rebuilt from its pixels (bevels and fills), as the Java mod's nine-slices read.
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
pub fn gui_background(draw: &mut dyn DrawContext, r: Rect) {
    fill(draw, r.x, r.y, r.w, r.h, 0xFF000000);
    fill(draw, r.x + 1, r.y + 1, r.w - 2, r.h - 2, 0xFFFFFFFF);
    fill(draw, r.x + 3, r.y + 3, r.w - 4, r.h - 4, 0xFF555555);
    fill(draw, r.x + 3, r.y + 3, r.w - 6, r.h - 6, 0xFFC6C6C6);
}

/// `single_recipe_background_v2`: a one pixel frame, lighter top and left, darker bottom and right.
pub fn recipe_background(draw: &mut dyn DrawContext, r: Rect) {
    fill(draw, r.x, r.y, r.w, r.h, 0xFF999999);
    fill(draw, r.x + 1, r.y + 1, r.w - 2, r.h - 2, 0xFFD8D8D8);
    fill(draw, r.x + 2, r.y + 2, r.w - 3, r.h - 3, 0xFFB3B3B3);
    fill(draw, r.x + 2, r.y + 2, r.w - 5, r.h - 5, 0xFFC6C6C6);
}

/// `slot.png` and `ingredient_list_slot_background_v2`: dark top and left, white bottom and right.
pub fn slot(draw: &mut dyn DrawContext, r: Rect) {
    fill(draw, r.x, r.y, r.w, r.h, 0xFF373737);
    fill(draw, r.x + 1, r.y + 1, r.w - 1, r.h - 1, 0xFFFFFFFF);
    fill(draw, r.x + 1, r.y + 1, r.w - 2, r.h - 2, 0xFF8B8B8B);
}

/// `button_enabled_v2` (and its highlight when hovered): black frame, light top and left, dark bottom and right.
pub fn button(draw: &mut dyn DrawContext, r: Rect, hovered: bool) {
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
pub fn arrow(draw: &mut dyn DrawContext, r: Rect, left: bool) {
    let (x0, y0) = (r.x + (r.w - 4) / 2, r.y + (r.h - 7) / 2);
    for i in 0..4 {
        let h = 2 * i + 1;
        let col = if left { i } else { 3 - i };
        fill(draw, x0 + col, y0 + 3 - i, 1, h, 0xFFFFFFFF);
    }
}

/// `recipe_transfer.png`: a plus 7 by 7 in the middle.
pub fn plus(draw: &mut dyn DrawContext, r: Rect) {
    let (x, y) = (r.x + (r.w - 7) / 2, r.y + (r.h - 7) / 2);
    fill(draw, x, y + 3, 7, 1, 0xFFFFFFFF);
    fill(draw, x + 3, y, 1, 7, 0xFFFFFFFF);
}

/// `recipe_arrow.png`: the 22 by 16 grey arrow of a recipe.
pub fn recipe_arrow(draw: &mut dyn DrawContext, x: i32, y: i32) {
    fill(draw, x, y + 6, 14, 4, 0xFF8B8B8B);
    for i in 0..8 {
        fill(draw, x + 14 + i, y + i, 1, 16 - 2 * i, 0xFF8B8B8B);
    }
}

/// A recipe tab (`tab_selected` and `tab`): the selected one is the panel's colour, the others darker.
pub fn tab(draw: &mut dyn DrawContext, r: Rect, selected: bool) {
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
pub fn cog(draw: &mut dyn DrawContext, r: Rect, color: u32) {
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
