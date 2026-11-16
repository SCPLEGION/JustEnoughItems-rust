# What the JEI bundle needs from Horizon

For the rewrite onto upstream's mod API: every hook point and helper `mixins/jei/runtime.rs` uses today (fork API 93), with
what it must do. Each is small and self-contained. The upstream equivalent is to be filled in by whoever knows that API.

## Hook points (`[[injection]]`, kind HEAD)

| Fork hook | When | Needs |
|---|---|---|
| `tick` | every game tick | load settings and the item/recipe data once a world is open |
| `container_draw` | after a vanilla container screen is drawn (inventory, creative, chests, stations, mod screens) | draw into the HUD batch over the screen, with the window rect |
| `container_click` | mouse press on such a screen, before the screen handles it | position, button; a way to claim the click |
| `container_release` | mouse release | same; used to open recipes and for ghost drag |
| `container_key` | key press/release with the typed text | key name, text, pressed; a way to claim; the Ctrl state is tracked from key events |
| `container_wheel` | scroll | position, steps; claim |

## Helpers (called on `Game`)

| Helper | Purpose |
|---|---|
| `native_fill`, `native_text`, `native_text_width` | draw rectangles and text in GUI pixels, measure text |
| `native_item`, `native_item_stack` | draw an item, or a stack with components |
| `native_cursor` | cursor position in GUI pixels |
| `native_hovered_stack` | the stack under the cursor in the open screen |
| `native_draw_item_tooltip` | the game's own item tooltip |
| `native_recipes` | all items (id, name, tags) and recipes: crafting, furnace types, campfire, stonecutting, smithing (trim results included), brewing |
| `native_anvil_recipes` | anvil repairs and enchanting with level costs, computed by the game |
| `native_creative_stacks` | every creative stack with components, display name, tooltip lines and creative tabs |
| `native_fuels(fast)` | burn time and speed of every fuel, for furnace or fast cooker |
| `native_compostables` | chance of a composter layer per item |
| `native_give` | give a stack (cheat mode, creative) |
| `native_place_recipe` | recipe-book transfer into the open crafting grid |
| `native_screen_at`, `native_screen_drop` | the mod screen under the cursor and dropping a stack into its ghost slot |
| `native_setting`, `native_set_setting` | per-mod persistent key/value store (bookmarks, hidden ingredients, options) |

Everything else (search, grid, recipe manager, bookmarks, plugin registry) is game-free in `mixins/jei/core`.
