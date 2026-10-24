use super::*;

#[test]
fn menu_bar_toggle_is_rebindable() {
    let defaults = with(&[]).unwrap();
    assert_eq!(
        defaults.primary(Command::ToggleMenuBar),
        if cfg!(target_os = "macos") {
            ""
        } else {
            "ctrl-shift-m"
        }
    );
    let rebound = with(&[("toggle_menu_bar", one("ctrl-alt-m"))]).unwrap();
    assert!(rebound.triggers(Command::ToggleMenuBar, &keystroke("ctrl-alt-m"), false));
    assert!(!rebound.triggers(Command::ToggleMenuBar, &keystroke("ctrl-shift-m"), false));
}
