use super::{State, backdrop, limitations};
use gpui::{
    BackgroundBlurSupport, DecorationNegotiation, WindowBackgroundAppearance, WindowDecorations,
};

#[test]
fn appearance_limitations_require_processed_refusal_and_actual_blur_request() {
    use BackgroundBlurSupport::*;
    use WindowDecorations::{Client, Server};
    let refused = Some(DecorationNegotiation::Configured {
        requested: Client,
        mode: Server,
    });
    for initial in [None, Some(Server), Some(Client)] {
        for decoration in [
            None,
            Some(DecorationNegotiation::Pending { requested: Client }),
            Some(DecorationNegotiation::Configured {
                requested: Client,
                mode: Client,
            }),
            Some(DecorationNegotiation::Configured {
                requested: Server,
                mode: Server,
            }),
            refused,
        ] {
            for appearance in [
                WindowBackgroundAppearance::Opaque,
                WindowBackgroundAppearance::Transparent,
                WindowBackgroundAppearance::Blurred,
            ] {
                for support in [None, Some(Pending), Some(Supported), Some(Unsupported)] {
                    let lines = limitations(initial, decoration, appearance, support);
                    assert_eq!(
                        lines.iter().any(|line| line.contains("decorations")),
                        initial == Some(Client) && decoration == refused
                    );
                    assert_eq!(
                        lines.iter().any(|line| line.contains("blur")),
                        appearance == WindowBackgroundAppearance::Blurred
                            && support == Some(Unsupported)
                    );
                }
            }
        }
    }
}

#[test]
fn appearance_limitations_dismissal_does_not_change_live_reporting() {
    let mut state = State::default();
    let unsupported = Some(BackgroundBlurSupport::Unsupported);
    let sync = |state: &mut State, appearance, support| {
        let lines = limitations(state.initial_decorations, None, appearance, support);
        state.sync_limitations(&lines);
        lines
    };
    let original = sync(&mut state, WindowBackgroundAppearance::Blurred, unsupported);
    assert!(!original.is_empty());
    let drawn = state.limitations.visible().unwrap().clone();
    assert!(state.limitations.dismiss(&drawn));
    assert_eq!(
        sync(&mut state, WindowBackgroundAppearance::Blurred, unsupported),
        original
    );
    assert!(state.limitations.visible().is_none());
    assert!(
        sync(
            &mut state,
            WindowBackgroundAppearance::Blurred,
            Some(BackgroundBlurSupport::Supported)
        )
        .is_empty()
    );
    assert!(state.limitations.visible().is_none());
    sync(&mut state, WindowBackgroundAppearance::Blurred, unsupported);
    assert!(state.limitations.visible().is_some());
    sync(
        &mut state,
        WindowBackgroundAppearance::Transparent,
        unsupported,
    );
    assert!(state.limitations.visible().is_none());
}

#[test]
fn appearance_limitations_follow_effective_backdrop_not_just_saved_blur() {
    let mut config = crate::config::Backdrop {
        blur: true,
        ..Default::default()
    };
    let unavailable = |config: &crate::config::Backdrop, picture| {
        limitations(
            None,
            None,
            backdrop::appearance(config, picture),
            Some(BackgroundBlurSupport::Unsupported),
        )
    };
    assert!(unavailable(&config, false).is_empty());
    config.opacity = 0.5;
    assert!(!unavailable(&config, false).is_empty());
    assert!(unavailable(&config, true).is_empty());
    config.blur = false;
    assert!(unavailable(&config, false).is_empty());
}

#[gpui::test]
fn appearance_limitations_coexist_with_connection_and_config_diagnostics(
    cx: &mut gpui::TestAppContext,
) {
    let (view, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    view.update_in(cx, |view, window, cx| {
        view.backdrop = State::new(Some(WindowDecorations::Client));
        // Live preferences cannot change the request made when this window opened.
        view.config.request_client_side_decorations = false;
        assert_eq!(
            view.backdrop.initial_decorations,
            Some(WindowDecorations::Client)
        );
        view.live.status = crate::state::ConnectionStatus::Disconnected;
        view.live.error = Some("connection failed".into());
        view.local_error = Some("old operation error".into());
        let status = view.live.status_text(view.local_error.as_deref());
        view.gui_config_diagnostic.sync(Some("config warning"));
        let before = view.config_diagnostic_cards(cx).len();
        let lines = limitations(
            view.backdrop.initial_decorations,
            Some(DecorationNegotiation::Configured {
                requested: WindowDecorations::Client,
                mode: WindowDecorations::Server,
            }),
            WindowBackgroundAppearance::Transparent,
            None,
        );
        view.backdrop.sync_limitations(&lines);
        assert_eq!(view.config_diagnostic_cards(cx).len(), before + 1);
        assert_eq!(view.live.status_text(view.local_error.as_deref()), status);
        assert!(status.contains("connection failed"));
        assert!(!status.contains("old operation error"));
        let drawn = view.backdrop.limitations.visible().unwrap().clone();
        view.backdrop.limitations.dismiss(&drawn);
        view.backdrop.sync_limitations(&lines);
        assert_eq!(view.config_diagnostic_cards(cx).len(), before);
        assert!(view.gui_config_diagnostic.visible().is_some());
        // Unreported test-backend queries must not be guessed by CLI reporting.
        assert!(view.appearance_limitations(window).is_empty());
    });
}
