# JEI on upstream Horizon (horizon-rust/horizon)

A native mod on the official mod API (`horizon-mod.toml`, `src/lib.rs`, `impl mod_api::Hooks`). `core/` is a copy of
`mixins/jei/core` (a native mod may not reach outside its folder); the old fork bundle in `mixins/jei` stays until this
one has everything.

Done: the ingredient list over the inventory and creative screens (registry items, search with JEI's tokenizer, prefixes
and modes, paging by click or wheel), a search box that takes typing, cheat give.

Not yet: recipes (`recipes_of_type`), bookmarks, options, tooltips, hide mode, categories. Gaps in upstream's API that the
fork covered with its own helpers: creative stacks with components and tabs, the hovered stack with components, a ghost
slot drop into mod screens, the anvil recipe table, fuel speed and fast-cooker fuels.

Type-check without a game: point `horizon-mod-api` in `Cargo.toml` at a clone of upstream's `crates/horizon-mod-api`
and run `cargo check --lib`.
