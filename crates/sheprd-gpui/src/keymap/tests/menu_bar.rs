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
    assert_eq!(
        defaults.primary(Command::ToggleMenuBarTemporary),
        if cfg!(target_os = "macos") {
            ""
        } else {
            "ctrl-alt-m"
        }
    );
    let rebound = with(&[("toggle_menu_bar", one("ctrl-alt-shift-m"))]).unwrap();
    assert!(rebound.triggers(
        Command::ToggleMenuBar,
        &keystroke("ctrl-alt-shift-m"),
        false
    ));
    assert!(!rebound.triggers(Command::ToggleMenuBar, &keystroke("ctrl-shift-m"), false));
}
