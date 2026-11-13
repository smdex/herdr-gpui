//! Toggle Menu Bar: the bar gives its row to the window, F10 still opens the
//! menus over it, and the window controls move to the title bar.

use super::*;
use crate::controls::Command;
use gpui::{point, px, size};

fn toggle(view: &Entity<SheprdWindow>, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.command(Command::ToggleMenuBarTemporary, window, cx)
        })
    });
    draw(cx);
}

fn press_f10(view: &Entity<SheprdWindow>, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.open_application_menu(Some(0), window, cx)
        })
    });
    draw(cx);
}

#[gpui::test]
fn hiding_gives_the_row_to_the_window_and_showing_takes_it_back(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    cx.simulate_resize(size(px(1000.), px(600.)));
    draw(cx);
    let bar = cx.debug_bounds("application-menu-bar").unwrap();
    let body = cx.debug_bounds("window-body").unwrap();
    assert!(bar.size.height > px(0.));

    toggle(&view, cx);
    assert!(cx.debug_bounds("application-menu-bar").is_none());
    let hidden = cx.debug_bounds("window-body").unwrap();
    assert_eq!(hidden.top(), body.top() - bar.size.height);
    assert_eq!(hidden.bottom(), body.bottom());

    toggle(&view, cx);
    assert_eq!(cx.debug_bounds("application-menu-bar").unwrap(), bar);
    assert_eq!(cx.debug_bounds("window-body").unwrap(), body);
}

/// The menus open from the first frame at the bar's full height and over the
/// window, so the terminal below never resizes for them.
#[gpui::test]
fn f10_opens_the_menus_over_the_window_while_hidden(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    cx.simulate_resize(size(px(1000.), px(600.)));
    draw(cx);
    let shown = cx.debug_bounds("application-menu-bar").unwrap();
    toggle(&view, cx);
    let body = cx.debug_bounds("window-body").unwrap();

    press_f10(&view, cx);
    let panel = cx.debug_bounds("application-menu-panel").unwrap();
    assert!((panel.top() - shown.bottom()).abs() <= px(1.));
    assert_eq!(cx.debug_bounds("window-body").unwrap(), body);
    assert!(!view.read_with(cx, |view, _| view.menu_bar_visible));
}

/// Headings measured while hidden stay under the overlay's, so a later
/// heading's menu opens beneath it rather than at the window's edge.
#[gpui::test]
fn a_later_headings_menu_hangs_under_it_while_hidden(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    cx.simulate_resize(size(px(1200.), px(700.)));
    draw(cx);
    toggle(&view, cx);
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.open_application_menu(Some(2), window, cx)
        })
    });
    draw(cx);
    let measured = view.read_with(cx, |view, _| view.menu.application_bar.buttons.borrow()[2]);
    let panel = cx.debug_bounds("application-menu-panel").unwrap();
    assert!(measured.left() > px(0.));
    assert!((panel.left() - measured.left()).abs() <= px(1.));
}

#[gpui::test]
fn initially_hidden_bar_measures_headings_for_navigation(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    cx.simulate_resize(size(px(1200.), px(700.)));
    view.update(cx, |view, cx| {
        view.menu_bar_visible = false;
        cx.notify();
    });
    draw(cx);
    press_f10(&view, cx);
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.open_application_menu(Some(2), window, cx)
        })
    });
    draw(cx);
    let measured = view.read_with(cx, |view, _| view.menu.application_bar.buttons.borrow()[2]);
    let panel = cx.debug_bounds("application-menu-panel").unwrap();
    assert!(measured.left() > px(0.));
    assert!((panel.left() - measured.left()).abs() <= px(1.));
}

#[gpui::test]
fn hidden_menu_stays_clamped_when_resized(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    view.update(cx, |view, cx| {
        view.menu_bar_visible = false;
        cx.notify();
    });
    cx.simulate_resize(size(px(1200.), px(700.)));
    draw(cx);
    press_f10(&view, cx);
    for width in [280., 1200., 360., 1000.] {
        cx.simulate_resize(size(px(width), px(700.)));
        draw(cx);
        let panel = cx.debug_bounds("application-menu-panel").unwrap();
        assert!(panel.left() >= px(0.) && panel.right() <= px(width));
    }
}

#[gpui::test]
fn initially_hidden_bar_places_f10_menu_on_first_frame(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    cx.simulate_resize(size(px(1200.), px(700.)));
    view.update(cx, |view, cx| {
        view.menu_bar_visible = false;
        cx.notify();
    });
    draw(cx);
    press_f10(&view, cx);
    let button = view.read_with(cx, |view, _| view.menu.application_bar.buttons.borrow()[0]);
    let panel = cx.debug_bounds("application-menu-panel").unwrap();
    assert!((panel.left() - button.left().max(px(8.))).abs() <= px(1.));
}

/// With the bar gone, nothing else on screen would close the window.
#[gpui::test]
fn the_window_controls_follow_into_the_title_bar(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    view.update(cx, |view, cx| {
        view.config.show_window_controls = true;
        cx.notify();
    });
    cx.simulate_resize(size(px(1000.), px(600.)));
    draw(cx);
    let bar = cx.debug_bounds("application-menu-bar").unwrap();
    let controls = cx.debug_bounds("window-controls").unwrap();
    assert!(bar.contains(&point(controls.left(), controls.center().y)));

    toggle(&view, cx);
    let controls = cx.debug_bounds("window-controls").unwrap();
    let header = cx
        .debug_bounds("titlebar")
        .or_else(|| cx.debug_bounds("strip-titlebar-trailing"))
        .unwrap();
    assert!(header.contains(&point(controls.left(), controls.center().y)));
}

#[gpui::test]
fn persistent_toggle_reverses_temporary_override(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    for saved in [true, false] {
        view.update(cx, |view, cx| {
            view.config.show_menu_bar = saved;
            view.menu_bar_visible = saved;
            cx.notify();
        });
        toggle(&view, cx);
        assert_eq!(view.read_with(cx, |view, _| view.menu_bar_visible), !saved);
        cx.update(|window, cx| {
            view.update(cx, |view, cx| {
                view.command(Command::ToggleMenuBar, window, cx)
            })
        });
        cx.run_until_parked();
        assert_eq!(view.read_with(cx, |view, _| view.menu_bar_visible), saved);
        assert_eq!(
            view.read_with(cx, |view, _| view.config.show_menu_bar),
            saved
        );
    }
}

#[gpui::test]
fn a_temporary_override_survives_an_unrelated_preference_reload(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    draw(cx);
    // ctrl-alt-m hid the bar for this session without persisting it.
    toggle(&view, cx);
    assert!(!view.read_with(cx, |view, _| view.menu_bar_visible));
    // Saving any other preference reloads the whole config. The reload must
    // not snap the bar back while `show_menu_bar` itself is unchanged.
    view.update(cx, |view, cx| {
        let mut config = view.config.clone();
        config.smooth_scroll = !config.smooth_scroll;
        let theme = view.theme.clone();
        view.load_gui_config_with(move || Ok((config, theme)), cx);
    });
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert!(!view.menu_bar_visible);
        assert!(!view.config.smooth_scroll);
    });
}

#[gpui::test]
fn persistent_toggle_rejects_concurrent_write_and_keeps_state_on_failure(cx: &mut TestAppContext) {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    let (view, cx) = window(cx);
    let calls = Arc::new(AtomicUsize::new(0));
    view.update(cx, |view, cx| {
        view.config.show_menu_bar = true;
        view.menu_bar_visible = true;
        let calls = calls.clone();
        view.toggle_persistent_menu_bar_with(
            move |shown| {
                assert!(!shown);
                calls.fetch_add(1, Ordering::SeqCst);
                Err(crate::config::Error::InvalidTerminalPadding("padding_x").into())
            },
            cx,
        );
        assert!(view.config_load.is_some());
        view.toggle_persistent_menu_bar_with(|_| panic!("a second write must not be started"), cx);
        assert!(view.flash.is_some());
        assert!(view.menu_bar_visible);
    });
    cx.run_until_parked();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    view.read_with(cx, |view, _| {
        assert!(view.menu_bar_visible);
        assert!(view.config.show_menu_bar);
        assert!(view.local_error.is_some());
        assert!(view.config_load.is_none());
    });
}

#[gpui::test]
fn persistent_menu_writes_are_gated_across_windows(cx: &mut TestAppContext) {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    let (first, _) = cx.add_window_view(fixture_window);
    let (second, _) = cx.add_window_view(fixture_window);
    let calls = Arc::new(AtomicUsize::new(0));
    let (release, wait) = std::sync::mpsc::channel();
    first.update(cx, |view, cx| {
        view.config.show_menu_bar = true;
        view.menu_bar_visible = true;
        let calls = calls.clone();
        view.toggle_persistent_menu_bar_with(
            move |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                wait.recv().unwrap();
                Err(crate::config::Error::InvalidTerminalPadding("padding_x").into())
            },
            cx,
        );
    });
    second.update(cx, |view, cx| {
        view.config.show_menu_bar = true;
        view.menu_bar_visible = true;
        view.toggle_persistent_menu_bar_with(|_| panic!("concurrent write"), cx);
        assert!(view.flash.is_some());
        assert!(view.menu_bar_visible);
    });
    release.send(()).unwrap();
    cx.run_until_parked();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    second.update(cx, |view, cx| {
        let calls = calls.clone();
        view.toggle_persistent_menu_bar_with(
            move |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                Err(crate::config::Error::InvalidTerminalPadding("padding_x").into())
            },
            cx,
        );
    });
    cx.run_until_parked();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[gpui::test]
fn the_toggle_lasts_for_the_session_and_leaves_the_setting_alone(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    draw(cx);
    toggle(&view, cx);
    assert!(!view.read_with(cx, |view, _| view.menu_bar_visible));
    assert!(view.read_with(cx, |view, _| view.config.show_menu_bar));
}
