use super::{Config, InitialAppearance, main_window_decorations};
use gpui::WindowDecorations;

#[test]
fn only_opted_in_wayland_main_windows_request_client_decorations() {
    for compositor in ["Wayland", "X11", "Windows", "Mac", "Test"] {
        assert_eq!(main_window_decorations(false, compositor), None);
        assert_eq!(
            main_window_decorations(true, compositor),
            if compositor == "Wayland" {
                Some(WindowDecorations::Client)
            } else {
                None
            },
        );
    }
}

#[test]
fn reopening_uses_the_loaded_preference_without_changing_existing_options() {
    let initial = InitialAppearance::load(|| Ok(Config::default()), false);
    let existing =
        main_window_decorations(initial.config.request_client_side_decorations, "Wayland");
    let reopened = InitialAppearance::load(
        || {
            Ok(Config {
                request_client_side_decorations: true,
                ..Default::default()
            })
        },
        false,
    );
    assert!(reopened.error.is_none());
    assert_eq!(existing, None);
    assert_eq!(
        main_window_decorations(reopened.config.request_client_side_decorations, "Wayland"),
        Some(WindowDecorations::Client)
    );
}
