// A key on a container screen (hook point `native_container_key`, API 87): the search box, R and U.
fn on_container_key(game: &mut crate::game::Game<'_>, key: &str, text: Option<&str>, pressed: bool, claimed: &mut bool) {
    crate::game::jei::container_key(game, key, text, pressed, claimed);
}
