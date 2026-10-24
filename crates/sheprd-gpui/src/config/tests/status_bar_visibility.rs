#![allow(clippy::unwrap_used)]

use super::*;

#[test]
fn button_visibility_overrides_are_known_to_the_cli_without_destroying_style() -> anyhow::Result<()>
{
    let temp = TempDirectory::new()?;
    let path = temp.0.join("local.toml");
    let daemon = temp.0.join("daemon.toml");
    fs::write(
        &path,
        "[status_bar]\ntheme = 'icon'\nshortcuts = 'hidden'\n",
    )?;
    Config::edit_key_at(&path, &daemon, "status_bar.show_theme", Some(false.into()))?;
    let hidden = Config::parse(&fs::read_to_string(&path)?)?;
    assert_eq!(
        hidden.status_bar.theme,
        crate::config::status_bar::Button::Icon
    );
    Config::edit_key_at(&path, &daemon, "status_bar.show_theme", Some(true.into()))?;
    let restored = Config::parse(&fs::read_to_string(&path)?)?;
    assert_eq!(
        restored.status_bar.theme,
        crate::config::status_bar::Button::Icon
    );
    assert!(restored.unknown_keys.is_empty());
    assert_eq!(
        hidden
            .status_bar
            .theme
            .with_visibility(hidden.status_bar.show_theme),
        crate::config::status_bar::Button::Hidden
    );
    assert_eq!(
        restored
            .status_bar
            .theme
            .with_visibility(restored.status_bar.show_theme),
        crate::config::status_bar::Button::Icon
    );
    assert!(
        !restored
            .status_bar
            .shortcuts
            .with_visibility(restored.status_bar.show_shortcuts)
            .shown()
    );
    Config::edit_key_at(
        &path,
        &daemon,
        "status_bar.show_shortcuts",
        Some(true.into()),
    )?;
    let legacy = Config::parse(&fs::read_to_string(&path)?)?;
    assert_eq!(
        legacy
            .status_bar
            .shortcuts
            .with_visibility(legacy.status_bar.show_shortcuts),
        crate::config::status_bar::Button::Label
    );
    let before = fs::read_to_string(&path)?;
    assert!(
        Config::edit_key_at(&path, &daemon, "status_bar.show_theme", Some("no".into())).is_err()
    );
    assert_eq!(fs::read_to_string(&path)?, before);
    Ok(())
}

/// The actual right-click and checkbox callbacks save through the CLI's file
/// editor; only the filesystem location/load is replaced with a private file.
#[gpui::test]
fn status_bar_checkboxes_round_trip_every_setting(cx: &mut gpui::TestAppContext) {
    use crate::{
        menu::StatusBarIo,
        sidebar::layout_tests::{fixture_window, full_draw},
    };
    use gpui::{Modifiers, MouseButton};
    let temp = TempDirectory::new().unwrap();
    let local = temp.0.join("local.toml");
    fs::write(&local, "show_network_quality = true\n[status_bar]\ntheme = 'icon'\nshortcuts = 'icon'\nreport_issue = 'icon'\n").unwrap();
    let path = local.clone();
    cx.update(|cx| {
        cx.set_global(StatusBarIo(std::sync::Arc::new(move |key, shown| {
            Config::edit_key_at(
                &path,
                &path.with_extension("absent"),
                key,
                Some(shown.into()),
            )?;
            Ok((
                Config::parse(&fs::read_to_string(&path)?)?,
                Default::default(),
            ))
        })))
    });
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update(cx, |view, cx| {
        view.config = Config::parse(&fs::read_to_string(&local).unwrap()).unwrap();
        cx.notify();
    });
    for (label, key) in [
        ("status-show-Usage", "usage.show"),
        ("status-show-CPU and memory", "show_system_load"),
        ("status-show-Network quality", "show_network_quality"),
        ("status-show-Listening ports", "show_listening_ports"),
        ("status-show-Keep Awake", "status_bar.keep_awake"),
        ("status-show-Theme", "status_bar.show_theme"),
        ("status-show-Shortcuts", "status_bar.show_shortcuts"),
        ("status-show-Report issue", "status_bar.show_report_issue"),
    ] {
        for shown in [false, true] {
            cx.update(|window, cx| full_draw(window, cx).clear(cx));
            let at = cx.debug_bounds("connection-status").unwrap().center();
            cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::default());
            cx.simulate_mouse_up(at, MouseButton::Right, Modifiers::default());
            cx.update(|window, cx| full_draw(window, cx).clear(cx));
            let at = cx.debug_bounds(label).unwrap().center();
            cx.simulate_click(at, Modifiers::default());
            cx.run_until_parked();
            let value = toml::Value::Table(fs::read_to_string(&local).unwrap().parse().unwrap());
            let actual = key.split('.').fold(&value, |value, part| &value[part]);
            assert_eq!(actual.as_bool(), Some(shown), "{label}");
            view.read_with(cx, |view, _| {
                assert!(view.menu.page.is_none());
                assert_eq!(
                    view.config.status_bar.theme,
                    crate::config::status_bar::Button::Icon
                );
            });
        }
    }
}

#[test]
fn settings_visibility_edits_and_cli_edits_share_the_saved_button_style() -> anyhow::Result<()> {
    use crate::config::{
        preferences::Preference,
        status_bar::{Button, Detail, Edit},
    };
    let temp = TempDirectory::new()?;
    let path = temp.0.join("local.toml");
    fs::write(
        &path,
        "[status_bar]\ntheme = 'icon'\nshortcuts = 'icon'\nreport_issue = 'icon'\nusage = 'compact'\nsystem_load = 'compact'\n",
    )?;
    for (edit, key, button) in [
        (Edit::ShowTheme(false), "status_bar.show_theme", "theme"),
        (
            Edit::ShowShortcuts(false),
            "status_bar.show_shortcuts",
            "shortcuts",
        ),
        (
            Edit::ShowReportIssue(false),
            "status_bar.show_report_issue",
            "report_issue",
        ),
    ] {
        Config::save_preference_path(Preference::StatusBar(edit), &path)?;
        let hidden = Config::parse(&fs::read_to_string(&path)?)?;
        let effective = |config: &Config| match button {
            "theme" => config
                .status_bar
                .theme
                .with_visibility(config.status_bar.show_theme),
            "shortcuts" => config
                .status_bar
                .shortcuts
                .with_visibility(config.status_bar.show_shortcuts),
            "report_issue" => config
                .status_bar
                .report_issue
                .with_visibility(config.status_bar.show_report_issue),
            _ => unreachable!(),
        };
        assert_eq!(effective(&hidden), Button::Hidden);
        Config::edit_key_at(&path, &temp.0.join("absent"), key, Some(true.into()))?;
        let shown = Config::parse(&fs::read_to_string(&path)?)?;
        assert_eq!(effective(&shown), Button::Icon);
        assert_eq!(shown.status_bar.usage, Detail::Compact);
        assert_eq!(shown.status_bar.system_load, Detail::Compact);
    }
    Ok(())
}
