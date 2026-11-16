# JEI on Horizon: port plan

JEI (Java, MC 26.3, MIT) is a Horizon **native mod** in `mods/jei/` (crate `mod-jei`, `horizon-mod.toml`), on the mod API
(`#[mod_api::horizon_mod] impl Hooks`). All logic that needs no game lives in `mods/jei/core` (`jei-core`, `cargo test`,
63 tests). The Java tree stays as the spec. The old `mixins/jei/` bundle is unused. Horizon helpers the mod needs are
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
- **Keys**: R, U, A, Backspace, Esc, Tab (search), Enter.

## Open, from the audit against the Java source

Config options in `OPTIONS` that nothing reads yet (wire each, then delete from this list):
`toastReflowEnabled`, `compactInventoryEffects`, `hideIngredientsWhenNotSearching`, `maxColumns`/`maxRows`
(grid size), `bookmarkOutputAsRecipe`, `dragToRearrangeBookmarksEnabled`, `bookmarkTooltip*`, `guiResizeEnabled`,
`dragDelayInMilliseconds`, `smoothScrolling*`, `showTagRecipesEnabled`,
`enableRecipesGuiIngredientsSummary`, `showCreativeTabNamesEnabled`, `tagContentTooltip*`, `lookupFluidContentsEnabled`,
`lookupBlockTagsEnabled`, lookup history `displaySide`, `giveMode`,
`cheatToHotbarUsingHotkeysEnabled`, `showHiddenIngredients`, `recipeSyncWarningEnabled`.
Options of `ClientConfig` missing from the table: `searchBarPosition`, `ingredientSortStages`, `bookmarkAddPosition`,
`recipeGuiHeight`, `recipeGuiWidth`.

Keybinds of the Java mod not bound (all `key.jei.*`): `toggleOverlay`, `toggleBookmarkOverlay`, `focusSearch`,
`clearSearchBar`, `previousSearch`/`nextSearch`, `previousPage`/`nextPage`, `previousCategory`/`nextCategory`,
`previousRecipePage`/`nextRecipePage`, `recipeBack`/`recipeForward`, `closeRecipeGui`, `bookmark` (A is bound),
`toggleCheatMode`, `toggleCheatModeConfigButton`, `toggleEditMode`, `toggleHideIngredient`,
`toggleWildcardHideIngredient`, `toggleHideIngredientsWhenNotSearching`, `cheatItemStack`, `cheatOneItem` (and the
`ToInventory` variants), `quickMove`, `transferRecipeBookmark`, `maxTransferRecipeBookmark`, `pauseRecipeCycling`,
`copy.recipe.id`, `shareToChat`. They need the keybinding list of the mod API (or fixed keys with a settings entry).

Screens and behaviour:
1. **Brewing category**: waits for a `brewing_recipes()` helper on the fork (requested).
2. **Grindstone category** (`plugins/vanilla/grindstone`): no helper; needs the grindstone rules (a host helper or a
   port of `GrindstoneRecipeMaker`).
3. **Fuel per furnace type** (smoking/blasting speeds) and the fuel and compost layouts drawn inside the layout
   (the text is under it now).
4. **Recipe bookmarks**: bookmarks store ingredients only; `bookmarkOutputAsRecipe` and the bookmark list's recipe
   entries, tooltip preview, drag to rearrange, transfer keys.
5. **Ghost ingredient drag** into mod screens: needs `screen_drop` and `screen_item_dropped_pre` on the fork
   (postponed until Mekanism resumes).
6. **Recipe screen**: GUI resize handles (`guiResizeEnabled`), `recipeGuiHeight/Width`, ingredient summary, focus
   highlight of the focused ingredient, the recipe-id copy and "recipe source" tooltips, scroll smoothing.
7. **Ingredient types beyond items**: fluids and other mods' types (needs the API's ingredient-type registry).
8. **Inventory integration**: shift to exclusion areas of the inventory effects, the recipe book and toasts
   (`toastReflowEnabled`, `compactInventoryEffects`), cheat give modes, `quickMove`.
9. **Sort stages** for the ingredient list (`ingredientSortStages`) and recipe sorting by bookmarks/craftable (the
   options toggle the setting; the list is not reordered yet).
10. **Language**: titles and strings are English; read the game's language files.
11. **Real run**: needs the `mixins` pin from `cargo horizon-mod new` and a prebuilt game; then fix whatever the first
    in-game run shows (sprite names and atlas packing of mod assets are unverified).
