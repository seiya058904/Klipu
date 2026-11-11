use super::*;

#[test]
fn empty_orders_require_one_valid_correlation_ticket() {
    assert_eq!(empty_ticket("empty 0"), Some("0".into()));
    assert_eq!(empty_ticket("empty 42"), Some("42".into()));
    for invalid in [
        "empty",
        "empty ",
        "empty -1",
        "empty 1 more",
        "empty nope",
        "empty 18446744073709551616",
    ] {
        assert_eq!(empty_ticket(invalid), None, "{invalid}");
    }
}
