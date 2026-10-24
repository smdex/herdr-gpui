use super::*;

#[test]
fn client_controls_are_required_and_server_controls_follow_the_existing_setting() {
    assert!(super::super::show_controls(false, FLOATING));
    assert!(super::super::show_controls(true, FLOATING));
    assert!(!super::super::show_controls(false, Decorations::Server));
    assert!(super::super::show_controls(true, Decorations::Server));
}
