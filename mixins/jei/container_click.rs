// A mouse press on a container screen (hook point `native_container_click`, API 87).
fn on_container_click(game: &mut crate::game::Game<'_>, window: [f32; 4], at: [f32; 2], button: u8, claimed: &mut bool) {
    crate::game::jei::container_click(game, window, at, button, claimed);
}
