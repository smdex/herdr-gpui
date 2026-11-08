use super::State;

#[gpui::test]
fn appearance_limitations_coexist_with_connection_and_config_diagnostics(
    cx: &mut gpui::TestAppContext,
) {
    let (view, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    view.update_in(cx, |view, window, cx| {
        view.backdrop = State::default();
        view.live.status = crate::state::ConnectionStatus::Disconnected;
        view.live.error = Some("connection failed".into());
        view.local_error = Some("old operation error".into());
        let status = view.live.status_text(view.local_error.as_deref());
        view.gui_config_diagnostic.sync(Some("config warning"));
        let before = view.config_diagnostic_cards(cx).len();

        // Exercise the notice plumbing independently of GPUI capabilities,
        // whose negotiation and support status are currently unavailable.
        let lines = vec!["appearance warning".to_string()];
        view.backdrop.sync_limitations(&lines);
        assert!(view.notice_shown(crate::window::NoticeCard::Appearance));
        assert_eq!(view.config_diagnostic_cards(cx).len(), before + 1);
        assert_eq!(view.live.status_text(view.local_error.as_deref()), status);
        assert!(status.contains("connection failed"));
        assert!(!status.contains("old operation error"));
        let drawn = view.backdrop.limitations.visible().unwrap().clone();
        view.backdrop.limitations.dismiss(&drawn);
        view.backdrop.sync_limitations(&lines);
        assert!(!view.notice_shown(crate::window::NoticeCard::Appearance));
        assert_eq!(view.config_diagnostic_cards(cx).len(), before);
        assert!(view.gui_config_diagnostic.visible().is_some());
        // Unknown platform capabilities must not be reported as failures.
        assert!(view.appearance_limitations(window).is_empty());
    });
}
