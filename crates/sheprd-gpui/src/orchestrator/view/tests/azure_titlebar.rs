use super::*;
use crate::orchestrator::{Look, OrchestratorView, Request, SourceStatus, SyncState};
use gpui::{AppContext as _, Modifiers, TestAppContext, px, size};

#[gpui::test]
fn azure_sign_in_marks_the_account_and_opens_its_page(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = crate::sidebar::layout_tests::fixture_window(window, cx);
        view.settings.shared = Some(
            crate::herdr_settings::Settings::parse_text("[ui]\ntab_bar_position = 'bottom'")
                .unwrap(),
        );
        view
    });
    cx.simulate_resize(size(px(1100.), px(600.)));
    cx.run_until_parked();
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            let id = view.sections_view(cx);
            let mut ui = view.config.ui.clone();
            ui.size = view.config.terminal.size;
            let look = Look {
                theme: view.theme.clone(),
                ui,
                mono: view.config.terminal.clone(),
            };
            let replacement = cx.new(|cx| {
                let mut view = OrchestratorView::with_repo(
                    Request {
                        target: sheprd_client::ConnectTarget::Local,
                        checkout: dir.path().to_string_lossy().into_owned(),
                        workspace_id: "w1".into(),
                        token: None,
                        data_root: Some(dir.path().join("data")),
                        azure_store: Default::default(),
                        gitlab_store: Default::default(),
                        timing: Default::default(),
                    },
                    look,
                    cx,
                );
                view.repo_mut().snapshot.sources = vec![SourceStatus {
                    key: SourceKey {
                        provider: Provider::Azure,
                        host: "dev.azure.com".into(),
                        repository: "org/project".into(),
                    },
                    state: SyncState::SignIn,
                }];
                view
            });
            view.orchestrators.get_mut(&id).unwrap().view = replacement;
            view.menu.github = crate::github::Auth::connected_fixture();
            cx.notify();
        })
    });
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
    assert!(cx.debug_bounds("titlebar-avatar-attention").is_some());
    let account = cx.debug_bounds("titlebar-avatar").unwrap();
    cx.simulate_click(account.center(), Modifiers::none());
    assert_eq!(
        view.read_with(cx, |view, _| view.menu.page),
        Some(crate::menu::Page::AzureDevOps)
    );
}
