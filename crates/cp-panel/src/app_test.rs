use super::*;
use slint::platform::{Key, WindowEvent};

const KEPT: crate::kept::Kept = crate::kept::Kept {
    english: false,
    theme: cp_config::Theme::Dark,
    hides: true,
    look: crate::kept::Look {
        size: cp_config::TextSize::Normal,
        density: cp_config::Density::Normal,
        accent: cp_config::Accent::Indigo,
        font: "Segoe UI",
        mono: "Consolas",
    },
};

thread_local! {
    static PLATFORM: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

struct Bench {
    panel: Panel,
    app: App,
    _dir: tempfile::TempDir,
}

fn bench(count: usize) -> Bench {
    if !PLATFORM.replace(true) {
        i_slint_backend_testing::init_no_event_loop();
    }
    let dir = tempfile::tempdir().expect("a temporary history");
    let db = dir.path().join("history.db");
    let store = Store::open(&db).expect("a store");
    for index in 0..count {
        let text = match index {
            0 => "alpha".to_owned(),
            1 => "beta needle".to_owned(),
            2 => "charlie".to_owned(),
            _ => format!("clip number {index}"),
        };
        store
            .insert_text(&format!("clip-{index}"), &text, index as i64 + 1)
            .expect("a clip");
    }
    let (panel, app) = App::start_with_kept(
        store,
        Options {
            db,
            measure: false,
            serve: false,
            signals: None,
            backdrop: "none".into(),
        },
        KEPT,
    )
    .expect("the test panel");
    panel
        .window()
        .set_size(slint::LogicalSize::new(480.0, 620.0));
    panel.set_gliding(false);
    panel.show().expect("the testing window");
    panel.invoke_focus_search();
    settle();
    Bench {
        panel,
        app,
        _dir: dir,
    }
}

#[test]
fn the_panel_uses_the_injected_settings_without_adopting_host_preferences() {
    let bench = bench(3);
    let state = bench.app.state.borrow();
    assert_eq!(state.kept.get(), KEPT);
    assert_eq!(state.metrics.shut_lines, KEPT.look.density.shut_lines());
    assert!(!bench.panel.global::<crate::Theme>().get_light());
    assert_eq!(
        bench.panel.global::<crate::Theme>().get_zoom(),
        KEPT.look.size.zoom()
    );
    assert!(!crate::say::in_english());
}

fn rows(bench: &Bench) -> Rc<Rows> {
    bench.app.state.borrow().rows.clone().expect("a list")
}

fn selected(bench: &Bench) -> Option<i64> {
    let index = usize::try_from(bench.panel.get_current()).ok()?;
    rows(bench).id_at(index)
}

fn tick(millis: u64) {
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(millis));
}

fn settle() {
    for _ in 0..10 {
        tick(10);
    }
}

fn press(panel: &Panel, key: Key) {
    panel
        .window()
        .dispatch_event(WindowEvent::KeyPressed { text: key.into() });
    panel
        .window()
        .dispatch_event(WindowEvent::KeyReleased { text: key.into() });
}

fn enter(panel: &Panel, plain: bool) {
    if plain {
        panel.window().dispatch_event(WindowEvent::KeyPressed {
            text: Key::Shift.into(),
        });
    }
    press(panel, Key::Return);
    if plain {
        panel.window().dispatch_event(WindowEvent::KeyReleased {
            text: Key::Shift.into(),
        });
    }
}

fn insert_newest(bench: &Bench) {
    bench
        .app
        .state
        .borrow()
        .store
        .insert_text("newest", "newly arrived", 10_000)
        .expect("a new clip");
    bench.panel.invoke_arrived();
}

#[test]
fn an_arrival_keeps_the_selected_item_and_the_visible_anchor() {
    let bench = bench(20);
    bench.panel.set_current(6);
    bench.panel.invoke_moved(6);
    tick(1);
    let top = rows(&bench).span_of(5).expect("a visible row").0;
    bench.panel.set_scroll_y(-top - 7.0);
    settle();
    let before = Place::capture(&bench.panel, &rows(&bench));
    bench.panel.set_hovered(8);
    insert_newest(&bench);
    settle();
    let after = Place::capture(&bench.panel, &rows(&bench));
    assert_eq!(after.selected, before.selected);
    assert_eq!(after.anchor, before.anchor);
    assert_eq!(bench.panel.get_current(), 7);
    assert_eq!(bench.panel.get_hovered(), -1);
}

#[test]
fn naming_and_pinning_deep_in_history_keep_the_page_item_and_expansion() {
    let bench = bench(300);
    rows(&bench).load_more();
    bench.panel.set_current(179);
    bench.panel.set_opened(true);
    bench.panel.invoke_moved(179);
    tick(1);
    let before = Place::capture(&bench.panel, &rows(&bench));
    let id = before.selected.expect("the selected item");
    bench.panel.invoke_named(id as i32, "a useful name".into());
    settle();
    let renamed = Place::capture(&bench.panel, &rows(&bench));
    assert_eq!(selected(&bench), Some(id));
    assert!(renamed.loaded >= before.loaded);
    assert!(renamed.opened);
    assert_eq!(renamed.anchor, before.anchor);
    bench.panel.invoke_pin(id as i32, true);
    settle();
    assert_eq!(selected(&bench), Some(id));
    assert!(rows(&bench).loaded() >= before.loaded);
    assert!(bench.panel.get_opened());
}

#[test]
fn a_page_end_selection_is_found_on_the_next_page_after_an_arrival() {
    let bench = bench(300);
    bench.panel.set_current(crate::model::PAGE as i32 - 1);
    let before = selected(&bench);
    insert_newest(&bench);
    settle();
    assert_eq!(selected(&bench), before);
    assert_eq!(bench.panel.get_current(), crate::model::PAGE as i32);
}

#[test]
fn removing_the_selection_chooses_a_neighbor_until_the_list_is_empty() {
    let bench = bench(3);
    bench.panel.set_current(1);
    bench.panel.invoke_remove(2);
    settle();
    assert_eq!(selected(&bench), Some(1));
    bench.panel.invoke_remove(1);
    settle();
    assert_eq!(selected(&bench), Some(3));
    bench.panel.invoke_remove(3);
    settle();
    assert_eq!(selected(&bench), None);
    assert_eq!(bench.panel.get_current(), -1);
    assert_eq!(bench.panel.get_scroll_y(), 0.0);
}

#[test]
fn unpinning_in_the_pinned_filter_selects_the_neighbor_and_handles_no_results() {
    let bench = bench(3);
    for id in [1, 2] {
        bench
            .app
            .state
            .borrow()
            .store
            .set_pinned(id, true, 20)
            .expect("pinned");
    }
    bench.panel.invoke_pin_filter();
    assert_eq!(selected(&bench), Some(2));
    bench.panel.invoke_pin(2, false);
    settle();
    assert_eq!(selected(&bench), Some(1));
    bench.panel.invoke_pin(1, false);
    settle();
    assert_eq!(selected(&bench), None);
    assert_eq!(bench.panel.get_current(), -1);
}

#[test]
fn enter_and_plain_enter_commit_pending_search_before_resolving_the_item() {
    for plain in [false, true] {
        for delay in [0, 69, 70, 71] {
            let bench = bench(3);
            let activated = Rc::new(RefCell::new(Vec::new()));
            let pasted = activated.clone();
            bench
                .panel
                .on_paste(move |id| pasted.borrow_mut().push((id, false)));
            let pasted = activated.clone();
            bench
                .panel
                .on_paste_plain(move |id| pasted.borrow_mut().push((id, true)));
            bench.panel.set_hovered(2);
            bench.panel.set_query("needle".into());
            bench.panel.invoke_search("needle".into());
            tick(delay);
            enter(&bench.panel, plain);
            assert_eq!(
                *activated.borrow(),
                [(2, plain)],
                "plain={plain}, delay={delay}"
            );
            assert!(!bench.app.state.borrow().typing.running());
            let generation = bench.app.state.borrow().generation.load(Ordering::SeqCst);
            settle();
            assert_eq!(
                bench.app.state.borrow().generation.load(Ordering::SeqCst),
                generation
            );
            assert_eq!(selected(&bench), Some(2));
        }
    }
}

#[test]
fn enter_does_not_activate_an_old_result_when_the_pending_query_matches_nothing() {
    for plain in [false, true] {
        let bench = bench(3);
        let activated = Rc::new(RefCell::new(Vec::new()));
        let pasted = activated.clone();
        bench.panel.on_paste(move |id| pasted.borrow_mut().push(id));
        let pasted = activated.clone();
        bench
            .panel
            .on_paste_plain(move |id| pasted.borrow_mut().push(id));
        bench.panel.set_query("zzzzzzzzxxxx".into());
        bench.panel.invoke_search("zzzzzzzzxxxx".into());
        enter(&bench.panel, plain);
        assert!(activated.borrow().is_empty());
        assert_eq!(bench.panel.get_current(), -1);
    }
}

#[test]
fn a_new_query_supersedes_an_unfinished_restoration() {
    let bench = bench(360);
    rows(&bench).load_more();
    bench.panel.set_current(179);
    insert_newest(&bench);
    let activated = Rc::new(RefCell::new(Vec::new()));
    let pasted = activated.clone();
    bench.panel.on_paste(move |id| pasted.borrow_mut().push(id));
    bench.panel.set_query("needle".into());
    bench.panel.invoke_search("needle".into());
    enter(&bench.panel, false);
    settle();
    assert_eq!(*activated.borrow(), [2]);
    assert_eq!(selected(&bench), Some(2));
    assert_eq!(rows(&bench).loaded(), 1);
}

#[test]
fn navigation_during_a_restore_keeps_the_users_new_selection() {
    let bench = bench(360);
    rows(&bench).load_more();
    bench.panel.set_current(179);
    insert_newest(&bench);
    bench.panel.set_current(180);
    bench.panel.invoke_moved(180);
    let wanted = selected(&bench);
    settle();
    assert_eq!(selected(&bench), wanted);
}

#[test]
fn relative_time_searches_keep_the_selection_when_the_clock_moves() {
    let bench = bench(300);
    bench
        .app
        .state
        .borrow()
        .store
        .raw()
        .execute("UPDATE items SET modified_at = ?1", [now_ms()])
        .expect("recent history");
    bench.app.search("~1d");
    rows(&bench).load_more();
    bench.panel.set_current(179);
    let selected = selected(&bench).expect("a deep result");
    let earlier = filter_of(&bench.app.state.borrow()).since;
    std::thread::sleep(Duration::from_millis(2));
    assert_ne!(filter_of(&bench.app.state.borrow()).since, earlier);
    bench
        .panel
        .invoke_named(selected as i32, "still here".into());
    settle();
    assert_eq!(
        rows(&bench).id_at(bench.panel.get_current() as usize),
        Some(selected)
    );
    assert!(rows(&bench).loaded() >= 240);
}

#[test]
fn an_image_arrival_keeps_the_selected_image_and_the_visible_grid_row() {
    let bench = bench(0);
    let item = cp_core::item::Item {
        kind: Some(cp_core::kind::Kind::Image),
        formats: vec![cp_core::item::Format {
            id: "public.png".into(),
            payload: cp_core::item::Payload::Inline(vec![1, 2, 3]),
        }],
    };
    for index in 0..20 {
        bench
            .app
            .state
            .borrow()
            .store
            .insert_item(&format!("picture-{index}"), &item, "", index)
            .expect("a picture");
    }
    bench.panel.invoke_chip_chosen("image".into(), false);
    settle();
    bench.panel.set_current(8);
    bench.panel.invoke_moved(8);
    settle();
    let before = Place::capture(&bench.panel, &rows(&bench));
    bench
        .app
        .state
        .borrow()
        .store
        .insert_item("new-picture", &item, "", 100)
        .expect("a picture");
    bench.panel.invoke_arrived();
    settle();
    let after = Place::capture(&bench.panel, &rows(&bench));
    assert_eq!(after.selected, before.selected);
    let (anchor, offset) = before.anchor.expect("a visible grid row");
    let (current_anchor, current_offset) = after.anchor.expect("the grid stayed visible");
    assert_eq!(
        rows(&bench).index_of(anchor).expect("the anchor") / 2,
        rows(&bench)
            .index_of(current_anchor)
            .expect("the visible row")
            / 2
    );
    assert_eq!(current_offset, offset);
}

#[test]
fn deep_image_results_survive_removal_and_continue_paging_in_pairs() {
    let bench = bench(300);
    bench
        .app
        .state
        .borrow()
        .store
        .raw()
        .execute("UPDATE items SET kind = 'image'", [])
        .expect("image cards");
    bench.panel.invoke_chip_chosen("image".into(), false);
    rows(&bench).load_more();
    bench.panel.set_current(179);
    bench.panel.invoke_moved(179);
    settle();
    let old = selected(&bench).expect("an image");
    let neighbor = rows(&bench).id_at(180);
    bench.panel.invoke_remove(old as i32);
    settle();
    assert_eq!(selected(&bench), neighbor);
    assert!(rows(&bench).loaded() >= 240);
    rows(&bench).load_more();
    settle();
    assert_eq!(rows(&bench).loaded(), 299);
    assert_eq!(bench.panel.get_grid_lines().row_count(), 150);
    assert_eq!(bench.panel.get_grid_lines().row_data(149), Some(149));
    assert!(bench.panel.get_cards().row_data(298).is_some());
}
