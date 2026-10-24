use super::*;
use crate::{menu::StatusBarIo, sidebar::layout_tests::full_draw};
use gpui::{Modifiers, MouseButton, TestAppContext, px, size};

#[gpui::test]
fn status_bar_keyboard_scrolls_large_rows_into_view_and_activates_them(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.set_global(StatusBarIo(Arc::new(|key, shown| {
            assert_eq!(key, "status_bar.show_report_issue");
            assert!(!shown);
            let mut config = crate::config::Config::default();
            config.status_bar.show_report_issue = Some(false);
            Ok((config, Default::default()))
        })))
    });
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update(cx, |view, cx| {
        view.config.ui.size = 48.;
        cx.notify();
    });
    cx.simulate_resize(size(px(1000.), px(400.)));
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    let at = cx.debug_bounds("connection-status").unwrap().center();
    cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::default());
    cx.simulate_mouse_up(at, MouseButton::Right, Modifiers::default());
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    assert!(cx.debug_bounds("release-notes").is_none());
    cx.simulate_event(gpui::MouseMoveEvent {
        position: gpui::point(px(0.), px(0.)),
        ..Default::default()
    });
    cx.simulate_keystrokes("up");
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    let panel = cx.debug_bounds("menu-panel").unwrap();
    let last = cx.debug_bounds("status-awake-until-agents").unwrap();
    assert!(
        last.top() >= panel.top() && last.bottom() <= panel.bottom(),
        "{last:?} outside {panel:?}"
    );
    cx.simulate_keystrokes("down");
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    let first = cx.debug_bounds("status-show-Usage").unwrap();
    let panel = cx.debug_bounds("menu-panel").unwrap();
    assert!(first.top() >= panel.top() && first.bottom() <= panel.bottom());
    cx.simulate_keystrokes("up up");
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    let report = cx.debug_bounds("status-show-Report issue").unwrap();
    let panel = cx.debug_bounds("menu-panel").unwrap();
    assert!(report.top() >= panel.top() && report.bottom() <= panel.bottom());
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert!(view.menu.page.is_none());
        assert_eq!(view.config.status_bar.show_report_issue, Some(false));
    });
}

#[gpui::test]
fn release_notes_still_render_on_the_update_page(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            assert!(view.open_menu(window, cx));
            view.menu.page = Some(crate::menu::Page::Update);
            cx.notify();
        })
    });
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    assert!(cx.debug_bounds("release-notes").is_some());
    assert!(cx.debug_bounds("status-show-Usage").is_none());
}
