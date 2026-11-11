use super::*;
use cp_core::item::{Format, Item, Payload, SYNTHETIC_TEXT};

fn of_kind(kind: Kind, text: &str) -> Item {
    Item {
        kind: Some(kind),
        formats: vec![Format {
            id: SYNTHETIC_TEXT.into(),
            payload: Payload::Inline(text.as_bytes().to_vec()),
        }],
    }
}

fn kept(store: &Store, kind: Kind, text: &str, at: i64) -> i64 {
    store
        .insert_item(&format!("uuid-{at}"), &of_kind(kind, text), text, at)
        .expect("stored")
}

fn group_of(store: &Store, id: i64) -> String {
    store
        .raw()
        .query_row("SELECT group_key FROM items WHERE id = ?1", [id], |row| {
            row.get(0)
        })
        .expect("read")
}

fn old_image(store: &Store, at: i64, formats: &[&str]) -> i64 {
    let item = Item {
        kind: Some(Kind::Image),
        formats: formats
            .iter()
            .map(|id| Format {
                id: (*id).into(),
                payload: Payload::Inline(vec![1, 2, 3]),
            })
            .collect(),
    };
    store
        .insert_item(&format!("image-{at}"), &item, "", at)
        .expect("stored")
}

#[test]
fn old_jpeg_images_recover_missing_work_once_even_if_ocr_finds_nothing() {
    let store = Store::in_memory().expect("schema");
    let jfif = old_image(&store, 1, &["JFIF"]);
    let legacy = old_image(&store, 2, &["image/jpeg"]);
    assert_eq!(store.catch_up_jpeg().expect("caught up"), 2);
    for job in ["thumb", "ocr"] {
        assert_eq!(
            store.take_pending(job, 100, 10).expect("queued"),
            [legacy, jfif]
        );
        for id in [jfif, legacy] {
            store.work_done(id, job).expect("finished without a result");
        }
    }
    assert_eq!(store.catch_up_jpeg().expect("already considered"), 0);
    for job in ["thumb", "ocr"] {
        assert!(store.take_pending(job, 100, 10).expect("queue").is_empty());
    }
}

#[test]
fn jpeg_recovery_preserves_results_and_existing_retry_delays() {
    let store = Store::in_memory().expect("schema");
    let complete = old_image(&store, 1, &["JFIF"]);
    store
        .set_thumb(complete, Some("kept.png"), 2)
        .expect("thumbnail");
    store
        .set_ocr_text(complete, "already read", 2)
        .expect("OCR");
    let delayed = old_image(&store, 3, &["image/jpeg"]);
    store.enqueue(delayed, "ocr").expect("queued");
    assert!(
        store
            .work_failed(delayed, "ocr", "try later", 1_000)
            .expect("delayed")
    );
    assert_eq!(store.catch_up_jpeg().expect("caught up"), 2);
    assert_eq!(
        store.take_pending("thumb", 100, 10).expect("queue"),
        [delayed]
    );
    assert!(
        store
            .take_pending("ocr", 100, 10)
            .expect("queue")
            .is_empty()
    );
    assert_eq!(
        store.take_pending("ocr", 1_000, 10).expect("queue"),
        [delayed]
    );
    assert_eq!(
        store.thumb_path(complete).expect("thumb").as_deref(),
        Some("kept.png")
    );
    assert_eq!(
        store.ocr_text(complete).expect("OCR").as_deref(),
        Some("already read")
    );
}

#[test]
fn jpeg_recovery_ignores_deleted_unreadable_and_already_png_images() {
    let store = Store::in_memory().expect("schema");
    old_image(&store, 1, &["JFIF", "PNG"]);
    old_image(&store, 2, &["image/jpeg", "image/png"]);
    old_image(&store, 3, &["PNG"]);
    let deleted = old_image(&store, 4, &["JFIF"]);
    store.mark_deleted(deleted, 5).expect("deleted");
    let unreadable = old_image(&store, 6, &["JFIF"]);
    store
        .raw()
        .execute(
            "UPDATE item_formats SET inline_data = NULL WHERE item_id = ?1",
            [unreadable],
        )
        .expect("unreadable");
    assert_eq!(store.catch_up_jpeg().expect("nothing eligible"), 0);
    assert!(
        store
            .take_pending("ocr", 100, 10)
            .expect("queue")
            .is_empty()
    );
    assert!(
        store
            .take_pending("thumb", 100, 10)
            .expect("queue")
            .is_empty()
    );
}

#[test]
fn jpeg_recovery_rolls_back_its_jobs_when_the_completion_marker_cannot_be_saved() {
    let store = Store::in_memory().expect("schema");
    let id = old_image(&store, 1, &["JFIF"]);
    store
        .raw()
        .execute_batch(
            "CREATE TRIGGER reject_jpeg_marker BEFORE INSERT ON item_meta
         WHEN NEW.key = 'jpeg-enrichment-v1'
         BEGIN SELECT RAISE(ABORT, 'disk full'); END;",
        )
        .expect("trigger");
    assert!(store.catch_up_jpeg().is_err());
    for job in ["thumb", "ocr"] {
        assert!(store.take_pending(job, 100, 10).expect("queue").is_empty());
    }
    assert_eq!(store.meta(id, "jpeg-enrichment-v1").expect("marker"), None);
    store
        .raw()
        .execute_batch("DROP TRIGGER reject_jpeg_marker")
        .expect("writes restored");
    assert_eq!(store.catch_up_jpeg().expect("retried"), 1);
}

#[test]
fn what_has_no_group_yet_is_what_comes_back_to_be_grouped() {
    let store = Store::in_memory().expect("schema");
    let link = kept(&store, Kind::Link, "https://example.com/one", 1);
    let file = kept(&store, Kind::File, "C:/one.txt", 2);
    let folder = kept(&store, Kind::Folder, "C:/somewhere", 3);

    let waiting = store.ungrouped(10).expect("asked");

    let ids: Vec<i64> = waiting.iter().map(|(id, _, _)| *id).collect();
    assert_eq!(ids, vec![folder, file, link], "newest first");
    assert_eq!(
        waiting.first().map(|(_, kind, _)| *kind),
        Some(Some(Kind::Folder)),
        "the kind comes back so the caller knows how to group it"
    );
    assert_eq!(
        waiting.first().map(|(_, _, preview)| preview.as_str()),
        Some("C:/somewhere"),
        "and the text it has to be grouped by"
    );
}

#[test]
fn once_it_has_a_group_it_stops_coming_back() {
    let store = Store::in_memory().expect("schema");
    let link = kept(&store, Kind::Link, "https://example.com/one", 1);
    let other = kept(&store, Kind::Link, "https://example.com/two", 2);
    assert_eq!(store.ungrouped(10).expect("asked").len(), 2);

    store.set_group(link, "example.com").expect("grouped");

    assert_eq!(
        group_of(&store, link),
        "example.com",
        "the group the caller decided is the one that got written"
    );
    let ids: Vec<i64> = store
        .ungrouped(10)
        .expect("asked")
        .iter()
        .map(|(id, _, _)| *id)
        .collect();
    assert_eq!(ids, vec![other], "only the one still without a group");
    assert_eq!(group_of(&store, other), "", "and nobody else was touched");
}

#[test]
fn a_group_can_be_changed_and_can_be_taken_away() {
    let store = Store::in_memory().expect("schema");
    let link = kept(&store, Kind::Link, "https://example.com/one", 1);
    store.set_group(link, "example.com").expect("grouped");
    store.set_group(link, "elsewhere.org").expect("regrouped");
    assert_eq!(group_of(&store, link), "elsewhere.org");

    store.set_group(link, "").expect("ungrouped again");
    assert_eq!(group_of(&store, link), "");
    assert_eq!(
        store.ungrouped(10).expect("asked").len(),
        1,
        "emptying the group puts it back in the queue"
    );
}

#[test]
fn only_the_kinds_that_are_grouped_by_anything_come_back() {
    let store = Store::in_memory().expect("schema");
    kept(&store, Kind::Text, "just words", 1);
    kept(&store, Kind::Json, "{}", 2);
    kept(&store, Kind::Image, "", 3);
    let link = kept(&store, Kind::Link, "https://example.com/one", 4);

    let ids: Vec<i64> = store
        .ungrouped(10)
        .expect("asked")
        .iter()
        .map(|(id, _, _)| *id)
        .collect();
    assert_eq!(ids, vec![link], "text, json and images group by nothing");
}

#[test]
fn what_was_deleted_is_not_waiting_to_be_grouped() {
    let store = Store::in_memory().expect("schema");
    let link = kept(&store, Kind::Link, "https://example.com/one", 1);
    store.mark_deleted(link, 2).expect("deleted");
    assert!(store.ungrouped(10).expect("asked").is_empty());
}

#[test]
fn the_limit_is_honoured_and_asking_for_none_brings_none() {
    let store = Store::in_memory().expect("schema");
    for at in 1..=5 {
        kept(&store, Kind::Link, &format!("https://example.com/{at}"), at);
    }
    assert_eq!(store.ungrouped(2).expect("asked").len(), 2);
    assert_eq!(store.ungrouped(0).expect("asked").len(), 0);
    assert_eq!(store.ungrouped(99).expect("asked").len(), 5);
}

#[test]
fn setting_the_group_of_something_that_is_not_there_is_not_an_error() {
    let store = Store::in_memory().expect("schema");
    store
        .set_group(4_242, "nowhere")
        .expect("no row, no change");
    assert!(store.ungrouped(10).expect("asked").is_empty());
}

#[test]
fn the_thumbnail_of_one_item_is_read_once_it_is_written() {
    let store = Store::in_memory().expect("schema");
    let id = kept(&store, Kind::Image, "", 1);
    let other = kept(&store, Kind::Image, "", 2);
    assert_eq!(store.thumb_path(id).expect("read"), None);
    store.set_thumb(id, Some("/thumbs/1.png"), 3).expect("set");
    assert_eq!(
        store.thumb_path(id).expect("read").as_deref(),
        Some("/thumbs/1.png")
    );
    assert_eq!(store.thumb_path(other).expect("read"), None);
    assert_eq!(store.thumb_path(9_999).expect("read"), None);
}
