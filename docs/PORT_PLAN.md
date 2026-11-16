# JEI on Horizon: port plan

JEI (Java, MC 26.3, MIT) is a Horizon **native mod** in `mods/jei/` (crate `mod-jei`, `horizon-mod.toml`), on the mod API
(`#[mod_api::horizon_mod] impl Hooks`). All logic that needs no game lives in `mods/jei/src/jei_core/` (a module of the one crate, as the native package
layout allows no second crate; `cargo test --lib` of the mod crate, 75 tests). The Java tree stays as the spec. The old `mixins/jei/` bundle is unused. Horizon helpers the mod needs are
added to the SCPLEGION/horizon fork (the `[native] horizon` pin points at it); nothing is opened on the official repo
without the user.

Faithfulness rules: port behaviour, strings and layout numbers exactly; no simplifications that diverge. Nothing here
has been run in game yet; everything is type-checked against the fork's `horizon-mod-api` and unit-tested in `jei-core`.

## Done

- **Search**: tokenizer, prefixes (`@ # $ % ^ &`), `|`, `-`, quotes, syntax highlight, history (Up/Down), completion
  candidates and dropdown (`searchCompletionEnabled`, `maxSearchCompletionRows`).
- **Ingredient list overlay**: grid layout from `overlay_layout` (border margins, nav row, buttons), paging, hover,
  tooltip (name, creative tooltip lines, mod), hide-mode (Ctrl+click), cheat mode (Shift+click on the config button),
  creative-tab stacks for tooltips and `%tab` search.
- **Recipe screen** (`recipe_gui`): panel, category tabs and tab paging, category and page arrows, title, grid with
  `maxRecipeGuiColumns`, recipe backgrounds, slot cycling, notes, transfer (+) button (left once, right max), bookmark
  button, crafting stations tab (click opens that item's recipes), option buttons tab (bookmarks first / craftable
  first), lookup history under the panel, wheel navigation, `navigationMode = SCROLLED`.
- **Textures**: the real JEI sprites ship as mod assets (`assets/jei/textures/gui/sprites`), with a fill fallback.
- **Categories**: crafting, smelting, smoking, blasting, campfire, stonecutting, smithing (transform and trim), fuel,
  compost, anvil; plugin API (`jei::api`: categories, catalysts, recipes) for other mods (info recipes come through it).
- **Bookmarks** (toggle with A, saved), history of lookups, `jei-client.toml` option table (`jei-core::config`) with the
  options screen on the config button.
- **Keys**: see Keybinds below; Tab and Enter in the search box.

## Open, from the audit against the Java source

Config options in `OPTIONS` that nothing reads yet (wire each, then delete from this list):
`toastReflowEnabled`, `compactInventoryEffects`, `bookmarkOutputAsRecipe`, `dragToRearrangeBookmarksEnabled`, `bookmarkTooltip*`, `guiResizeEnabled`,
`dragDelayInMilliseconds`, `smoothScrolling*`, `showTagRecipesEnabled`,
`enableRecipesGuiIngredientsSummary`, `tagContentTooltip*`, `lookupFluidContentsEnabled`,
`lookupBlockTagsEnabled`, lookup history `displaySide`,
`cheatToHotbarUsingHotkeysEnabled`, `recipeSyncWarningEnabled`.
Options of `ClientConfig` missing from the table: `searchBarPosition`, `ingredientSortStages`, `bookmarkAddPosition`
(`recipeGuiHeight`/`recipeGuiWidth` and `showHiddenIngredients` are done.)

Keybinds: the keyboard mappings of `InternalKeyMappings` are registered on Options > Controls (`jei-core::keys`) and
dispatched from the bound keys: `toggleOverlay`, `focusSearch`, `previousPage`/`nextPage`, `bookmark`, `showRecipe`,
`showUses`, `toggleCheatMode`, `toggleEditMode`, `recipeForward`, `pauseRecipeCycling` (held), `previousSearch`/`nextSearch` (search history), `recipeBack`, `previousRecipePage`/`nextRecipePage`, `previousCategory`/`nextCategory`,
`closeRecipeGui`. Still unbound: `toggleBookmarkOverlay`, the mouse mappings as keys (cheat give variants,
`quickMove`, `transferRecipeBookmark`, `maxTransferRecipeBookmark`, `toggleHideIngredient*`, `clearSearchBar`),
`copy.recipe.id`, `shareToChat`.

Screens and behaviour:
1. **Brewing and grindstone categories** are in (fork helpers `brewing_recipes`, `grindstone_recipes`), but potion
   variants share one item id, so the bottles of a brewing recipe are not told apart: recipes need to carry stacks.
3. **Fuel per furnace type** (smoking/blasting speeds) and the fuel and compost layouts drawn inside the layout
   (the text is under it now).
4. **Recipe bookmarks**: bookmarks store ingredients only; `bookmarkOutputAsRecipe` and the bookmark list's recipe
   entries, tooltip preview, drag to rearrange, transfer keys.
5. **Ghost ingredient drag** is in (`jei::api::register_ghost_target`: a screen id, the drop rectangles for an ingredient and an
   accept callback; JEI draws its list over that screen). Not ported: drag to rearrange bookmarks, dropping onto the
   ingredient list itself, and the exact fade of Java's highlight.
6. **Recipe screen**: GUI resize handles (`guiResizeEnabled`), `recipeGuiHeight/Width`, ingredient summary, focus
   highlight of the focused ingredient, the recipe-id copy and "recipe source" tooltips, scroll smoothing.
7. **Ingredient types beyond items**: fluids are an `api::IngredientType` (`fluid:` ids, drawn with `DrawContext::fluid`) and other
   mods register theirs (`jei::api::register_ingredient_type`: Mekanism chemicals later). Fluid amounts in recipe slots, fluid
   tank sizes and the fluid tooltip content (`lookupFluidContentsEnabled`) are not ported.
8. **Inventory integration**: shift to exclusion areas of the inventory effects, the recipe book and toasts
   (`toastReflowEnabled`, `compactInventoryEffects`), cheat give modes, `quickMove`.
9. **Sort stages**: the default stages (mod name, creative menu) are applied; the `ingredientSortStages` option and the
   other stages (type, alphabetical, tag, armor, durability) are not; `recipeSortingBookmarks`, `recipeSortingCraftable` (crafting only, one item per slot),
   `giveMode` and the tag tooltip (`Accepts any: #tag`) are done.
10. **Language**: the lang files ship as mod assets and category titles, notes and costs read them (`host.translate`); the
    overlay's own strings (search hint, options, tooltips) are still English literals.
11. **Real run**: needs the `mixins` pin from `cargo horizon-mod new` and a prebuilt game; then fix whatever the first
    in-game run shows (sprite names and atlas packing of mod assets are unverified).
