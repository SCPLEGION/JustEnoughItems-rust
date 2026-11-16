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
