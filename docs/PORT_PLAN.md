# JEI on Horizon: port plan

JEI (Java, MC 26.3, MIT) becomes a Horizon **native mod**: a horizon-mixins bundle in `mixins/jei/` plus a game-free
crate `jei-core` (all logic, unit-tested alone), the same shape as Mekanism-rust. The Java tree stays as the spec.
Horizon already has a basic item panel (API 2: `native_panel_items`, `native_panel_recipes`, `native_panel_event`,
`native_recipes`, `native_screen_drop`); it draws fixed recipe layouts (<= 3x3), so a faithful JEI needs more generic hooks.
Horizon changes stay generic (never JEI-specific) and go to SCPLEGION/horizon as their own PRs.

## Milestones (one commit group each, in order; parallel where independent)

| # | Milestone | Where | Java source |
|---|---|---|---|
| J0 | Scaffold: workspace, bundle, `jei-core` | this repo | n/a |
| J1 | Search: tokenizer, prefix table (done), query parse (`\|`, `-`, quotes), index + matching, sort order | core | `Gui/.../search`, `Common/.../search` |
| J2 | Ingredient model: ingredient types, registry, visibility/blacklist, list elements, mod names, tags, tooltips, colours | core | `Library/.../ingredients`, `Gui/.../ingredients` |
| J3 | Plugin API in Rust: categories, recipe registration, recipe maps/lookups (focus, input/output/catalyst), vanilla plugin data (crafting, smelting, stonecutting, smithing, anvil, brewing, fuel, info) | core | `Common/src/api`, `Library/.../recipes`, `plugins/vanilla` |
| H1 | Horizon: container overlay hooks (draw/click/key/typing/wheel over any container screen; screen rect + exclusion areas; arbitrary ingredient render + tooltip) | horizon | n/a |
| J4 | Ingredient list overlay: grid, paging, search box, config button, hover, click to open recipes, cheat-give | bundle + core | `Gui/.../overlay` |
| J5 | Recipe GUI: category tabs, pages, layouts drawn by categories (slots, drawables, arrows, flames, progress), focus history, catalysts, recipe transfer (+) | bundle + core | `Gui/.../recipes`, `Library/.../gui` |
| J6 | Bookmarks, config (`jei` options), keybinds (R, U, A, O, ...), ghost-ingredient drag into filter slots | bundle + core | `Gui/.../bookmarks`, `config`, `input` |
| J7 | Plugin registration for other native mods + Mekanism-rust plugin (its recipe categories) | core + Mekanism-rust | `IModPlugin` |

Faithfulness rules: port behaviour, strings (lang keys) and layout numbers exactly; no simplifications that diverge.
Search storage uses plain in-memory substring indexes instead of Lucene (same results: literal, case-sensitive substring
on JEI's already lowercased keys).

## Status (updated after the first working version)

Done: J0 scaffold, J1 search, J2 ingredient strings, J3 recipe manager, H1 container overlay hooks (Horizon API 87),
game tooltip (88), release hook for ghost drag (89), anvil recipe data (90), ingredient grid layout, vanilla category layouts
(crafting, furnace types, campfire, stonecutting, smithing, brewing, anvil), recipe view with tabs/paging/history,
R/U/A/Backspace keys, bookmarks (saved), cheat mode, options (search modes, grid size), recipe transfer (+), plugin
registry for other mods, ghost-ingredient drag into mod screens, ingredient variants (creative-tab stacks, Horizon API 91/92),
smelting/smoking/blasting fuel and composting categories (API 93; text is drawn under the layout, not inside it yet).

Not tested in game yet (no Minecraft resources in the build container).

## Audit against the Java source: gaps still open

1. **Smithing** trim/transform extensions and **brewing** potion recipes use components; the native recipe data carries them
   only for stock recipes.
2. **Info category** (`IngredientInfoRecipe`, used by other mods' plugins): add a generic text category to the plugin registry.
3. **Hide ingredients / edit mode** (`IngredientBlacklistInternal`, `jei-blacklist`) and the **config file** (`jei-client.toml`
   with the full option set). The options screen covers search modes and grid size only.
4. **Search completion overlay**, **syntax highlighting** and **history list** (the bottom history of recently opened
   ingredients; recipe-view history is done).
5. **Recipe view fidelity**: JEI's background sprites, animated arrow/flame/bubbles, slot backgrounds, focus highlight of the
   focused ingredient, recipe category sorting config, "recipe bookmarks", catalysts column.
6. **Mod names / aliases**: real mod display names (now derived from the namespace) and `modAliases`.
7. **Language**: category titles are English; read the game's language.
8. **Mekanism plugin**: Mekanism-rust is paused; its recipe categories would call `jei::register_category` / `add_recipe`.
