use super::*;

/// macOS has no menu bar in the window to toggle, so it spends no key on one.
#[test]
fn toggle_menu_bar_has_a_default_shortcut_where_the_window_draws_the_bar() {
    assert!(macos::defaults(Command::ToggleMenuBar).is_empty());
    assert_eq!(linux::defaults(Command::ToggleMenuBar), ["ctrl-shift-m"]);
    assert_eq!(windows::defaults(Command::ToggleMenuBar), ["ctrl-shift-m"]);
    assert!(macos::defaults(Command::ToggleMenuBarTemporary).is_empty());
    assert_eq!(
        linux::defaults(Command::ToggleMenuBarTemporary),
        ["ctrl-alt-m"]
    );
    assert_eq!(
        windows::defaults(Command::ToggleMenuBarTemporary),
        ["ctrl-alt-m"]
    );
}
