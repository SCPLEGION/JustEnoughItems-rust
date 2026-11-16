//! Port of `IngredientListOverlayLayout` and `IngredientGridWithNavigationLayout` (search bar at the bottom, not
//! centred; slot background style on): where the ingredient grid, its page navigation, its background, the search
//! field and the config button go.
use crate::jei_core::grid::Rect;

pub const BORDER_MARGIN: i32 = 6;
pub const INNER_PADDING: i32 = 2;
pub const BORDER_PADDING: i32 = 5;
pub const BUTTON_SIZE: i32 = 20;
pub const SEARCH_HEIGHT: i32 = BUTTON_SIZE;
pub const NAVIGATION_HEIGHT: i32 = 20;

fn inset(r: Rect, n: i32) -> Rect {
    Rect::new(r.x + n, r.y + n, (r.w - 2 * n).max(0), (r.h - 2 * n).max(0))
}

fn union(a: Rect, b: Rect) -> Rect {
    let (x, y) = (a.x.min(b.x), a.y.min(b.y));
    Rect::new(x, y, (a.x + a.w).max(b.x + b.w) - x, (a.y + a.h).max(b.y + b.h) - y)
}

/// The room the grid may use (`getAvailableGridArea` over the display area less the search row).
pub fn available_grid_area(gui_right: i32, screen_w: i32, screen_h: i32) -> Rect {
    available_grid_area_in(Rect::new(gui_right, 0, (screen_w - gui_right).max(0), screen_h), true)
}

/// The same for any display area: the bookmark list, left of the window, has no search row (`reserve_search` false).
pub fn available_grid_area_in(display: Rect, reserve_search: bool) -> Rect {
    let contents = if reserve_search { Rect::new(display.x, display.y, display.w, (display.h - (SEARCH_HEIGHT + INNER_PADDING)).max(0)) } else { display };
    let a = inset(contents, BORDER_MARGIN);
    let a = Rect::new(a.x, a.y + NAVIGATION_HEIGHT + INNER_PADDING, a.w, (a.h - (NAVIGATION_HEIGHT + INNER_PADDING)).max(0));
    inset(a, BORDER_PADDING + INNER_PADDING)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Areas {
    /// The grid expanded by `INNER_PADDING`.
    pub slot_background: Rect,
    /// The page navigation row, above the slot background.
    pub navigation: Rect,
    /// The panel behind it all.
    pub background: Rect,
    pub search: Rect,
    pub config_button: Rect,
}

/// The areas once the grid's bounds are known (`calculateBounds` of the available area).
pub fn areas(gui_right: i32, screen_w: i32, screen_h: i32, grid: Rect) -> Areas {
    let display = Rect::new(gui_right, 0, (screen_w - gui_right).max(0), screen_h);
    let slot_background = grid.expand_by(INNER_PADDING);
    let navigation = Rect::new(slot_background.x, slot_background.y - (NAVIGATION_HEIGHT + INNER_PADDING), slot_background.w, NAVIGATION_HEIGHT);
    let background = union(slot_background, navigation).expand_by(BORDER_PADDING);
    // `getSearchAndConfigArea`: the bottom row of the inset display area, over the contents' width when there is room.
    let row = inset(display, BORDER_MARGIN);
    let (x, w) = if grid.is_empty() { (row.x, row.w) } else { (background.x, background.w) };
    let row = Rect::new(x, row.y + row.h - SEARCH_HEIGHT, w, SEARCH_HEIGHT);
    if row.w < BUTTON_SIZE || row.h < SEARCH_HEIGHT {
        return Areas { slot_background, navigation, background, search: Rect::EMPTY, config_button: Rect::EMPTY };
    }
    Areas {
        slot_background,
        navigation,
        background,
        search: Rect::new(row.x, row.y, row.w - BUTTON_SIZE, row.h),
        config_button: Rect::new(row.x + row.w - BUTTON_SIZE, row.y, BUTTON_SIZE, row.h),
    }
}

/// `PageNavigation.updateBounds`: the square buttons at both ends of a row.
pub fn nav_buttons(area: Rect) -> (Rect, Rect) {
    let size = area.h.min(area.w / 2);
    (Rect::new(area.x, area.y, size, area.h), Rect::new(area.x + area.w - size, area.y, size, area.h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_room_and_areas() {
        // A 854x480 screen with the inventory ending at x = 507.
        let avail = available_grid_area(507, 854, 480);
        assert_eq!(avail, Rect::new(507 + 13, 28 + 7, 347 - 26, 480 - 22 - 12 - 22 - 14));
        let grid = Rect::new(avail.x, avail.y, 306, 396);
        let a = areas(507, 854, 480, grid);
        assert_eq!(a.slot_background, grid.expand_by(2));
        assert_eq!(a.navigation, Rect::new(grid.x - 2, grid.y - 2 - 22, 310, 20));
        assert_eq!(a.background, Rect::new(grid.x - 7, grid.y - 2 - 22 - 5, 320, 396 + 4 + 22 + 10));
        assert_eq!(a.search.h, 20);
        assert_eq!(a.search.y, 480 - 6 - 20);
        assert_eq!(a.config_button.x + 20, a.background.x + a.background.w);
        assert_eq!(a.search.x + a.search.w, a.config_button.x);
    }

    #[test]
    fn no_grid_spans_the_display() {
        let a = areas(500, 854, 480, Rect::EMPTY);
        assert_eq!(a.search.x, 506);
        assert_eq!(a.search.w + 20, 854 - 500 - 12);
    }

    #[test]
    fn nav_buttons_are_square() {
        let (b, n) = nav_buttons(Rect::new(10, 10, 100, 20));
        assert_eq!((b.w, b.h), (20, 20));
        assert_eq!(n.x, 90);
    }
}
