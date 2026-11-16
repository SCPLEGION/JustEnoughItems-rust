// A container screen is drawn (hook point `native_container_draw`, API 87): the ingredient list and the recipe view.
fn on_container_draw(game: &crate::game::Game<'_>, window: [f32; 4], batch: &mut horizon_client::hud::HudBatch) {
    crate::game::jei::container_draw(game, window, batch);
}
