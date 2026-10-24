use super::*;
use gpui::prelude::*;

struct AltInput(Entity<HerdrWindow>);

impl Render for AltInput {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let view = self.0.clone();
        div().size_full().child(view.clone()).map(|root| {
            #[cfg(not(windows))]
            let root = root.on_modifiers_changed(move |event, window, cx| {
                view.update(cx, |view, cx| {
                    view.application_menu_modifiers(event, window, cx)
                });
            });
            root
        })
    }
}

// Only the Windows modifier delivery is adapted; input and activation use the real handlers.
fn alt_window(cx: &mut TestAppContext) -> (Entity<HerdrWindow>, &mut VisualTestContext) {
    let (root, cx) = cx.add_window_view(|window, cx| {
        AltInput(cx.new(|cx| {
            crate::bind_keys(cx);
            let mut view = fixture_window(window, cx);
            view._activation =
                cx.observe_window_activation(window, HerdrWindow::window_activation_changed);
            view.focus.focus(window, cx);
            view
        }))
    });
    let view = root.read_with(cx, |root, _| root.0.clone());
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    (view, cx)
}

#[gpui::test]
fn lone_alt_opens_but_alt_chords_and_altgr_do_not(cx: &mut TestAppContext) {
    let (view, cx) = alt_window(cx);
    draw(cx);
    let alt = Modifiers {
        alt: true,
        ..Modifiers::default()
    };
    cx.simulate_modifiers_change(alt);
    assert!(view.read_with(cx, |view, _| view.menu.page.is_none()));
    cx.simulate_modifiers_change(Modifiers::default());
    draw(cx);
    assert!(view.read_with(cx, |view, _| view.menu.page == Some(Page::Application)));
    cx.simulate_keystrokes("escape");
    draw(cx);
    for chord in [false, true] {
        cx.simulate_modifiers_change(if chord {
            alt
        } else {
            Modifiers {
                control: true,
                ..alt
            }
        });
        if chord {
            cx.simulate_keystrokes("alt-x");
        } else {
            cx.simulate_modifiers_change(alt);
        }
        cx.simulate_modifiers_change(Modifiers::default());
        assert!(view.read_with(cx, |view, _| view.menu.page.is_none()));
    }
}

#[gpui::test]
fn activation_modifier_resync_cannot_open_the_menu_on_alt_release(cx: &mut TestAppContext) {
    let (view, cx) = alt_window(cx);
    draw(cx);
    let alt = Modifiers {
        alt: true,
        ..Modifiers::default()
    };
    cx.deactivate_window();
    view.read_with(cx, |view, _| assert!(!view.active));
    // Windows resynchronizes modifiers before asynchronously reporting activation.
    cx.simulate_modifiers_change(alt);
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    view.read_with(cx, |view, _| assert!(view.active));
    cx.simulate_modifiers_change(Modifiers::default());
    view.read_with(cx, |view, _| assert!(view.menu.page.is_none()));
    cx.update(|window, cx| assert!(view.read(cx).focus.is_focused(window)));

    cx.simulate_modifiers_change(alt);
    cx.deactivate_window();
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    cx.simulate_modifiers_change(Modifiers::default());
    view.read_with(cx, |view, _| assert!(view.menu.page.is_none()));
}
