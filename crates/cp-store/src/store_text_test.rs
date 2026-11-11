use super::*;

#[test]
fn a_concurrent_content_edit_is_not_overwritten_by_an_older_repair() {
    let store = Store::in_memory().unwrap();
    let stale = Item::plain("older text");
    let id = store.insert_item("edited", &stale, "old", 1).unwrap();
    store.update_text(id, "newer text", 2).unwrap();
    assert!(!store.repair_item_search(id, &stale).unwrap());
    assert!(
        store
            .matches(
                &crate::Filter {
                    query: Some("newer".into()),
                    ..Default::default()
                },
                id
            )
            .unwrap()
    );
}

#[test]
fn windows_and_legacy_utf16_payloads_rebuild_text_with_current_folding() {
    for (format, big, bom) in [
        ("CF_UNICODETEXT", false, false),
        ("public.utf16-plain-text", false, true),
        ("public.utf16-plain-text", true, true),
    ] {
        let value = "中文完整尾词 café";
        let mut units: Vec<u16> = if bom { vec![0xfeff] } else { Vec::new() };
        units.extend(value.encode_utf16());
        if format == "CF_UNICODETEXT" {
            units.push(0);
        }
        let bytes = units
            .into_iter()
            .flat_map(|u| {
                if big {
                    u.to_be_bytes()
                } else {
                    u.to_le_bytes()
                }
            })
            .collect();
        let item = Item {
            kind: None,
            formats: vec![cp_core::item::Format {
                id: format.into(),
                payload: Payload::Inline(bytes),
            }],
        };
        let store = Store::in_memory().unwrap();
        let id = store.insert_item("utf16", &item, "中文", 1).unwrap();
        assert_eq!(store.repair_search().unwrap(), 1);
        assert!(
            store
                .matches(
                    &crate::Filter {
                        query: Some("整尾词 cafe".into()),
                        ..Default::default()
                    },
                    id
                )
                .unwrap()
        );
        assert_eq!(store.full_preview(id, &item).unwrap(), value);
    }
}

#[test]
fn repair_recovers_the_tail_without_changing_personal_metadata() {
    let store = Store::in_memory().unwrap();
    let text = format!("{}末尾关键字", "前文".repeat(1500));
    let item = Item::plain(&text);
    let id = store.insert_item("long", &item, "前文", 123).unwrap();
    store.set_label(id, Some("我的命名"), 456).unwrap();
    store.set_ocr_text(id, "图片文字", 789).unwrap();
    store.set_pinned(id, true, 900).unwrap();
    store.record_paste(id, 1000).unwrap();
    let before = store
        .list(&crate::Filter::default(), 100, None)
        .unwrap()
        .rows
        .remove(0);
    assert_eq!(store.repair_search().unwrap(), 1);
    assert_eq!(store.repair_search().unwrap(), 0);
    assert_eq!(
        store
            .count_matching(&crate::Filter {
                query: Some("尾关键".into()),
                ..Default::default()
            })
            .unwrap(),
        1
    );
    let after = store
        .list(&crate::Filter::default(), 100, None)
        .unwrap()
        .rows
        .remove(0);
    assert_eq!(
        (
            after.label,
            after.pinned,
            after.last_used_at,
            after.modified_at,
            after.created_at
        ),
        (
            before.label,
            before.pinned,
            before.last_used_at,
            before.modified_at,
            before.created_at
        )
    );
    assert_eq!(store.ocr_text(id).unwrap().as_deref(), Some("图片文字"));
    assert_eq!(store.item(id).unwrap(), Some(item));
}

#[test]
fn a_missing_payload_never_replaces_the_existing_search() {
    let store = Store::in_memory().unwrap();
    let item = Item {
        kind: None,
        formats: vec![cp_core::item::Format {
            id: SYNTHETIC_TEXT.into(),
            payload: Payload::Announced { size: Some(3000) },
        }],
    };
    let id = store
        .insert_item("missing", &item, "still searchable", 1)
        .unwrap();
    assert_eq!(store.repair_search().unwrap(), 0);
    assert!(
        store
            .matches(
                &crate::Filter {
                    query: Some("searchable".into()),
                    ..Default::default()
                },
                id
            )
            .unwrap()
    );
}

#[test]
fn matching_an_id_respects_search_pins_kinds_and_deletion() {
    let store = Store::in_memory().unwrap();
    let one = store.insert_text("one", "keep this", 1).unwrap();
    let other = store.insert_text("other", "another", 2).unwrap();
    let mut filter = crate::Filter {
        query: Some("keep".into()),
        ..Default::default()
    };
    assert!(store.matches(&filter, one).unwrap());
    assert!(!store.matches(&filter, other).unwrap());
    filter.pinned_only = true;
    assert!(!store.matches(&filter, one).unwrap());
    store.set_pinned(one, true, 3).unwrap();
    assert!(store.matches(&filter, one).unwrap());
    filter.kinds.push(cp_core::kind::Kind::Image);
    assert!(!store.matches(&filter, one).unwrap());
    filter.kinds.clear();
    store.mark_deleted(one, 4).unwrap();
    assert!(!store.matches(&filter, one).unwrap());
}
