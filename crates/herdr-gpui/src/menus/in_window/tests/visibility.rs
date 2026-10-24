use super::*;

#[gpui::test]
fn hidden_menu_opens_without_moving_content_and_restores_focus(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.config.show_menu_bar = false;
            cx.notify();
        })
    });
    for width in [1000., 280.] {
        cx.simulate_resize(size(px(width), px(500.)));
        draw(cx);
        assert!(cx.debug_bounds("application-menu-bar").is_none());
        let body = cx.debug_bounds("window-body").unwrap();
        cx.simulate_keystrokes("f10");
        draw(cx);
        assert!(cx.debug_bounds("application-menu-bar").is_some());
        assert_eq!(cx.debug_bounds("window-body").unwrap(), body);
        let panel = cx.debug_bounds("application-menu-panel").unwrap();
        assert!(panel.top() > px(0.));
        assert!(panel.right() <= px(width));
        cx.simulate_keystrokes("a b c down f10");
        draw(cx);
        assert!(cx.debug_bounds("application-menu-bar").is_none());
        cx.update(|window, cx| {
            assert!(view.read(cx).menu.page.is_none());
            assert!(view.read(cx).focus.is_focused(window));
        });
    }
}

#[gpui::test]
fn hidden_menu_preserves_the_server_window_controls_setting(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    for (position, width, enabled) in [
        ("top", 280., true),
        ("bottom", 280., true),
        ("top", 319., true),
        ("top", 320., true),
        ("top", 1000., true),
        ("top", 280., false),
        ("top", 1000., false),
        ("bottom", 1000., false),
    ] {
        cx.simulate_resize(size(px(width), px(500.)));
        cx.update(|_, cx| {
            view.update(cx, |view, cx| {
                view.config.show_menu_bar = false;
                view.config.show_window_controls = enabled;
                view.settings.shared = Some(
                    crate::herdr_settings::Settings::parse_text(&format!(
                        "[ui]\ntab_bar_position = '{position}'"
                    ))
                    .unwrap(),
                );
                cx.notify();
            })
        });
        draw(cx);
        assert_eq!(cx.debug_bounds("window-close").is_some(), enabled);
        assert_eq!(
            cx.debug_bounds("titlebar").is_some(),
            position == "bottom" || enabled
        );
        if let Some(close) = cx.debug_bounds("window-close") {
            assert!(
                close.left() >= px(0.) && close.right() <= px(width),
                "{position}: {close:?}"
            );
        }
        let body = cx.debug_bounds("window-body").unwrap();
        cx.simulate_keystrokes("f10");
        draw(cx);
        assert_eq!(cx.debug_bounds("window-body").unwrap(), body);
        assert_eq!(cx.debug_bounds("window-close").is_some(), enabled);
        cx.simulate_keystrokes("escape");
        draw(cx);
        cx.update(|window, cx| {
            view.update(cx, |view, cx| {
                view.command(crate::controls::Command::ToggleZenMode, window, cx);
            })
        });
        draw(cx);
        assert_eq!(cx.debug_bounds("zen-inset").is_some(), enabled);
        assert_eq!(cx.debug_bounds("window-close").is_some(), enabled);
        cx.update(|window, cx| {
            view.update(cx, |view, cx| {
                view.command(crate::controls::Command::ToggleZenMode, window, cx);
                view.config.show_menu_bar = true;
                cx.notify();
            })
        });
        draw(cx);
        assert!(cx.debug_bounds("application-menu-bar").is_some());
        assert_eq!(cx.debug_bounds("titlebar").is_some(), position == "bottom");
    }
}

#[gpui::test]
fn zen_menu_is_temporary_and_does_not_move_content(cx: &mut TestAppContext) {
    let (view, cx) = window(cx);
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.command(crate::controls::Command::ToggleZenMode, window, cx);
        })
    });
    cx.run_until_parked();
    draw(cx);
    assert!(cx.debug_bounds("application-menu-bar").is_none());
    let body = cx.debug_bounds("window-body").unwrap();
    cx.simulate_keystrokes("f10");
    draw(cx);
    assert_eq!(cx.debug_bounds("window-body").unwrap(), body);
    assert!(cx.debug_bounds("application-menu-bar").is_some());
    cx.simulate_keystrokes("escape");
    draw(cx);
    assert!(cx.debug_bounds("application-menu-bar").is_none());
    view.read_with(cx, |view, _| assert!(view.config.show_menu_bar));
}

#[gpui::test]
fn hidden_menu_header_keeps_controls_accessible_with_long_git_counts_and_split_groups(
    cx: &mut TestAppContext,
) {
    let (view, cx) = window(cx);
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.config.show_menu_bar = false;
            view.config.show_window_controls = true;
            view.live.snapshot = Some(std::sync::Arc::new(
                serde_json::from_str(include_str!(
                    "../../../../../herdr-protocol/tests/fixtures/endpoint-snapshot-v1.json"
                ))
                .unwrap(),
            ));
            view.live.status = crate::state::ConnectionStatus::Connected;
            let group = view.ensure_layout().unwrap().active();
            view.split_group(group, window, cx);
            assert!(view.ensure_layout().unwrap().drag(0, 900., 1000.));
            view.git = crate::git::Git::fixture(
                crate::pull_request::Input {
                    checkout: None,
                    repo_key: Some(crate::sidebar::layout_tests::REPO_KEY.into()),
                    branch: "develop".into(),
                },
                crate::git::Status {
                    additions: 999_999_999,
                    deletions: 999_999_999,
                    untracked: 1,
                },
            );
            cx.notify();
        })
    });
    cx.run_until_parked();
    for width in [280., 320., 1000.] {
        cx.simulate_resize(size(px(width), px(500.)));
        for connected in [false, true] {
            view.update(cx, |view, cx| {
                view.menu.github = if connected {
                    crate::github::Auth::connected_fixture()
                } else {
                    Default::default()
                };
                view.sidebar_visible = connected;
                cx.notify();
            });
            draw(cx);
            assert!(cx.debug_bounds("titlebar").is_some());
            assert!(cx.debug_bounds("titlebar-git-additions").is_some());
            assert!(cx.debug_bounds("g1-group").unwrap().size.width < px(280.));
            for selector in ["titlebar-avatar", "window-close"] {
                let bounds = cx.debug_bounds(selector).unwrap();
                assert!(
                    bounds.left() >= px(0.) && bounds.right() <= px(width),
                    "{selector}: {bounds:?}"
                );
            }
            let body = cx.debug_bounds("window-body").unwrap();
            cx.simulate_keystrokes("f10");
            draw(cx);
            assert_eq!(cx.debug_bounds("window-body").unwrap(), body);
            cx.simulate_keystrokes("escape");
        }
    }
}
