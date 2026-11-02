use super::*;

#[test]
fn a_first_install_takes_the_tour() {
    assert_eq!(decide(true, None), Greeting::Tour);
    assert_eq!(decide(true, Some("1.0.0")), Greeting::Tour);
}

#[test]
fn a_first_run_cut_short_before_the_welcome_closed_takes_the_tour_again() {
    assert_eq!(decide(false, None), Greeting::Tour);
}

#[test]
fn a_welcome_nobody_can_read_is_treated_as_never_given() {
    assert_eq!(decide(false, Some("not a version")), Greeting::Tour);
}

#[test]
fn a_welcome_already_given_says_nothing() {
    assert_eq!(decide(false, Some("1.0.0")), Greeting::Nothing);
    assert_eq!(decide(false, Some("0.9.0")), Greeting::Nothing);
}

#[test]
fn only_a_welcome_behind_the_running_version_is_moved() {
    assert!(behind(None, "1.0.0"));
    assert!(behind(Some("0.9.0"), "1.0.0"));
    assert!(!behind(Some("1.0.0"), "1.0.0"));
    assert!(!behind(Some("1.1.0"), "1.0.0"));
}

#[test]
fn opening_it_again_asks_the_window_for_the_shortcut_alone() {
    assert!(!remembers(Some(&Greeting::Keys)));
}

#[test]
fn closing_the_shortcut_alone_leaves_the_welcome_as_it_was() {
    assert!(remembers(Some(&Greeting::Tour)));
    assert!(remembers(None));
}
