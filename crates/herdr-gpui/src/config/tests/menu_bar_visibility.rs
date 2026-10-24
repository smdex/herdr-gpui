#![allow(clippy::unwrap_used)]

use super::*;
use crate::config::preferences::Preference;

#[test]
fn menu_bar_visibility_defaults_visible_and_round_trips_saved_and_cli_edits() -> anyhow::Result<()>
{
    assert!(Config::parse("")?.show_menu_bar);
    let temp = TempDirectory::new()?;
    let local = temp.0.join("local.toml");
    let daemon = temp.0.join("daemon.toml");
    Config::save_preference_path(Preference::ShowMenuBar(false), &local)?;
    assert!(!Config::parse(&fs::read_to_string(&local)?)?.show_menu_bar);
    Config::edit_key_at(&local, &daemon, "show_menu_bar", Some(true.into()))?;
    assert!(Config::parse(&fs::read_to_string(&local)?)?.show_menu_bar);
    let before = fs::read_to_string(&local)?;
    assert!(Config::edit_key_at(&local, &daemon, "show_menu_bar", Some("no".into())).is_err());
    assert_eq!(fs::read_to_string(&local)?, before);
    Ok(())
}

#[cfg(any(target_os = "linux", windows))]
#[gpui::test]
fn saved_menu_visibility_updates_two_open_main_windows(cx: &mut gpui::TestAppContext) {
    use crate::sidebar::layout_tests::{fixture_window, full_draw};
    use gpui::VisualTestContext;
    let temp = TempDirectory::new().unwrap();
    let path = temp.0.join("local.toml");
    let (first, first_window) = cx.add_window_view(fixture_window);
    let first_window = first_window.update(|window, _| window.window_handle());
    let (second, second_window) = cx.add_window_view(fixture_window);
    let second_window = second_window.update(|window, _| window.window_handle());
    for shown in [false, true] {
        Config::save_preference_path(Preference::ShowMenuBar(shown), &path).unwrap();
        for view in [&first, &second] {
            let path = path.clone();
            view.update(cx, |view, cx| {
                view.load_gui_config_with(
                    move || {
                        Ok((
                            Config::parse(&fs::read_to_string(path)?)?,
                            Default::default(),
                        ))
                    },
                    cx,
                )
            });
        }
        cx.run_until_parked();
        for window in [first_window, second_window] {
            let mut visual = VisualTestContext::from_window(window, cx);
            visual.update(|window, cx| full_draw(window, cx).clear(cx));
            assert_eq!(visual.debug_bounds("application-menu-bar").is_some(), shown);
        }
        for view in [&first, &second] {
            view.read_with(cx, |view, _| assert_eq!(view.config.show_menu_bar, shown));
        }
    }
}
