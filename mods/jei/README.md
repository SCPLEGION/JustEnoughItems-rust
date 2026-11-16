# JEI on upstream Horizon (horizon-rust/horizon)

A native mod on the official mod API (`horizon-mod.toml`, `src/lib.rs`, `impl mod_api::Hooks`). `core/` is a copy of
`mixins/jei/core` (a native mod may not reach outside its folder); the old fork bundle in `mixins/jei` stays until this
one has everything.

Done: the ingredient list over the inventory and creative screens (registry items, search with JEI's tokenizer, prefixes
and modes, paging by click or wheel), a search box that takes typing, cheat give.

Recipes: crafting (shaped by the recipe file's pattern), cooking, stonecutting and smithing from `recipes_of_type`, with a view (tabs, paging, R/U keys, click an ingredient, Backspace history) and a Cheat button.

Also: lookup history (strip of recent items under the recipe view, saved; `lookups.maxIngredients`), search completion (Options toggle, Tab accepts; `search.searchCompletionEnabled`, kept as `jei-client.toml` text in the mod's settings), bookmarks (A key, saved), options (search modes, grid size, hide mode: Ctrl+click hides), search syntax colours, smelting fuel and composting categories.

Not yet: brewing, anvil, info, smoking and blasting fuel (needs fuel speed), variants (needs the creative stacks). Tooltips show name and mod only (the game's full tooltip needs the hovered stack with components). Gaps in upstream's API that the
fork covered with its own helpers: creative stacks with components and tabs, the hovered stack with components, a ghost
slot drop into mod screens, the anvil recipe table, fuel speed and fast-cooker fuels.

Type-check without a game: point `horizon-mod-api` in `Cargo.toml` at a clone of upstream's `crates/horizon-mod-api`
and run `cargo check --lib`.

Pin: `[native] horizon` names upstream main at the time of writing. The launcher also wants `mixins` (a horizon-mixins
commit) for a build; it is not set here, so the mod has not been built by the launcher. Only `cargo check --lib` against
upstream's `horizon-mod-api` has run.

## Plugin API (other mods)

Another mod lists `jei` under `[dependencies]` in its `horizon-mod.toml` and calls `jei::api::register_category`,
`add_catalyst` and `add_recipe` (with `jei::api::{CategoryDef, CustomRecipe, Layout, view}`) from its own hooks, for
example in `client_tick_post`. The viewer rebuilds its recipes when the registry changes. Guard calls with
`mod_api::if_installed!(host, jei, { .. })` when JEI is optional.

Tests: `cargo test --lib` in a copy with the SDK path pointed at upstream's `horizon-mod-api` (9 tests: plugin
registry, search modes, hide mode, completion, history).
