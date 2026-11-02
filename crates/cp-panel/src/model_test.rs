use super::*;

const SIZES: Metrics = Metrics {
    body_folder: 36.0,
    body_papers: 40.0,
    head: 23.0,
    plain: 86.0,
    shut_lines: 2,
};

fn store_with(count: usize) -> Rc<Store> {
    let store = Store::in_memory().expect("esquema");
    for at in 0..count {
        store
            .insert_text(&format!("u{at}"), &format!("elemento {at}"), at as i64)
            .expect("insert");
    }
    Rc::new(store)
}

fn open(store: Rc<Store>, now: i64) -> Rc<Rows> {
    Rows::open(store, Filter::default(), now, SIZES, None)
}

#[test]
fn the_first_page_comes_with_the_model_and_the_rest_waits_for_the_view() {
    let rows = open(store_with(300), 1_000);
    assert_eq!(rows.row_count(), PAGE);
    assert_eq!(rows.loaded(), PAGE);
    assert_eq!(rows.row_data(0).map(|card| card.id), Some(300));
    let more = rows.load_page();
    assert_eq!(more, PAGE);
    assert_eq!(rows.row_count(), 2 * PAGE);
    assert_eq!(rows.load_page(), 60);
    assert_eq!(rows.load_page(), 0, "there are no more pages");
    assert_eq!(rows.row_count(), 300);
}

#[test]
fn a_row_is_built_once_and_read_back_from_the_cache() {
    let rows = open(store_with(3), 5_000);
    let first = rows.row_data(0).expect("fila");
    assert_eq!(first.body.as_str(), "elemento 2");
    assert_eq!(first.age.as_str(), "刚刚");
    assert!(rows.cards.borrow()[0].is_some());
    let again = rows.row_data(0).expect("from the cache");
    assert_eq!((again.id, again.body.as_str()), (first.id, "elemento 2"));
    assert_eq!(rows.row_data(9).map(|card| card.id), None);
}

#[test]
fn the_next_page_is_wanted_only_near_the_end_and_only_once() {
    let rows = open(store_with(300), 0);
    assert!(!rows.needs_more(0));
    assert!(!rows.needs_more(PAGE - AHEAD - 1));
    assert!(rows.needs_more(PAGE - AHEAD));
    assert!(rows.needs_more(PAGE - 1));
    rows.loading.set(true);
    assert!(!rows.needs_more(PAGE - 1), "a page is already on its way");
    rows.loading.set(false);
    while rows.load_page() > 0 {}
    assert!(!rows.needs_more(299), "no queda nada que pedir");
}

#[test]
fn a_row_written_from_the_view_is_what_comes_back() {
    let rows = open(store_with(2), 0);
    let mut card = rows.row_data(1).expect("fila");
    card.body = "editado".into();
    rows.set_row_data(1, card);
    assert_eq!(
        rows.row_data(1).map(|c| c.body.to_string()),
        Some("editado".into())
    );
    rows.set_row_data(9, Card::default());
    assert_eq!(rows.row_count(), 2, "fuera de rango no crea filas");
}

#[test]
fn a_query_that_matches_nothing_is_an_empty_model_not_an_error() {
    let filter = Filter {
        query: Some("nada-de-esto".into()),
        ..Default::default()
    };
    let rows = Rows::open(store_with(10), filter, 0, SIZES, None);
    assert_eq!(rows.row_count(), 0);
    assert!(rows.exhausted.get());
}

#[test]
fn the_open_row_grows_what_its_text_asks_and_nobody_else_pays() {
    use crate::face::{Face, LINE, open_px};
    let store = Store::in_memory().expect("esquema");
    store.insert_text("u0", "corto", 0).expect("insert");
    store
        .insert_text("u1", &"palabra ".repeat(40), 1)
        .expect("insert");
    store.insert_text("u2", "otro corto", 2).expect("insert");
    let rows = open(Rc::new(store), 0);
    let short = Face::Words(1).shut_px();
    let two = Face::Words(2).shut_px();
    let long = open_px(6.0 * LINE);

    assert_eq!(rows.span_of(1), Some((short, two)));

    rows.open_at(Some(1));
    assert_eq!(
        rows.span_of(0),
        Some((0.0, short)),
        "la de arriba no se mueve"
    );
    assert_eq!(
        rows.span_of(1),
        Some((short, long)),
        "crece lo que pide su texto"
    );
    assert_eq!(
        rows.span_of(2),
        Some((short + long, short)),
        "the one below drops by what the open one grew"
    );

    rows.open_at(Some(2));
    assert_eq!(rows.span_of(1), Some((short, two)), "la anterior vuelve");
    assert_eq!(
        rows.span_of(2),
        Some((short + two, open_px(LINE))),
        "incluso un texto corto gana la fila de atajos al abrirse"
    );

    rows.open_at(None);
    assert_eq!(rows.span_of(1), Some((short, two)));
}

#[test]
fn a_row_in_the_mixed_list_is_as_tall_as_its_face() {
    let store = store_with(2);
    store.set_thumb(2, Some("miniatura.png"), 1).expect("thumb");
    let rows = open(store, 0);
    assert_eq!(
        rows.span_of(0),
        Some((0.0, crate::face::Face::Thumb.shut_px())),
        "the row with a thumbnail makes room for the strip it draws"
    );
    assert_eq!(
        rows.span_of(1).map(|(_, span)| span),
        Some(crate::face::Face::Words(1).shut_px()),
        "a short text takes the one line it needs"
    );
}

#[test]
fn revealing_a_row_only_scrolls_when_the_row_is_out_of_sight() {
    let viewport = 400.0;
    assert_eq!(reveal(0.0, 124.0, 0.0, viewport), 0.0, "ya se ve");
    assert_eq!(
        reveal(124.0, 124.0, -124.0, viewport),
        -124.0,
        "arriba del todo"
    );
    assert_eq!(
        reveal(124.0, 124.0, -300.0, viewport),
        -116.0,
        "queda encima: baja con un respiro"
    );
    assert_eq!(
        reveal(4.0, 124.0, -20.0, viewport),
        0.0,
        "arriba del todo no se pasa de largo"
    );
    assert_eq!(
        reveal(500.0, 124.0, 0.0, viewport),
        -232.0,
        "queda debajo: sube lo justo"
    );
    assert_eq!(reveal(500.0, 124.0, 0.0, 0.0), 0.0, "sin alto no se decide");
}

fn grouped(group: &str) -> cp_store::Listed {
    cp_store::Listed {
        id: 1,
        modified_at: 0,
        created_at: 0,
        kind: Some(cp_core::kind::Kind::Link),
        preview: String::new(),
        app: None,
        label: None,
        color: 0,
        thumb_path: None,
        paste_count: 0,
        last_used_at: None,
        broken_since: None,
        pinned: false,
        group: group.to_owned(),
        snippet: None,
    }
}

#[test]
fn a_view_that_does_not_group_draws_no_heading_at_all() {
    let general = Filter::default();
    let rows = [grouped("docs.rs"), grouped("github.com")];
    assert!(!heads_group(&general, &rows, 0));
    assert!(!heads_group(&general, &rows, 1));
}

#[test]
fn choosing_the_newest_way_drops_the_headings_with_the_grouping() {
    let newest = Filter {
        kinds: vec![cp_core::kind::Kind::Link],
        order: cp_store::Order::Recent,
        ..Default::default()
    };
    let rows = [grouped("docs.rs"), grouped("github.com")];
    assert!(
        !heads_group(&newest, &rows, 0),
        "a heading over a list that is not grouped would be a lie"
    );
}

#[test]
fn a_group_nobody_could_name_heads_nothing() {
    let linked = Filter {
        kinds: vec![cp_core::kind::Kind::Link],
        order: cp_store::Order::ByGroup,
        ..Default::default()
    };
    let rows = [grouped(crate::group::UNKNOWN), grouped("")];
    assert!(!heads_group(&linked, &rows, 0));
    assert!(!heads_group(&linked, &rows, 1));
}

#[test]
fn the_row_that_heads_a_group_is_taller_by_exactly_its_heading() {
    let store = Store::in_memory().expect("esquema");
    for (at, group) in ["a.test", "a.test", "b.test"].iter().enumerate() {
        let id = store
            .insert_text(
                &format!("u{at}"),
                &format!("https://{group}/{at}"),
                at as i64,
            )
            .expect("insert");
        store.set_group(id, group).expect("grouped");
    }
    let grouped = Filter {
        order: cp_store::Order::ByGroup,
        ..Default::default()
    };
    let rows = Rows::open(Rc::new(store), grouped, 1_000, SIZES, None);
    assert_eq!(rows.row_count(), 3);
    let mut heading = 0;
    for at in 0..rows.row_count() {
        let card = rows.row_data(at).expect("a card");
        let span = rows.span_of(at).expect("a row").1;
        let wanted = if card.heads_group {
            heading += 1;
            crate::face::Face::Link.shut_px() + SIZES.head
        } else {
            crate::face::Face::Link.shut_px()
        };
        assert_eq!(
            span, wanted,
            "row {at} draws its heading as {} and measures {span}",
            card.heads_group
        );
    }
    assert_eq!(heading, 2, "two groups, two headings");
}

#[test]
fn a_filter_on_one_kind_still_comes_back_grouped() {
    let store = Store::in_memory().expect("esquema");
    for at in 0..3 {
        let id = store
            .insert_text(
                &format!("u{at}"),
                &format!("https://una.test/{at}"),
                at as i64,
            )
            .expect("insert");
        store.set_group(id, "una.test").expect("grouped");
    }
    let linked = Filter {
        kinds: vec![cp_core::kind::Kind::Link],
        order: cp_store::Order::ByGroup,
        ..Default::default()
    };
    let rows = Rows::open(Rc::new(store), linked, 1_000, SIZES, None);
    assert_eq!(rows.row_count(), 3, "the three links came back");
    let first = rows.row_data(0).expect("a card");
    assert!(first.heads_group, "the first link heads its domain");
    assert_eq!(first.group_said.as_str(), "una.test");
    let second = rows.row_data(1).expect("a card");
    assert!(!second.heads_group, "same domain, no second heading");
}

fn grouped_store(groups: &[(&str, usize)]) -> Rc<Store> {
    let store = Store::in_memory().expect("esquema");
    let mut at = 0i64;
    for (key, how_many) in groups {
        for _ in 0..*how_many {
            at += 1;
            let id = store
                .insert_text(&format!("u{at}"), &format!("https://{key}/path/{at}"), at)
                .expect("insert");
            store.set_group(id, key).expect("grouped");
        }
    }
    Rc::new(store)
}

fn by_group() -> Filter {
    Filter {
        order: cp_store::Order::ByGroup,
        ..Default::default()
    }
}

#[test]
fn exactly_one_card_per_group_carries_the_heading() {
    let store = grouped_store(&[("a.com", 3), ("b.com", 2), ("c.com", 4)]);
    let rows = Rows::open(store, by_group(), 1_000, SIZES, None);
    let mut heading_at = Vec::new();
    for index in 0..rows.row_count() {
        if rows.row_data(index).expect("a card").heads_group {
            heading_at.push(index);
        }
    }
    assert_eq!(
        heading_at.len(),
        3,
        "three groups, three headings, at {heading_at:?}"
    );
}

#[test]
fn the_room_a_heading_row_gets_is_the_card_plus_the_heading() {
    let store = grouped_store(&[("a.com", 2), ("b.com", 2)]);
    let rows = Rows::open(store, by_group(), 1_000, SIZES, None);
    let mut with_heading = None;
    let mut without = None;
    for index in 0..rows.row_count() {
        let card = rows.row_data(index).expect("a card");
        let (_, span) = rows.span_of(index).expect("a span");
        if card.heads_group {
            with_heading.get_or_insert(span);
        } else {
            without.get_or_insert(span);
        }
    }
    let (with_heading, without) = (
        with_heading.expect("one heads a group"),
        without.expect("one does not"),
    );
    assert_eq!(
        with_heading - without,
        SIZES.head,
        "the row that carries the heading gets exactly one heading more of room"
    );
}

#[test]
fn a_thumbnail_opened_in_the_mixed_list_grows_like_everything_else_there() {
    let store = store_with(2);
    store.set_thumb(2, Some("miniatura.png"), 1).expect("thumb");
    let rows = open(store, 0);
    let shut = rows.span_of(0).expect("la fila").1;
    assert_eq!(shut, crate::face::Face::Thumb.shut_px());
    rows.open_at(Some(0));
    let open = rows.span_of(0).expect("la fila").1;
    assert!(
        open > shut,
        "opening a thumbnail must move the rows below: \
         the model said {open} and the delegate draws it grown"
    );
    assert_eq!(
        open,
        crate::face::open_px(crate::face::OPEN_THUMB + crate::face::GAP + crate::face::NOTE),
        "opening a picture is for looking at it, so the row makes room for the picture the \
         card draws, and the scrolling follows this number"
    );
}

#[test]
fn a_card_carries_the_two_heights_the_model_believes() {
    let rows = open(store_with(3), 0);
    let card = rows.row_data(0).expect("una tarjeta");
    assert_eq!(
        card.shut_px,
        rows.span_of(0).expect("la fila").1,
        "the delegate draws with this number, so it has to be the one the model reserved"
    );
    assert!(card.open_px > 0.0);
}

#[test]
fn a_card_used_again_is_found_where_it_went_and_not_where_it_was() {
    let store = store_with(5);
    let before = open(store.clone(), 100);
    let used = before.rows.borrow()[3].id;
    let neighbour = before.rows.borrow()[2].id;
    store.record_paste(used, 50).expect("used");

    let after = open(store, 100);

    assert_eq!(after.index_of(used), Some(0));
    assert_eq!(after.index_of(neighbour), Some(3));
    assert_eq!(after.index_of(-1), None);
}

#[test]
fn the_general_list_is_grouped_by_when_it_was_copied() {
    let noon = 1_791_201_600_000;
    let store = Store::in_memory().expect("esquema");
    for (at, when) in [
        ("u0", noon - 60_000),
        ("u1", noon - 5 * 3_600_000),
        ("u2", noon - 6 * 3_600_000),
        ("u3", noon - 20 * 3_600_000),
        ("u4", noon - 40 * 86_400_000),
    ] {
        store.insert_text(at, at, when).expect("insert");
    }
    let rows = Rows::open(
        Rc::new(store),
        Filter::default(),
        noon,
        SIZES,
        Some(Rc::new(|_| 0)),
    );
    let cards: Vec<_> = (0..rows.row_count())
        .map(|at| rows.row_data(at).expect("card"))
        .collect();
    let heads: Vec<(bool, String)> = cards
        .iter()
        .map(|card| (card.heads_group, card.group_said.to_string()))
        .collect();
    assert_eq!(
        heads,
        vec![
            (true, "刚刚".to_owned()),
            (true, "今天".to_owned()),
            (false, String::new()),
            (true, "昨天".to_owned()),
            (true, "更早".to_owned()),
        ]
    );
    assert_eq!(cards[1].age, "07:00", "inside today the age is the clock");
    assert_eq!(
        rows.span_of(2).map(|(_, span)| span),
        Some(crate::face::Face::Words(1).shut_px()),
        "only the first of a group pays for the heading"
    );
}

#[test]
fn a_list_of_pictures_wears_the_card_of_the_general_view() {
    let store = Store::in_memory().expect("esquema");
    let item = cp_core::item::Item {
        kind: Some(cp_core::kind::Kind::Image),
        formats: vec![cp_core::item::Format {
            id: "public.png".into(),
            payload: cp_core::item::Payload::Inline(vec![1; 8]),
        }],
    };
    store.insert_item("u0", &item, "", 1).expect("insert");
    store.set_thumb(1, Some("miniatura.png"), 1).expect("thumb");
    let filter = Filter {
        kinds: vec![cp_core::kind::Kind::Image],
        ..Default::default()
    };
    let rows = Rows::open(Rc::new(store), filter, 2, SIZES, None);
    assert_eq!(
        rows.span_of(0),
        Some((0.0, crate::face::Face::Thumb.shut_px()))
    );
}

#[test]
fn a_card_whose_thumbnail_will_not_load_opens_with_the_text_it_falls_back_to() {
    let store = store_with(1);
    store.set_thumb(1, Some("no-existe.png"), 1).expect("thumb");
    let rows = open(store, 0);
    let card = rows.row_data(0).expect("card");
    assert!(!card.has_thumb);
    assert_eq!(card.face, "words");
    assert!(
        card.open_lines > 0 && !card.opened.is_empty(),
        "the open card draws its text, not an empty box of the height the model kept"
    );
}

#[test]
fn a_card_whose_file_is_gone_opens_without_the_keys_to_paste_it() {
    let store = Store::in_memory().expect("esquema");
    let id = store
        .insert_text("u0", "/no/existe.txt", 1)
        .expect("insert");
    store.mark_broken(id, 1).expect("broken");
    let shown = Filter {
        broken: cp_store::Broken::Shown,
        ..Default::default()
    };
    let rows = Rows::open(Rc::new(store), shown, 2, SIZES, None);
    rows.open_at(Some(0));
    let open = rows.span_of(0).expect("la fila").1;
    let card = rows.row_data(0).expect("card");
    assert!(card.broken);
    let body = crate::face::LINE * card.open_lines as f32;
    assert_eq!(open, crate::face::open_lost_px(body));
    assert_eq!(
        crate::face::open_px(body) - open,
        crate::face::GAP + crate::face::KEYS_ROW,
        "the row of paste keys is not drawn, so it is not kept either"
    );
}

static ASKED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn shifting_clock(millis: i64) -> i64 {
    if millis < 0 {
        -3 * 3_600_000
    } else {
        -4 * 3_600_000
    }
}

fn counted_clock(millis: i64) -> i64 {
    ASKED.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    shifting_clock(millis)
}

#[test]
fn the_offset_is_asked_once_per_quarter_of_an_hour() {
    let clock = by_the_quarter(counted_clock);
    let start = ASKED.load(std::sync::atomic::Ordering::SeqCst);
    for minute in 0..15 {
        assert_eq!(clock(1_000 + minute * 60_000), -4 * 3_600_000);
    }
    assert_eq!(
        ASKED.load(std::sync::atomic::Ordering::SeqCst) - start,
        1,
        "fifteen rows in one quarter ask once"
    );
    assert_eq!(clock(15 * 60_000 + 1), -4 * 3_600_000);
    assert_eq!(
        ASKED.load(std::sync::atomic::Ordering::SeqCst) - start,
        2,
        "the next quarter asks again"
    );
}

#[test]
fn a_change_of_offset_on_the_quarter_is_never_blurred() {
    let clock = by_the_quarter(shifting_clock);
    assert_eq!(
        clock(-1),
        -3 * 3_600_000,
        "the instant before keeps the old offset"
    );
    assert_eq!(
        clock(0),
        -4 * 3_600_000,
        "the instant of the change takes the new one"
    );
}

fn store_with_a_picture() -> (Rc<Store>, i64) {
    let store = Store::in_memory().expect("esquema");
    store
        .insert_text("before", "words before", 1)
        .expect("insert");
    let item = cp_core::item::Item {
        kind: Some(cp_core::kind::Kind::Image),
        formats: vec![cp_core::item::Format {
            id: "public.png".into(),
            payload: cp_core::item::Payload::Inline(vec![1, 2, 3]),
        }],
    };
    let id = store.insert_item("picture", &item, "", 2).expect("insert");
    (Rc::new(store), id)
}

fn png_in(dir: &std::path::Path) -> String {
    let path = dir.join("drawn.png");
    image::RgbaImage::from_pixel(8, 8, image::Rgba([12, 200, 140, 255]))
        .save(&path)
        .expect("drawn");
    path.to_string_lossy().into_owned()
}

#[test]
fn renewing_what_is_not_listed_or_not_rows_changes_nothing() {
    let (store, _) = store_with_a_picture();
    let rows = open(store, 10);
    assert!(!rows.renew(9_999));
    let other: slint::ModelRc<Card> =
        slint::ModelRc::new(slint::VecModel::from(vec![Card::default()]));
    assert!(!renew_in(&other, 1));
}

#[test]
fn an_open_card_that_is_renewed_keeps_its_open_height() {
    let (store, id) = store_with_a_picture();
    let rows = open(store.clone(), 10);
    let at = rows.index_of(id).expect("listed");
    rows.open_at(Some(at));
    let opened = rows.span_of(at).expect("a span");
    let dir = tempfile::tempdir().expect("a folder");
    store
        .set_thumb(id, Some(&png_in(dir.path())), 3)
        .expect("thumb");
    assert!(rows.renew(id));
    assert_eq!(rows.span_of(at), Some(opened));
}
