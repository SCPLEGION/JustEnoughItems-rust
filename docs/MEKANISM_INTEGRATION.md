# JEI + Mekanism in one instance

`tools/duo.sh` (Mekanism-rust) already builds both native mods into one game. JEI works alone; Mekanism gets its
recipe categories, chemicals and filter drops by calling `jei::api` from its own hooks.

## Manifest (mods/mekanism/horizon-mod.toml)

```toml
[dependencies]
jei = { version = "^0.1", optional = true }
```

Do not touch `Cargo.toml`: the launcher adds the path dependency and a feature named `jei`.

## Calls (from `Hooks::register`)

```rust
mod_api::if_installed!(host, jei, {
    jei::api::register_ingredient_type(jei::api::IngredientType {
        prefix: "chemical:",                // ids are "chemical:mekanism:hydrogen"
        list: |host| /* every chemical id with the prefix */ vec![],
        draw: |ctx, id, at, size| { /* tinted sprite, like fluids */ },
        name: |host, id| String::new(),
    });
    jei::api::register_category(jei::api::CategoryDef::custom(
        "mekanism:crusher", "Crusher", "mekanism:crusher",
        jei::api::Layout { width: 116, height: 54, slots: vec![jei::api::view::input(0, 1, 1), jei::api::view::output(79, 1)], arrow: Some((52, 2)) },
    )).ok();
    jei::api::add_catalyst("mekanism:crusher", "mekanism:crusher").ok();
    jei::api::add_recipe(jei::api::CustomRecipe {
        recipe_type: "mekanism:crusher".into(),
        inputs: vec![vec!["minecraft:cobblestone".into()]],
        outputs: vec![vec!["minecraft:gravel".into()]],
        notes: vec![],
    }).ok();
});
```

Notes:
- Register categories and recipes once (every call raises `jei::api::version()`, the viewer rebuilds).
- A slot holds a list of alternatives (tags expand to the item list).
- Chemical (or fluid) amounts in slots and tank gauges are not drawn yet.
- Drops from the ingredient list onto a Mekanism filter screen: `jei::api::register_ghost_target(GhostTarget { screen: "mekanism:...", targets, accept })`.
- Layout numbers: take them from the Java category's `IGuiHelper` calls (`Gui/.../mekanism/.../*RecipeCategory.java`), x/y are relative to the layout's top left.
- The `jei` calls compile only when the dependency is installed, so Mekanism still builds and runs without JEI.
