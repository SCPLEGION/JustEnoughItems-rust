//! Port of `IngredientGridLayout` and `IngredientGridPageState`'s page arithmetic: how many ingredient slots fit in
//! the space beside a screen, which slots the screen's own areas block, and paging over the filtered list.

pub const INGREDIENT_PADDING: i32 = 1;
const BASE: i32 = 16;
pub const INGREDIENT_WIDTH: i32 = BASE + 2 * INGREDIENT_PADDING;
pub const INGREDIENT_HEIGHT: i32 = BASE + 2 * INGREDIENT_PADDING;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const EMPTY: Rect = Rect { x: 0, y: 0, w: 0, h: 0 };
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
    pub fn intersects(&self, o: &Rect) -> bool {
        self.x < o.x + o.w && o.x < self.x + self.w && self.y < o.y + o.h && o.y < self.y + self.h
    }
    pub fn expand_by(&self, n: i32) -> Rect {
        Rect { x: self.x - n, y: self.y - n, w: self.w + 2 * n, h: self.h + 2 * n }
    }
    pub fn is_empty(&self) -> bool {
        self.w <= 0 || self.h <= 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HorizontalAlignment {
    Left,
    Center,
    Right,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

/// Java's `Math.round(float)`: half rounds up.
fn round_half_up(v: f32) -> i32 {
    (v + 0.5).floor() as i32
}

impl HorizontalAlignment {
    pub fn x_pos(self, available: i32, element: i32) -> i32 {
        match self {
            Self::Left => 0,
            Self::Center => round_half_up((available - element) as f32 / 2.0),
            Self::Right => available - element,
        }
    }
}
impl VerticalAlignment {
    pub fn y_pos(self, available: i32, element: i32) -> i32 {
        match self {
            Self::Top => 0,
            Self::Center => round_half_up((available - element) as f32 / 2.0),
            Self::Bottom => available - element,
        }
    }
}

/// `IIngredientGridConfig`: JEI's defaults are 16 rows by 9 columns (limits 1..100 and 2..100); the ingredient list
/// aligns right, the bookmark list left.
#[derive(Clone, Copy, Debug)]
pub struct GridConfig {
    pub max_columns: i32,
    pub max_rows: i32,
    pub horizontal: HorizontalAlignment,
    pub vertical: VerticalAlignment,
}
pub const MIN_COLUMNS: i32 = 2;
pub const MIN_ROWS: i32 = 1;

impl GridConfig {
    pub fn ingredient_list() -> Self {
        Self { max_columns: 9, max_rows: 16, horizontal: HorizontalAlignment::Right, vertical: VerticalAlignment::Top }
    }
    pub fn bookmark_list() -> Self {
        Self { horizontal: HorizontalAlignment::Left, ..Self::ingredient_list() }
    }
}

/// `calculateSize`: the grid's size in the available area (`(0, 0)` when too small).
pub fn calculate_size(config: &GridConfig, area: Rect, allow_partial_rows: bool) -> (i32, i32) {
    let columns = (area.w / INGREDIENT_WIDTH).min(config.max_columns);
    let rows = (area.h / INGREDIENT_HEIGHT).min(config.max_rows);
    if rows < MIN_ROWS || columns < MIN_COLUMNS {
        return (0, 0);
    }
    let mut height = rows * INGREDIENT_HEIGHT;
    if allow_partial_rows && rows < config.max_rows {
        height = area.h;
    }
    (columns * INGREDIENT_WIDTH, height)
}

/// `calculateBounds`.
pub fn calculate_bounds(config: &GridConfig, area: Rect, allow_partial_rows: bool) -> Rect {
    let (w, h) = calculate_size(config, area, allow_partial_rows);
    Rect {
        x: area.x + config.horizontal.x_pos(area.w, w),
        y: area.y + config.vertical.y_pos(area.h, h),
        w,
        h,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotLayout {
    pub area: Rect,
    pub blocked: bool,
}

/// `calculateSlots`: every cell of the area (row by row), blocked when an exclusion area (grown by 2) touches it or
/// the mouse exclusion point is in it. `smooth_scroll_row_pixel_offset` shifts the rows up.
pub fn calculate_slots(area: Rect, exclusions: &[Rect], mouse_exclusion: Option<(i32, i32)>, smooth_scroll_row_pixel_offset: i32) -> Vec<SlotLayout> {
    let mut out = Vec::new();
    let offset = smooth_scroll_row_pixel_offset.clamp(0, INGREDIENT_HEIGHT - 1);
    let mut y = area.y - offset;
    while y < area.y + area.h {
        let mut x = area.x;
        while x < area.x + area.w {
            let slot = Rect::new(x, y, INGREDIENT_WIDTH, INGREDIENT_HEIGHT);
            let blocked = exclusions.iter().any(|e| e.intersects(&slot.expand_by(2)))
                || mouse_exclusion.is_some_and(|(mx, my)| slot.contains(mx, my));
            out.push(SlotLayout { area: slot, blocked });
            x += INGREDIENT_WIDTH;
        }
        y += INGREDIENT_HEIGHT;
    }
    out
}

/// `calculateAvailableSlotCount`.
pub fn available_slot_count(area: Rect, exclusions: &[Rect]) -> usize {
    calculate_slots(area, exclusions, None, 0).iter().filter(|s| !s.blocked).count()
}

/// `getFirstItemIndexForValidPage`: the start of the page holding `first_item_index`, clamped to the last page.
pub fn first_item_index_for_valid_page(first_item_index: i32, item_count: i32, items_per_page: i32) -> i32 {
    if item_count == 0 || items_per_page == 0 {
        return 0;
    }
    let requested = (first_item_index.max(0) / items_per_page) * items_per_page;
    let last = ((item_count - 1) / items_per_page) * items_per_page;
    requested.min(last)
}

/// `getPageCount`.
pub fn page_count(item_count: i32, items_per_page: i32) -> i32 {
    if items_per_page == 0 {
        return 1;
    }
    ((item_count + items_per_page - 1) / items_per_page).max(1)
}

/// `getPageNumberForFirstItemIndex` (0-based).
pub fn page_number(first_item_index: i32, items_per_page: i32, item_count: i32) -> i32 {
    let first = first_item_index_for_valid_page(first_item_index, item_count, items_per_page);
    if items_per_page == 0 { 0 } else { first / items_per_page }
}

/// `findNextSlot`: step through the slots by a cell or a row; `None` when no occupied slot remains.
pub fn find_next_slot(occupied: &[bool], columns: i32, current: i32, step: i32, cross_rows: bool) -> Option<i32> {
    let len = occupied.len() as i32;
    let mut next = current + step;
    while next >= 0 && next < len {
        if !cross_rows && next / columns != current / columns {
            return None;
        }
        if occupied[next as usize] {
            return Some(next);
        }
        next += step;
    }
    if step.abs() > 1 {
        let column = current.rem_euclid(columns);
        let first_row = current.div_euclid(columns) + step.signum();
        let mut row = first_row * columns;
        while row >= 0 && row < len {
            for offset in 1..columns {
                let (left, right) = (row + column - offset, row + column + offset);
                if left >= row && left < len && occupied[left as usize] {
                    return Some(left);
                }
                if right < row + columns && right < len && occupied[right as usize] {
                    return Some(right);
                }
            }
            row += step;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_and_bounds() {
        let c = GridConfig::ingredient_list();
        // 200x300 free: 11 columns -> 9 (max), 16 rows (300/18 = 16).
        assert_eq!(calculate_size(&c, Rect::new(0, 0, 200, 300), false), (162, 288));
        // Partial rows fill the height when fewer than max rows fit.
        assert_eq!(calculate_size(&c, Rect::new(0, 0, 200, 100), true), (162, 100));
        assert_eq!(calculate_size(&c, Rect::new(0, 0, 200, 100), false), (162, 90));
        // Too narrow: nothing.
        assert_eq!(calculate_size(&c, Rect::new(0, 0, 30, 300), false), (0, 0));
        // Right aligned.
        let b = calculate_bounds(&c, Rect::new(100, 10, 200, 300), false);
        assert_eq!((b.x, b.y, b.w, b.h), (138, 10, 162, 288));
        let b = calculate_bounds(&GridConfig::bookmark_list(), Rect::new(100, 10, 200, 300), false);
        assert_eq!(b.x, 100);
    }

    #[test]
    fn exclusions_block_slots() {
        let area = Rect::new(0, 0, 36, 18);
        assert_eq!(available_slot_count(area, &[]), 2);
        // An area touching only the second slot's 2px margin blocks it.
        assert_eq!(available_slot_count(area, &[Rect::new(37, 0, 10, 10)]), 1);
        assert_eq!(available_slot_count(area, &[Rect::new(40, 0, 10, 10)]), 2);
        let s = calculate_slots(area, &[], Some((20, 5)), 0);
        assert!(!s[0].blocked && s[1].blocked);
    }

    #[test]
    fn paging() {
        assert_eq!(first_item_index_for_valid_page(25, 100, 10), 20);
        assert_eq!(first_item_index_for_valid_page(500, 95, 10), 90);
        assert_eq!(first_item_index_for_valid_page(-5, 95, 10), 0);
        assert_eq!(first_item_index_for_valid_page(5, 0, 10), 0);
        assert_eq!((page_count(95, 10), page_count(0, 10), page_count(5, 0)), (10, 1, 1));
        assert_eq!(page_number(27, 10, 95), 2);
    }

    #[test]
    fn keyboard_navigation() {
        let occ = [true, true, false, true, true, true];
        assert_eq!(find_next_slot(&occ, 3, 0, 1, false), Some(1));
        assert_eq!(find_next_slot(&occ, 3, 1, 1, false), None); // slot 2 empty, next would cross a row
        assert_eq!(find_next_slot(&occ, 3, 0, 3, true), Some(3));
        assert_eq!(find_next_slot(&occ, 3, 2, 3, true), Some(5)); // 2 -> 5
    }
}
