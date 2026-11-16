// A mouse release on a container screen (hook point `native_container_release`, API 89): ends a drag from the list.
fn on_container_release(game: &mut crate::game::Game<'_>, window: [f32; 4], at: [f32; 2], button: u8, claimed: &mut bool) {
    crate::game::jei::container_release(game, window, at, button, claimed);
}
