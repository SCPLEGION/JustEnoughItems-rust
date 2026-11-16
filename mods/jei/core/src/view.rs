//! Layouts of the vanilla recipe categories JEI draws (`CraftingRecipeCategory`, `AbstractCookingCategory`,
//! `StoneCuttingRecipeCategory`, `SmithingRecipeCategory`, `BrewingRecipeCategory`): the category's size and where its
//! slots, arrow and texts go, in the recipe's own pixels. The glue draws them with the game's item and sprite calls.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Crafting,
    Smelting,
    Blasting,
    Smoking,
    Campfire,
    Stonecutting,
    Smithing,
    Brewing,
}

impl Kind {
    /// `IRecipeType` uid of the vanilla category.
    pub fn recipe_type(self) -> &'static str {
        match self {
            Kind::Crafting => "minecraft:crafting",
            Kind::Smelting => "minecraft:smelting",
            Kind::Blasting => "minecraft:blasting",
            Kind::Smoking => "minecraft:smoking",
            Kind::Campfire => "minecraft:campfire_cooking",
            Kind::Stonecutting => "minecraft:stonecutting",
            Kind::Smithing => "minecraft:smithing",
            Kind::Brewing => "jei:brewing",
        }
    }
    /// The category's title key (`getTitle`).
    pub fn title_key(self) -> &'static str {
        match self {
            Kind::Crafting => "gui.jei.category.craftingTable",
            Kind::Smelting => "gui.jei.category.smelting",
            Kind::Blasting => "gui.jei.category.blasting",
            Kind::Smoking => "gui.jei.category.smoking",
            Kind::Campfire => "gui.jei.category.campfire",
            Kind::Stonecutting => "gui.jei.category.stoneCutter",
            Kind::Smithing => "gui.jei.category.smithing",
            Kind::Brewing => "gui.jei.category.brewing",
        }
    }
    /// English text of `title_key` (the lang file's, until the game's language is read).
    pub fn title_en(self) -> &'static str {
        match self {
            Kind::Crafting => "Crafting",
            Kind::Smelting => "Smelting",
            Kind::Blasting => "Blasting",
            Kind::Smoking => "Smoking",
            Kind::Campfire => "Campfire Cooking",
            Kind::Stonecutting => "Stonecutting",
            Kind::Smithing => "Smithing",
            Kind::Brewing => "Brewing",
        }
    }
    pub const ALL: [Kind; 8] = [
        Kind::Crafting,
        Kind::Smelting,
        Kind::Blasting,
        Kind::Smoking,
        Kind::Campfire,
        Kind::Stonecutting,
        Kind::Smithing,
        Kind::Brewing,
    ];
    /// The category's tab icon (`createDrawableItemLike`), also its crafting station.
    pub fn icon(self) -> &'static str {
        match self {
            Kind::Crafting => "minecraft:crafting_table",
            Kind::Smelting => "minecraft:furnace",
            Kind::Blasting => "minecraft:blast_furnace",
            Kind::Smoking => "minecraft:smoker",
            Kind::Campfire => "minecraft:campfire",
            Kind::Stonecutting => "minecraft:stonecutter",
            Kind::Smithing => "minecraft:smithing_table",
            Kind::Brewing => "minecraft:brewing_stand",
        }
    }
}

/// A slot's role in the layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotRole {
    Input,
    Output,
    /// The fuel slot of a furnace (shown, not a focus).
    Fuel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotPos {
    pub role: SlotRole,
    /// Index into the recipe's input slots (`Input`) or output slots (`Output`), `usize::MAX` for the fuel.
    pub input: usize,
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    pub width: i32,
    pub height: i32,
    pub slots: Vec<SlotPos>,
    /// The progress arrow's position, when the category has one.
    pub arrow: Option<(i32, i32)>,
}

pub fn input(i: usize, x: i32, y: i32) -> SlotPos {
    SlotPos { role: SlotRole::Input, input: i, x, y }
}
pub fn output(x: i32, y: i32) -> SlotPos {
    SlotPos { role: SlotRole::Output, input: 0, x, y }
}
fn fuel(x: i32, y: i32) -> SlotPos {
    SlotPos { role: SlotRole::Fuel, input: usize::MAX, x, y }
}

/// The layout of a recipe. `grid` is a crafting recipe's `(width, height)`: shaped recipes keep their grid, shapeless
/// ones fill it row by row (`CraftingGridHelper`: a 1x1 recipe sits in the middle, 1 wide in the middle column...).
pub fn layout(kind: Kind, grid: (i32, i32), inputs: usize) -> Layout {
    match kind {
        Kind::Crafting => {
            let (gw, gh) = (grid.0.clamp(1, 3), grid.1.clamp(1, 3));
            let slots = if inputs == 1 && gw == 1 && gh == 1 {
                // `CraftingGridHelper.getInputSlotIndex`: a single ingredient goes in the centre.
                vec![input(0, 1 + 18, 1 + 18), output(95, 19)]
            } else {
                let mut v: Vec<SlotPos> = (0..inputs)
                    .map(|i| {
                        let (col, row) = (i as i32 % gw, i as i32 / gw);
                        input(i, 1 + col * 18, 1 + row * 18)
                    })
                    .collect();
                v.push(output(95, 19));
                v
            };
            Layout { width: 116, height: 54, slots, arrow: Some((61, 19)) }
        }
        Kind::Smelting | Kind::Blasting | Kind::Smoking => Layout {
            width: 82,
            height: 54,
            slots: vec![input(0, 1, 1), fuel(1, 37), output(61, 19)],
            arrow: Some((26, 17)),
        },
        Kind::Campfire => Layout { width: 82, height: 34, slots: vec![input(0, 1, 1), output(61, 9)], arrow: Some((26, 9)) },
        Kind::Stonecutting => Layout { width: 82, height: 34, slots: vec![input(0, 1, 9), output(61, 9)], arrow: Some((26, 9)) },
        Kind::Smithing => Layout {
            width: 108,
            height: 28,
            slots: vec![input(0, 1, 6), input(1, 19, 6), input(2, 37, 6), output(91, 6)],
            arrow: Some((62, 6)),
        },
        Kind::Brewing => Layout {
            width: 114,
            height: 61,
            slots: vec![input(0, 1, 37), input(1, 24, 44), input(2, 47, 37), input(3, 24, 3), output(81, 3)],
            arrow: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crafting_grid() {
        let l = layout(Kind::Crafting, (3, 2), 5);
        assert_eq!((l.width, l.height), (116, 54));
        assert_eq!(l.slots[3], input(3, 1, 19));
        assert_eq!(l.slots.last().copied(), Some(output(95, 19)));
        let one = layout(Kind::Crafting, (1, 1), 1);
        assert_eq!(one.slots[0], input(0, 19, 19));
    }

    #[test]
    fn furnace_has_fuel() {
        let l = layout(Kind::Smelting, (0, 0), 1);
        assert!(l.slots.iter().any(|s| s.role == SlotRole::Fuel));
        assert_eq!(layout(Kind::Stonecutting, (0, 0), 1).height, 34);
    }
}
