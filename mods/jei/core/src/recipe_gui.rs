//! Port of the geometry of `RecipesGui`, `RecipeGuiSizing`, `RecipeGuiGrid`, `RecipeGuiTabs` and the recipe layout's
//! side buttons: where the panel, the category and page arrows, the title, the tabs and each recipe go. Paged mode
//! only (`recipeGuiNavigationMode = PAGED`, the default).
use crate::grid::Rect;

pub const BORDER_PADDING: i32 = 6;
pub const NAV_BAR_PADDING: i32 = 2;
pub const TITLE_INNER_PADDING: i32 = 14;
pub const SMALL_BUTTON: i32 = 13;
pub const MIN_GUI_WIDTH: i32 = 198;
pub const MIN_GUI_HEIGHT: i32 = 175;
pub const DEFAULT_GUI_HEIGHT: i32 = 350;
/// `font.lineHeight`.
pub const FONT_HEIGHT: i32 = 9;
pub const TAB_WIDTH: i32 = 24;
pub const TAB_HEIGHT: i32 = 24;
const TAB_GUI_OVERLAP: i32 = 3;
const TAB_HORIZONTAL_INSET: i32 = 2;
const TAB_NAVIGATION_HEIGHT: i32 = 20;
/// `RecipeManager.createRecipeLayoutDrawable`: the border of a category that `needsRecipeBorder`.
pub const RECIPE_BORDER: i32 = 4;
pub const RECIPE_BUTTON_SPACING: i32 = 2;
pub const RECIPE_BUTTON_SIZE: i32 = 13;
const MIN_RECIPE_PADDING: i32 = 4;

/// `RecipeGuiSizing.calculateInitialSize`: the height of the panel and the space left above it.
pub fn initial_size(screen_height: i32, search_bar_centered: bool, max_height: i32) -> (i32, i32) {
    let mut y = screen_height - if search_bar_centered { 76 } else { 58 };
    if y < MIN_GUI_HEIGHT {
        y = MIN_GUI_HEIGHT;
    }
    if y > max_height {
        (max_height, y - max_height)
    } else {
        (y, 0)
    }
}

/// `RecipesGui.updateSize`: the ideal area of the panel. `gui_width` is `recipeGuiWidth`, `left_extra` the width of the
/// crafting stations column.
pub fn ideal_area(screen_w: i32, screen_h: i32, gui_width: i32, max_height: i32, left_extra: i32) -> Rect {
    let max_width = MIN_GUI_WIDTH.max(screen_w - 2 * BORDER_PADDING.max(left_extra));
    let x_size = gui_width.clamp(MIN_GUI_WIDTH, max_width.max(MIN_GUI_WIDTH));
    let (y_size, extra) = initial_size(screen_h, false, max_height);
    Rect::new((screen_w - x_size) / 2, TAB_HEIGHT + 21 + extra / 2, x_size, y_size)
}

/// `calculateAreaToFitLayouts`: the panel widens (centred) to hold the recipes.
pub fn area_to_fit(ideal: Rect, screen_w: i32, recipes_width: i32) -> Rect {
    if recipes_width == 0 {
        return ideal;
    }
    let padding = 2 * BORDER_PADDING;
    let width = recipes_width.max(ideal.w - padding);
    let new_w = width + padding;
    Rect::new((screen_w - new_w) / 2, ideal.y, new_w, ideal.h)
}

/// The arrows, the title and the room left for recipes (`updateNavigation`, `updateLayout`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Navigation {
    pub previous_category: Rect,
    pub next_category: Rect,
    pub previous_page: Rect,
    pub next_page: Rect,
    pub header_height: i32,
    /// Between the category arrows, less `titleInnerPadding` on each side.
    pub title: Rect,
    /// Between the page arrows: where "1/3" is centred.
    pub page_area: Rect,
}

pub fn navigation(ideal: Rect) -> Navigation {
    let right_x = ideal.x + ideal.w - BORDER_PADDING - SMALL_BUTTON;
    let left_x = ideal.x + BORDER_PADDING;
    let title_height = FONT_HEIGHT + BORDER_PADDING;
    let category_top = ideal.y + title_height - SMALL_BUTTON + NAV_BAR_PADDING;
    let page_top = category_top + SMALL_BUTTON + NAV_BAR_PADDING;
    let b = |x, y| Rect::new(x, y, SMALL_BUTTON, SMALL_BUTTON);
    let (pc, nc, pp, np) = (b(left_x, category_top), b(right_x, category_top), b(left_x, page_top), b(right_x, page_top));
    let union = Rect::new(pc.x, pc.y, nc.x + nc.w - pc.x, nc.h);
    let title = Rect::new(union.x + SMALL_BUTTON + TITLE_INNER_PADDING, union.y, union.w - 2 * (SMALL_BUTTON + TITLE_INNER_PADDING), union.h);
    let page_area = Rect::new(pp.x, pp.y, np.x + np.w - pp.x, np.h);
    Navigation { previous_category: pc, next_category: nc, previous_page: pp, next_page: np, header_height: page_top + SMALL_BUTTON - ideal.y, title, page_area }
}

/// `getRecipeLayoutsArea`.
pub fn layouts_area(area: Rect, header_height: i32) -> Rect {
    Rect::new(area.x + BORDER_PADDING, area.y + header_height + NAV_BAR_PADDING, area.w - 2 * BORDER_PADDING, area.h - (header_height + BORDER_PADDING + NAV_BAR_PADDING))
}

/// `RecipeGuiGrid`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecipeGrid {
    pub rows: i32,
    pub columns: i32,
}

impl RecipeGrid {
    pub fn calculate(available: (i32, i32), recipe_with_buttons: (i32, i32), max_columns: i32) -> Self {
        let rows = ((available.1 + MIN_RECIPE_PADDING) / (recipe_with_buttons.1 + MIN_RECIPE_PADDING)).max(1);
        let columns = ((available.0 + MIN_RECIPE_PADDING) / (recipe_with_buttons.0 + MIN_RECIPE_PADDING)).clamp(1, max_columns.max(1));
        Self { rows, columns }
    }
    pub fn recipes_per_page(self) -> i32 {
        self.rows * self.columns
    }
    /// The cell of recipe `index` of the page: `recipe_size` is the layout with its border, `width_with_buttons` its width
    /// with the side buttons.
    pub fn recipe_area(self, index: i32, available: Rect, recipe_size: (i32, i32), width_with_buttons: i32) -> Rect {
        let column_width = (available.w - (self.columns - 1) * MIN_RECIPE_PADDING) / self.columns;
        let (column, row) = (index % self.columns, index / self.columns);
        let column_x = available.x + column * (column_width + MIN_RECIPE_PADDING);
        let button_space = width_with_buttons - recipe_size.0;
        let recipe_x = if column_width > recipe_size.0 + 2 * button_space {
            column_x + (column_width - recipe_size.0) / 2
        } else {
            column_x + (column_width - width_with_buttons) / 2
        };
        let available_height = available.h.max(recipe_size.1);
        let spacing = (available_height - self.rows * recipe_size.1) / (self.rows + 1);
        Rect::new(recipe_x, available.y + spacing + row * (recipe_size.1 + spacing), width_with_buttons, recipe_size.1)
    }
}

/// The recipe tabs above the panel (`RecipeGuiTabs.initLayout`): per page, the area of the tabs and the page arrows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tabs {
    pub per_page: usize,
    pub page_count: usize,
    pub page: usize,
    /// The tab rectangles of the visible page.
    pub rects: Vec<Rect>,
    /// The navigation row above the tabs (`PageNavigation`).
    pub navigation: Rect,
}

pub fn tabs(ideal: Rect, categories: usize, selected: usize) -> Option<Tabs> {
    if categories == 0 {
        return None;
    }
    let area = Rect::new(ideal.x + TAB_HORIZONTAL_INSET, ideal.y - (TAB_HEIGHT - TAB_GUI_OVERLAP), ideal.w - 2 * TAB_HORIZONTAL_INSET, TAB_HEIGHT);
    let per_page = ((area.w / TAB_WIDTH).max(1) as usize).min(categories);
    let page_count = categories.div_ceil(per_page);
    let page = selected / per_page;
    let rects = (0..per_page).filter(|i| page * per_page + i < categories).map(|i| Rect::new(area.x + i as i32 * TAB_WIDTH, area.y, TAB_WIDTH, TAB_HEIGHT)).collect();
    let navigation = Rect::new(area.x, area.y - (2 + TAB_NAVIGATION_HEIGHT), area.w, TAB_NAVIGATION_HEIGHT);
    Some(Tabs { per_page, page_count, page, rects, navigation })
}

/// `RecipeLayout.getSideButtonArea`, relative to the recipe's own rectangle (without its border): the transfer button is
/// button 0; the next ones stack upwards, then to the right.
pub fn side_button(category_w: i32, category_h: i32, border: i32, index: i32) -> Rect {
    let mut b = Rect::new(category_w + border + RECIPE_BUTTON_SPACING, category_h + border - RECIPE_BUTTON_SIZE, RECIPE_BUTTON_SIZE, RECIPE_BUTTON_SIZE);
    if index > 0 {
        let with_border_h = category_h + 2 * border;
        let max_rows = ((with_border_h + RECIPE_BUTTON_SPACING) / (RECIPE_BUTTON_SIZE + RECIPE_BUTTON_SPACING)).max(1);
        b.x += (index / max_rows) * (RECIPE_BUTTON_SIZE + RECIPE_BUTTON_SPACING);
        b.y -= (index % max_rows) * (RECIPE_BUTTON_SIZE + RECIPE_BUTTON_SPACING);
    }
    b
}

/// `RecipeLayoutWithButtons.totalWidth`: the left border plus the right area, which holds the border and the `buttons`
/// visible side buttons.
pub fn total_width(category_w: i32, category_h: i32, border: i32, buttons: i32) -> i32 {
    let mut right = category_w + border;
    for i in 0..buttons {
        let b = side_button(category_w, category_h, border, i);
        right = right.max(b.x + b.w);
    }
    border + right
}

/// `RecipeGuiLogic.getPageString`.
pub fn page_string(recipe_index: i32, per_page: i32, recipe_count: i32) -> String {
    let per = per_page.max(1);
    let page = (recipe_index + 1 + per - 1) / per;
    let count = (recipe_count + per - 1) / per;
    format!("{}/{}", page, count.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizing() {
        assert_eq!(initial_size(480, false, 350), (350, 72));
        assert_eq!(initial_size(200, false, 350), (175, 0));
        assert_eq!(initial_size(300, true, 350), (224, 0));
    }

    #[test]
    fn ideal_and_navigation() {
        let a = ideal_area(854, 480, 198, 350, 0);
        assert_eq!(a, Rect::new(328, 24 + 21 + 36, 198, 350));
        let n = navigation(a);
        assert_eq!(n.previous_category, Rect::new(a.x + 6, a.y + 15 - 13 + 2, 13, 13));
        assert_eq!(n.next_category.x, a.x + a.w - 6 - 13);
        assert_eq!(n.previous_page.y, n.previous_category.y + 13 + 2);
        assert_eq!(n.header_height, n.previous_page.y + 13 - a.y);
        assert_eq!(n.title.w, 198 - 12 - 2 * (13 + 14));
    }

    #[test]
    fn grid_and_cells() {
        let g = RecipeGrid::calculate((186, 300), (126, 44), 2);
        assert_eq!(g, RecipeGrid { rows: 6, columns: 1 });
        let g = RecipeGrid::calculate((400, 100), (126, 44), 2);
        assert_eq!(g, RecipeGrid { rows: 2, columns: 2 });
        assert_eq!(g.recipes_per_page(), 4);
        let avail = Rect::new(10, 20, 400, 100);
        let c = g.recipe_area(3, avail, (116, 44), 131);
        assert_eq!(c.y, 20 + (100 - 88) / 3 + (44 + 4));
        assert_eq!(c.h, 44);
    }

    #[test]
    fn widens_to_fit() {
        let ideal = Rect::new(328, 60, 198, 350);
        assert_eq!(area_to_fit(ideal, 854, 0), ideal);
        let a = area_to_fit(ideal, 854, 300);
        assert_eq!((a.w, a.x), (312, (854 - 312) / 2));
    }

    #[test]
    fn tabs_page() {
        let t = tabs(Rect::new(328, 60, 198, 350), 12, 9).unwrap();
        assert_eq!(t.per_page, 8);
        assert_eq!((t.page, t.page_count), (1, 2));
        assert_eq!(t.rects.len(), 4);
        assert_eq!(t.rects[0], Rect::new(330, 60 - 21, 24, 24));
        let one = tabs(Rect::new(328, 60, 198, 350), 3, 0).unwrap();
        assert_eq!(one.rects.len(), 3);
    }

    #[test]
    fn side_buttons() {
        let b0 = side_button(116, 54, 4, 0);
        assert_eq!(b0, Rect::new(122, 45, 13, 13));
        let b1 = side_button(116, 54, 4, 1);
        assert_eq!((b1.x, b1.y), (122, 45 - 15));
        assert_eq!(total_width(116, 54, 4, 1), 4 + 135);
        assert_eq!(total_width(116, 54, 4, 0), 4 + 120);
    }

    #[test]
    fn page_strings() {
        assert_eq!(page_string(0, 2, 5), "1/3");
        assert_eq!(page_string(4, 2, 5), "3/3");
        assert_eq!(page_string(0, 1, 0), "1/1");
    }
}
