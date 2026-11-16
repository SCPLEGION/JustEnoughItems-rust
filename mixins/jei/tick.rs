// Every tick of every `Game` (hook point `native_tick`): JEI reads the game's items and recipes once there is a world.
fn on_tick(game: &mut crate::game::Game<'_>) {
    crate::game::jei::tick(game);
}
