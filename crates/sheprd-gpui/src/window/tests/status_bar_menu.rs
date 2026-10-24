use super::*;
use crate::sidebar::layout_tests::full_draw;
use gpui::{Modifiers, MouseButton};

#[gpui::test]
fn right_clicking_the_status_bar_opens_a_checked_menu_without_activating_a_button(
    cx: &mut gpui::TestAppContext,
) {
    let (view, cx) = cx.add_window_view(fixture_window);
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    let at = cx.debug_bounds("status-caffeine").unwrap().center();
    cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::default());
    cx.simulate_mouse_up(at, MouseButton::Right, Modifiers::default());
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    assert!(cx.debug_bounds("status-show-Usage").is_some());
    cx.update(|window, cx| assert!(view.read(cx).menu.focus.is_focused(window)));
    view.read_with(cx, |_, cx| {
        assert_eq!(crate::caffeine::cup(cx), crate::caffeine::Cup::Off)
    });
    cx.simulate_keystrokes("escape");
    view.read_with(cx, |view, _| assert!(view.menu.page.is_none()));
}

#[gpui::test]
fn status_bar_menu_keyboard_can_reach_the_until_agents_action(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    let at = cx.debug_bounds("connection-status").unwrap().center();
    cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::default());
    cx.simulate_mouse_up(at, MouseButton::Right, Modifiers::default());
    cx.simulate_keystrokes("up enter");
    cx.run_until_parked();
    view.read_with(cx, |view, cx| {
        assert!(view.menu.page.is_none());
        assert_eq!(crate::caffeine::cup(cx), crate::caffeine::Cup::Off);
    });
}

#[gpui::test]
fn a_narrow_bar_with_every_optional_item_hidden_still_opens_and_dismisses_the_menu(
    cx: &mut gpui::TestAppContext,
) {
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update(cx, |view, cx| {
        view.config.usage.show = false;
        view.config.show_system_load = false;
        view.config.show_network_quality = false;
        view.config.show_listening_ports = false;
        view.config.status_bar.keep_awake = false;
        view.config.status_bar.show_theme = Some(false);
        view.config.status_bar.show_shortcuts = Some(false);
        view.config.status_bar.show_report_issue = Some(false);
        view.local_error = Some("Connection error".into());
        cx.notify();
    });
    cx.simulate_resize(gpui::size(gpui::px(320.), gpui::px(480.)));
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    assert!(cx.debug_bounds("status-version").is_some());
    assert!(cx.debug_bounds("connection-message").is_some());
    assert!(cx.debug_bounds("status-caffeine").is_none());
    let at = cx.debug_bounds("connection-status").unwrap().center();
    cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::default());
    cx.simulate_mouse_up(at, MouseButton::Right, Modifiers::default());
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    let panel = cx.debug_bounds("menu-panel").unwrap();
    assert!(panel.left() >= gpui::px(0.) && panel.right() <= gpui::px(320.));
    assert!(panel.top() >= gpui::px(0.) && panel.bottom() <= gpui::px(480.));
    for selector in [
        "status-show-Usage",
        "status-show-CPU and memory",
        "status-show-Network quality",
        "status-show-Listening ports",
        "status-show-Keep Awake",
        "status-show-Theme",
        "status-show-Shortcuts",
        "status-show-Report issue",
    ] {
        assert!(cx.debug_bounds(selector).is_some(), "{selector}");
    }
    cx.simulate_click(
        gpui::point(gpui::px(1.), gpui::px(1.)),
        Modifiers::default(),
    );
    view.read_with(cx, |view, _| assert!(view.menu.page.is_none()));
}
