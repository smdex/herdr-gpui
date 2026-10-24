//! The status bar cup's buttons reach the keep-awake modes.
use super::*;
use crate::sidebar::layout_tests::{fixture_window, full_draw};
use gpui::{Modifiers, MouseButton};

#[gpui::test]
fn the_status_bar_menu_holds_until_agents_finish_and_either_control_ends_it(
    cx: &mut gpui::TestAppContext,
) {
    let log = Log::default();
    let (view, cx) = cx.add_window_view(fixture_window);
    cx.update(|window, cx| {
        let id = window.window_handle().window_id();
        // A worker that already exists never runs the real power APIs.
        cx.set_global(Caffeine {
            worker: Some(spawn(holder(&log)).unwrap()),
            working: HashSet::from([id]),
            ..Caffeine::default()
        });
        full_draw(window, cx).clear(cx);
    });
    let click = |cx: &mut gpui::VisualTestContext, button| {
        let at = cx.debug_bounds("status-caffeine").unwrap().center();
        cx.simulate_mouse_down(at, button, Modifiers::default());
        cx.simulate_mouse_up(at, button, Modifiers::default());
        if button == MouseButton::Right {
            cx.update(|window, cx| full_draw(window, cx).clear(cx));
            let at = cx
                .debug_bounds("status-awake-until-agents")
                .unwrap()
                .center();
            cx.simulate_click(at, Modifiers::default());
        }
        cx.run_until_parked();
        let state = cx.update(|window, cx| {
            full_draw(window, cx).clear(cx);
            // Selecting a mode dismisses the menu.
            assert!(view.read(cx).menu.page.is_none());
            cup(cx)
        });
        let dot = cx.debug_bounds("status-caffeine-until-agents").is_some();
        (state, dot)
    };
    let until = Cup::On(Mode::UntilAgentsFinish);
    for ending in [MouseButton::Right, MouseButton::Left] {
        assert_eq!(click(cx, MouseButton::Right), (until, true));
        assert_eq!(click(cx, ending), (Cup::Off, false), "{ending:?}");
    }
    assert_eq!(
        commands(&log),
        [
            Command::Hold,
            Command::Release,
            Command::Hold,
            Command::Release
        ]
    );
}
