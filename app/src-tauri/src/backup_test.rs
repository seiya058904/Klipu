use crate::settings::history;

#[test]
fn backup_and_clear_operations_share_one_exclusive_claim() {
    let crossing = super::Crossing::default();
    let claim = crossing.claim().unwrap();
    assert!(crossing.underway());
    assert!(crossing.claim().is_none());
    drop(claim);
    assert!(!crossing.underway());
    assert!(crossing.claim().is_some());
}

#[test]
fn the_review_exposes_quality_and_retention_without_any_contents() {
    let review = super::Review {
        token: "once".into(),
        name: "test.cpbackup".into(),
        preview: cp_store::backup::Preview {
            partial: 1,
            expires: 2,
            bytes_after: 123,
            ..Default::default()
        },
    };
    let value = serde_json::to_value(review).unwrap();
    assert_eq!(value["token"], "once");
    assert_eq!(value["partial"], 1);
    assert_eq!(value["expires"], 2);
    assert_eq!(value["bytesAfter"], 123);
    assert!(value.get("contents").is_none());
}

#[test]
fn the_history_a_backup_speaks_for_is_the_one_the_panel_writes() {
    let Ok(db) = history() else {
        return;
    };
    let dir = crate::settings::folder().expect("a folder");
    assert!(db.starts_with(&dir));
    assert_eq!(
        db.file_name().and_then(|it| it.to_str()),
        Some("history.db")
    );
}
