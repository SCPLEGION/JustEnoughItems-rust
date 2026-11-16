// The wheel on a container screen (hook point `native_container_wheel`, API 87): pages the ingredient list.
fn on_container_wheel(game: &mut crate::game::Game<'_>, at: [f32; 2], steps: f32, claimed: &mut bool) {
    crate::game::jei::container_wheel(game, at, steps, claimed);
}
