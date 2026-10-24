use super::*;

#[gpui::test]
fn hiding_a_pending_hold_releases_it_even_if_shown_again_before_completion(
    cx: &mut gpui::TestAppContext,
) {
    let (log, reports) = (Log::default(), Reports::default());
    cx.update(|cx| {
        toggle_with(cx, holder(&log), reports.sink());
        assert_eq!(cup(cx), Cup::Pending);
        set_visible(false, cx, reports.sink());
        set_visible(true, cx, reports.sink());
    });
    cx.run_until_parked();
    assert_eq!(cup_of(cx), Cup::Off);
    assert_eq!(commands(&log), [Command::Hold, Command::Release]);
    cx.update(|cx| set_visible(true, cx, reports.sink()));
    cx.run_until_parked();
    assert_eq!(cup_of(cx), Cup::Off);
    assert!(reports.take().is_empty());
}

#[gpui::test]
fn hiding_either_active_mode_releases_the_guard_and_restoring_stays_off(
    cx: &mut gpui::TestAppContext,
) {
    let (log, reports) = (Log::default(), Reports::default());
    for mode in [Mode::Indefinite, Mode::UntilAgentsFinish] {
        cx.update(|cx| {
            cx.default_global::<Caffeine>().working.insert(window(1));
            hold_with(cx, mode, holder(&log), reports.sink());
        });
        cx.run_until_parked();
        assert_eq!(cup_of(cx), Cup::On(mode));
        cx.update(|cx| set_visible(false, cx, reports.sink()));
        assert_eq!(cup_of(cx), Cup::Pending);
        cx.run_until_parked();
        assert_eq!(cup_of(cx), Cup::Off);
        cx.update(|cx| set_visible(true, cx, reports.sink()));
        assert_eq!(cup_of(cx), Cup::Off);
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
    assert!(reports.take().is_empty());
}

struct FailingRelease;
impl Drop for FailingRelease {
    fn drop(&mut self) {
        panic!("release failed");
    }
}

#[gpui::test]
fn hiding_reports_a_worker_release_failure_without_stranding_a_pending_cup(
    cx: &mut gpui::TestAppContext,
) {
    let reports = Reports::default();
    let (view, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    cx.update(|_, cx| toggle_with(cx, |_| Ok(FailingRelease), reports.sink()));
    cx.run_until_parked();
    view.update(cx, |_, cx| {
        let report = crate::HerdrWindow::caffeine_report(cx);
        set_visible(false, cx, report);
    });
    assert_eq!(cx.update(|_, cx| cup(cx)), Cup::Pending);
    cx.run_until_parked();
    assert_eq!(cx.update(|_, cx| cup(cx)), Cup::Off);
    view.read_with(cx, |view, _| {
        assert_eq!(
            view.flash.as_ref().unwrap().0.text.as_ref(),
            crate::Error::Caffeine(Error::WorkerStopped).to_string()
        );
    });
    cx.update(|_, cx| set_visible(true, cx, reports.sink()));
    assert_eq!(cx.update(|_, cx| cup(cx)), Cup::Off);
    assert!(reports.take().is_empty());
}
