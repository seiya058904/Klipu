use crate::aside::Leaving;
use crate::handing::{hand_over, paste_plain};
use crate::here;
use crate::landing::Towards;
use crate::model::{Metrics, Rows, reveal};
use crate::note::note;
use crate::opening::Reached;
use crate::reaching::reach_for;
use crate::showing::{NEXT_FRAME, appear, leave_when_left, vanish};
use crate::view::{
    chips_of, compact, count_text, empty_of, harvest, kinds_of_group, label_of_group,
};
use crate::{Chip, Options, Panel};
use cp_store::{Clock, Filter, Store};
use slint::{ComponentHandle, Model, ModelRc};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicIsize, AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

const CHIP_STEP: f32 = 78.0;
const KEPT_IN_VIEW: usize = 2;
const SETTLES: Duration = Duration::from_millis(70);
const RESTS: Duration = Duration::from_millis(110);
const CHINESE_FACE: &str = "Microsoft YaHei UI";
const LOOKS: Duration = Duration::from_millis(250);

#[derive(Clone)]
pub struct App {
    ui: slint::Weak<Panel>,
    state: Rc<RefCell<State>>,
}

struct State {
    store: Rc<Store>,
    engine: Option<Rc<crate::engine::Engine>>,
    dressing: Option<Rc<crate::dressing::Dressing<crate::handing::Shelves>>>,
    ahead: Arc<AtomicIsize>,
    query: String,
    tags: Vec<String>,
    pinned: bool,
    shown: Option<Search>,
    rows: Option<Rc<Rows>>,
    options: Options,
    metrics: Metrics,
    typing: slint::Timer,
    leaving: slint::Timer,
    pointing: slint::Timer,
    last_refresh: Duration,
    kept: crate::kept::Shelf,
    generation: Arc<AtomicU64>,
    counter: mpsc::Sender<Request>,
}

struct Request {
    generation: u64,
    base: Filter,
    full: Filter,
    keys: Vec<String>,
}

#[derive(PartialEq, Eq)]
struct Search {
    query: String,
    tags: Vec<String>,
    pinned: bool,
}

impl Search {
    fn of(state: &State) -> Self {
        Self {
            query: state.query.clone(),
            tags: state.tags.clone(),
            pinned: state.pinned,
        }
    }
}

impl App {
    pub fn start(store: Store, options: Options) -> Result<(Panel, Self), slint::PlatformError> {
        let panel = Panel::new()?;
        let kept = crate::kept::read();
        crate::say::adopt_english(kept.english);
        crate::view::dress_words(&panel);
        let theme = panel.global::<crate::Theme>();
        let metrics = Metrics {
            head: theme.get_row_head(),
            body_folder: theme.get_body_folder(),
            body_papers: theme.get_body_papers(),
            plain: theme.get_row_plain(),
            shut_lines: kept.look.density.shut_lines(),
        };
        let generation = Arc::new(AtomicU64::new(0));
        let counter = spawn_counter(options.db.clone(), panel.as_weak(), generation.clone());
        let state = Rc::new(RefCell::new(State {
            store: Rc::new(store),
            engine: None,
            dressing: None,
            ahead: Arc::new(AtomicIsize::new(0)),
            query: String::new(),
            tags: Vec::new(),
            pinned: false,
            shown: None,
            rows: None,
            options,
            metrics,
            typing: slint::Timer::default(),
            leaving: slint::Timer::default(),
            pointing: slint::Timer::default(),
            last_refresh: Duration::ZERO,
            kept: crate::kept::Shelf::new(kept),
            generation,
            counter,
        }));
        let app = Self {
            ui: panel.as_weak(),
            state,
        };
        app.wire(&panel);
        app.wire_quick(&panel);
        dress_theme(panel.global::<crate::Theme>(), kept);
        refresh(&panel, &app.state);
        Ok((panel, app))
    }

    pub fn close(&self) {
        let dressing = self.state.borrow().dressing.clone();
        if let Some(dressing) = dressing {
            dressing.put_back();
        }
        let engine = self.state.borrow().engine.clone();
        if let Some(engine) = engine {
            engine.close();
        }
    }

    pub fn run(&self, panel: &Panel) -> Result<(), slint::PlatformError> {
        let serving = self.state.borrow().options.serve;
        let _awake = serving.then(here::keep_awake);
        if !serving {
            panel.show()?;
            crate::showing::zoom(panel);
            self.dress(panel);
            appear(panel);
            panel.invoke_focus_search();
        }
        if !self.state.borrow().options.measure {
            self.keep_watch(panel.as_weak());
        }
        if let Some(dir) = self.state.borrow().options.signals.clone() {
            crate::orders::watch_signals(panel.as_weak(), dir, self.state.borrow().kept.clone());
        }
        if serving {
            let state = self.state.borrow();
            let shelf = state.kept.clone();
            leave_when_left(panel, move || shelf.get().hides);
            crate::orders::listen(
                panel.as_weak(),
                state.ahead.clone(),
                state.options.backdrop.clone(),
                state.kept.clone(),
            );
        }
        if self.state.borrow().options.measure {
            crate::measure::run(self);
        }
        slint::run_event_loop_until_quit()
    }

    fn keep_watch(&self, ui: slint::Weak<Panel>) {
        let db = self.state.borrow().options.db.clone();
        let drawn = ui.clone();
        let latest: crate::dressing::Latest = Arc::new(std::sync::Mutex::new(None));
        let noted = latest.clone();
        let arrived = move |_, copied| {
            if let Ok(mut held) = noted.lock() {
                *held = copied;
            }
            let _ = ui.upgrade_in_event_loop(|panel| {
                if panel.window().is_visible() {
                    panel.invoke_arrived();
                }
                panel.invoke_copied();
            });
        };
        let changed = move |id| {
            let _ = drawn.upgrade_in_event_loop(move |panel| {
                crate::model::renew_in(&panel.get_cards(), id);
            });
        };
        match crate::engine::Engine::start(&db, arrived, changed) {
            Ok(engine) => {
                let engine = Rc::new(engine);
                self.dress_terminals(&engine, latest);
                self.state.borrow_mut().engine = Some(engine);
            }
            Err(why) => crate::note::trouble(
                cp_core::trouble::Trouble::Unwatched,
                &format!("nothing watches the clipboard: {why}"),
            ),
        }
    }

    fn dress_terminals(&self, engine: &Rc<crate::engine::Engine>, latest: crate::dressing::Latest) {
        let store = self.state.borrow().store.clone();
        let shelves = crate::handing::Shelves {
            store,
            engine: engine.clone(),
        };
        let dressing = crate::dressing::Dressing::start(shelves, latest);
        if !dressing.listening() {
            note("nobody hears which window comes forward, so terminals get no image paths");
        }
        if let Some(panel) = self.ui.upgrade() {
            let heard = dressing.clone();
            panel.on_copied(move || heard.arrived());
        }
        self.state.borrow_mut().dressing = Some(dressing);
    }

    pub fn ui(&self) -> slint::Weak<Panel> {
        self.ui.clone()
    }

    pub fn last_refresh(&self) -> Duration {
        self.state.borrow().last_refresh
    }

    pub fn search(&self, query: &str) {
        self.state.borrow_mut().query = query.to_owned();
        self.refresh();
    }

    pub fn choose_chip(&self, key: &str) {
        pick_tag(&self.state, key, false);
        self.refresh();
    }

    fn wire_chrome(&self, panel: &Panel) {
        let (ui, state) = (self.ui.clone(), self.state.clone());
        panel.on_ask_settings(move || {
            let Some(ui) = ui.upgrade() else {
                return;
            };
            step_aside(&ui, &state, Leaving::Away);
            crate::note::tell("settings");
        });
        let ui = self.ui.clone();
        panel.on_nudge(move |dx, dy| {
            if let Some(ui) = ui.upgrade() {
                crate::showing::nudge(&ui, dx, dy);
            }
        });
    }

    fn wire_quick(&self, panel: &Panel) {
        let state = self.state.clone();
        let quick = crate::quick::Quick::new(
            Rc::new(move || {
                let state = state.borrow();
                crate::quick::Aim {
                    store: state.store.clone(),
                    engine: state.engine.clone(),
                }
            }),
            self.state.borrow().ahead.clone(),
        );
        panel.on_plain_asked(move || quick.plain());
    }

    fn wire(&self, panel: &Panel) {
        let ui = self.ui.clone();
        let state = self.state.clone();
        panel.on_search(move |query| {
            let (taken, rest) = harvest(query.as_str());
            {
                let mut state = state.borrow_mut();
                for tag in taken {
                    if !state.tags.contains(&tag) {
                        state.tags.push(tag);
                    }
                }
                state.query = rest.clone();
                state.generation.fetch_add(1, Ordering::SeqCst);
            }
            if let Some(ui) = ui.upgrade() {
                if rest != query.as_str() {
                    ui.set_query(rest.into());
                }
                let later = ui.as_weak();
                let mine = Rc::downgrade(&state);
                state
                    .borrow()
                    .typing
                    .start(slint::TimerMode::SingleShot, SETTLES, move || {
                        if let (Some(ui), Some(state)) = (later.upgrade(), mine.upgrade()) {
                            refresh(&ui, &state);
                        }
                    });
            }
        });
        let ui = self.ui.clone();
        let state = self.state.clone();
        panel.on_commit_search(move || {
            let pending = state.borrow().typing.running();
            if pending && let Some(ui) = ui.upgrade() {
                refresh(&ui, &state);
            }
        });
        let ui = self.ui.clone();
        let state = self.state.clone();
        panel.on_chip_chosen(move |key, adding| {
            pick_tag(&state, key.as_str(), adding);
            if let Some(ui) = ui.upgrade() {
                blink(&ui);
                refresh(&ui, &state);
            }
        });
        let ui = self.ui.clone();
        let state = self.state.clone();
        panel.on_clear_filters(move || {
            {
                let mut state = state.borrow_mut();
                state.query.clear();
                state.tags.clear();
                state.pinned = false;
            }
            if let Some(ui) = ui.upgrade() {
                ui.set_query(Default::default());
                blink(&ui);
                refresh(&ui, &state);
            }
        });
        let ui = self.ui.clone();
        let state = self.state.clone();
        panel.on_drop_tag(move |key| {
            {
                let mut state = state.borrow_mut();
                if key.is_empty() {
                    state.tags.pop();
                } else if let Some(at) = state.tags.iter().position(|one| one == key.as_str()) {
                    state.tags.remove(at);
                }
            }
            if let Some(ui) = ui.upgrade() {
                blink(&ui);
                refresh(&ui, &state);
            }
        });
        let ui = self.ui.clone();
        let state = self.state.clone();
        panel.on_pin_filter(move || {
            {
                let mut state = state.borrow_mut();
                state.pinned = !state.pinned;
            }
            if let Some(ui) = ui.upgrade() {
                ui.set_pinned_on(state.borrow().pinned);
                blink(&ui);
                refresh(&ui, &state);
            }
        });

        let ui = self.ui.clone();
        let state = self.state.clone();
        panel.on_cycle(move |delta| {
            let Some(ui) = ui.upgrade() else {
                return;
            };
            let chips = ui.get_chips();
            let mut keys: Vec<String> = vec![String::new()];
            keys.extend(
                (0..chips.row_count())
                    .filter_map(|index| chips.row_data(index).map(|chip| chip.key.to_string())),
            );
            let here = keys
                .iter()
                .position(|key| state.borrow().tags.first() == Some(key))
                .unwrap_or(0);
            let next = (here as i32 + delta).rem_euclid(keys.len() as i32) as usize;
            ui.set_chips_scroll(-(next.saturating_sub(KEPT_IN_VIEW + 1) as f32) * CHIP_STEP);
            {
                let mut state = state.borrow_mut();
                state.tags.clear();
                if !keys[next].is_empty() {
                    state.tags.push(keys[next].clone());
                }
            }
            blink(&ui);
            refresh(&ui, &state);
        });
        let ui = self.ui.clone();
        let state = self.state.clone();
        panel.on_pin(move |id, on| {
            let now = now_ms();
            if let Err(why) = state.borrow().store.set_pinned(i64::from(id), on, now) {
                note(&format!("{id} could not be pinned: {why}"));
            }
            if let Some(ui) = ui.upgrade() {
                keeping_place(&ui, &state);
            }
        });
        let ui = self.ui.clone();
        let state = self.state.clone();
        panel.on_remove(move |id| {
            if let Err(why) = state.borrow().store.mark_deleted(i64::from(id), now_ms()) {
                note(&format!("{id} could not be deleted: {why}"));
            }
            if let Some(ui) = ui.upgrade() {
                keeping_place(&ui, &state);
            }
        });
        let ui = self.ui.clone();
        let state = self.state.clone();
        panel.on_paste(move |id| {
            let (store, engine, towards) = aimed(&state);
            let handed = hand_over(&store, engine.as_deref(), i64::from(id), towards);
            if let Some(ui) = ui.upgrade() {
                if handed {
                    deliver(&ui, &state);
                } else {
                    complain(&ui, busy());
                }
            }
        });
        let ui = self.ui.clone();
        let state = self.state.clone();
        panel.on_copy(move |id| {
            let (store, engine) = {
                let state = state.borrow();
                (state.store.clone(), state.engine.clone())
            };
            let handed = hand_over(&store, engine.as_deref(), i64::from(id), Towards::Elsewhere);
            if let Some(ui) = ui.upgrade() {
                if handed {
                    following(&ui, &state, i64::from(id));
                    complain(&ui, crate::say::pick("已复制", "copied"));
                } else {
                    complain(&ui, busy());
                }
            }
        });
        let ui = self.ui.clone();
        let state = self.state.clone();
        panel.on_moved(move |index| {
            if index < 0 {
                return;
            }
            let Some(ui) = ui.upgrade() else {
                return;
            };
            ui.set_reveal_selection(true);
            let state = state.borrow();
            let Some(rows) = state.rows.as_ref() else {
                return;
            };
            rows.open_at(ui.get_opened().then_some(index as usize));
            let Some((top, span)) = span_of(&ui, rows, index as usize) else {
                return;
            };
            set_scroll(
                &ui,
                reveal(top, span, scroll_of(&ui), ui.get_viewport_height()),
            );
        });
        let ui = self.ui.clone();
        let state = self.state.clone();
        panel.on_stirred(move |index| {
            let Some(ui) = ui.upgrade() else {
                return;
            };
            if index < 0 {
                state.borrow().pointing.stop();
                ui.set_hovered(-1);
                return;
            }
            let later = ui.as_weak();
            state
                .borrow()
                .pointing
                .start(slint::TimerMode::SingleShot, RESTS, move || {
                    if let Some(ui) = later.upgrade() {
                        ui.set_hovered(index);
                    }
                });
        });
        let ui = self.ui.clone();
        let state = self.state.clone();
        panel.on_reopened(move || {
            if let Some(ui) = ui.upgrade() {
                keeping_place(&ui, &state);
            }
        });
        let ui = self.ui.clone();
        let state = self.state.clone();
        panel.on_fresh_start(move || {
            {
                let mut state = state.borrow_mut();
                state.query.clear();
                state.tags.clear();
                state.pinned = false;
            }
            if let Some(ui) = ui.upgrade() {
                ui.set_query(Default::default());
                ui.set_chips_scroll(0.0);
                let kept = state.borrow().kept.get();
                state.borrow_mut().metrics.shut_lines = kept.look.density.shut_lines();
                dress_theme(ui.global::<crate::Theme>(), kept);
                refresh(&ui, &state);
                back_to_the_newest(&ui, &state);
                watch_leaving(&ui, &state);
            }
        });
        let ui = self.ui.clone();
        let state = self.state.clone();
        panel.on_emptied(move |ticket| {
            let store = state.borrow().store.clone();
            match store.clear_all_unpinned(now_ms()) {
                Ok(gone) => {
                    note(&format!("{gone} unpinned items were emptied out"));
                    crate::note::tell(&format!("emptied {ticket} {gone}"));
                }
                Err(why) => {
                    note(&format!("the history could not be emptied: {why}"));
                    crate::note::tell(&format!("empty-failed {ticket}"));
                }
            }
            if let Some(ui) = ui.upgrade() {
                refresh(&ui, &state);
            }
        });
        let ui = self.ui.clone();
        let state = self.state.clone();
        panel.on_arrived(move || {
            if let Some(ui) = ui.upgrade() {
                keeping_place(&ui, &state);
            }
        });
        self.wire_chrome(panel);
        let ui = self.ui.clone();
        let state = self.state.clone();
        panel.on_paste_plain(move |id| {
            let (store, engine, _) = aimed(&state);
            let done = paste_plain(&store, engine.as_deref(), i64::from(id));
            if let Some(ui) = ui.upgrade() {
                match done {
                    Ok(()) => deliver(&ui, &state),
                    Err(refused) => complain(&ui, refused.said()),
                }
            }
        });
        let (ui, state) = (self.ui.clone(), self.state.clone());
        panel.on_dismiss(move || {
            if let Some(ui) = ui.upgrade() {
                step_aside(&ui, &state, Leaving::Back);
            }
        });
        self.wire_opening(panel);
        self.wire_dragging(panel);
        self.wire_naming(panel);
    }

    fn wire_opening(&self, panel: &Panel) {
        let ui = self.ui.clone();
        let state = self.state.clone();
        panel.on_open_asked(move |id| {
            let store = state.borrow().store.clone();
            let said = reach_for(&store, i64::from(id));
            let Some(ui) = ui.upgrade() else {
                return;
            };
            match said {
                Reached::Opened | Reached::Working => {
                    if said == Reached::Working {
                        note(&format!("{id} is being opened, slowly"));
                    }
                    step_aside(&ui, &state, Leaving::Away);
                }
                Reached::NoLink => complain(&ui, crate::excuse::cannot_open_link()),
                Reached::Missing => {
                    let now = now_ms();
                    if let Err(why) = state.borrow().store.mark_broken(i64::from(id), now) {
                        note(&format!("{id} could not be marked as gone: {why}"));
                    }
                    keeping_place(&ui, &state);
                    complain(&ui, crate::excuse::cannot_open());
                }
                Reached::Refused => complain(&ui, crate::excuse::cannot_open()),
            }
        });
    }

    fn wire_dragging(&self, panel: &Panel) {
        let ui = self.ui.clone();
        let state = self.state.clone();
        panel.on_dragged(move |id| {
            let Some(ui) = ui.upgrade() else {
                return;
            };
            let store = state.borrow().store.clone();
            let Ok(Some(item)) = store.item(i64::from(id)) else {
                note(&format!("{id} could not be looked up to drag it"));
                return;
            };
            let label = store.label_of(i64::from(id)).unwrap_or_default();
            let content = here::content_of(&item, None);
            let files = crate::dragging::files_for(
                item.kind,
                &content.paths,
                content.image,
                label.as_deref(),
                std::time::SystemTime::now(),
            );
            if files.is_empty() {
                note(&format!("{id} has no file another application could take"));
                return;
            }
            let Some(handle) = handle_of(ui.window()) else {
                note(&format!(
                    "{id} cannot be dragged: the panel has no native window"
                ));
                return;
            };
            let carried: Vec<&std::path::Path> = files.iter().map(|one| one.as_path()).collect();
            let went = here::drag_out(handle, &carried);
            if went != here::Dragged::Started {
                note(&format!("{id} was not dragged: {went:?}"));
            }
        });
    }

    fn wire_naming(&self, panel: &Panel) {
        let ui = self.ui.clone();
        let state = self.state.clone();
        panel.on_named(move |id, said| {
            let name = crate::view::name_worth_keeping(&said);
            let kept = state
                .borrow()
                .store
                .set_label(i64::from(id), name.as_deref(), now_ms());
            let landed = match &kept {
                Ok(landed) => *landed,
                Err(why) => {
                    note(&format!("{id} could not be named: {why}"));
                    false
                }
            };
            if kept.is_ok() && !landed {
                note(&format!("{id} was named but it is no longer in the store"));
            }
            if let Some(ui) = ui.upgrade() {
                ui.set_naming(-1);
                if !landed {
                    complain(&ui, crate::excuse::cannot_name());
                }
                keeping_place(&ui, &state);
            }
        });
    }

    fn refresh(&self) {
        if let Some(panel) = self.ui.upgrade() {
            refresh(&panel, &self.state);
        }
    }

    fn dress(&self, panel: &Panel) {
        let state = self.state.borrow();
        dress(
            panel,
            &state.options.backdrop,
            state.kept.get().light(here::system_is_light()),
        );
    }
}

pub fn handle_of(window: &slint::Window) -> Option<raw_window_handle::RawWindowHandle> {
    use raw_window_handle::HasWindowHandle;
    let handle = window.window_handle();
    HasWindowHandle::window_handle(&handle)
        .ok()
        .map(|raw| raw.as_raw())
}

pub fn dress(panel: &Panel, wanted: &str, light: bool) {
    if let Some(handle) = handle_of(panel.window()) {
        here::dress(handle, wanted, light);
    }
}

fn back_to_the_newest(ui: &Panel, state: &Rc<RefCell<State>>) {
    if let Some(rows) = state.borrow().rows.as_ref() {
        rows.open_at(None);
    }
    state.borrow().pointing.stop();
    ui.set_hovered(-1);
    ui.set_opened(false);
    ui.set_current(if ui.get_cards().row_count() > 0 {
        0
    } else {
        -1
    });
    ui.invoke_to_the_top();
}

fn following(ui: &Panel, state: &Rc<RefCell<State>>, id: i64) {
    refresh(ui, state);
    let Some(at) = state
        .borrow()
        .rows
        .as_ref()
        .and_then(|rows| rows.index_of(id))
        .and_then(|at| i32::try_from(at).ok())
    else {
        return;
    };
    ui.set_current(at);
    ui.invoke_moved(at);
}

fn keeping_place(ui: &Panel, state: &Rc<RefCell<State>>) {
    begin_refresh(ui, state, true);
}

fn refresh(ui: &Panel, state: &Rc<RefCell<State>>) {
    begin_refresh(ui, state, false);
}

fn scroll_of(ui: &Panel) -> f32 {
    if ui.get_layout() == "image" {
        ui.get_grid_scroll_y()
    } else {
        ui.get_scroll_y()
    }
}

fn set_scroll(ui: &Panel, scroll: f32) {
    if ui.get_layout() == "image" {
        ui.set_grid_scroll_y(scroll);
    } else {
        ui.set_scroll_y(scroll);
    }
}

fn span_of(ui: &Panel, rows: &Rows, index: usize) -> Option<(f32, f32)> {
    if ui.get_layout() == "image" {
        let height = ui.global::<crate::Theme>().get_cell();
        rows.id_at(index)
            .map(|_| ((index / 2) as f32 * height, height))
    } else {
        rows.span_of(index)
    }
}

struct Place {
    selected: Option<i64>,
    index: usize,
    loaded: usize,
    opened: bool,
    anchor: Option<(i64, f32)>,
}

impl Place {
    fn capture(ui: &Panel, rows: &Rows) -> Self {
        let scroll = scroll_of(ui);
        let index = ui.get_current().max(0) as usize;
        let top = if ui.get_layout() == "image" {
            Some((-scroll / ui.global::<crate::Theme>().get_cell()).floor() as usize * 2)
        } else {
            rows.index_at(-scroll)
        };
        let visible = i64::from(ui.get_viewport_first_id());
        let anchor = if rows.index_of(visible).is_some() {
            Some((visible, ui.get_viewport_first_offset()))
        } else {
            top.and_then(|top| Some((rows.id_at(top)?, span_of(ui, rows, top)?.0 + scroll)))
        };
        Self {
            selected: (ui.get_current() >= 0).then(|| rows.id_at(index)).flatten(),
            index,
            loaded: rows.loaded(),
            opened: ui.get_opened(),
            anchor,
        }
    }

    fn restore(self, ui: &Panel, rows: &Rows) {
        if rows.loaded() == 0 {
            ui.set_current(-1);
            ui.set_opened(false);
            set_scroll(ui, 0.0);
            return;
        }
        let retained = self.selected.and_then(|id| rows.index_of(id));
        let selected = retained.unwrap_or_else(|| self.index.min(rows.loaded() - 1));
        let opened = self.opened && retained.is_some();
        rows.open_at(opened.then_some(selected));
        ui.set_current(selected as i32);
        ui.set_opened(opened);
        if ui.get_layout() == "image"
            && let Some(scroll) = self
                .anchor
                .and_then(|(id, offset)| Some(offset - span_of(ui, rows, rows.index_of(id)?)?.0))
        {
            set_scroll(ui, scroll.min(0.0));
        }
    }
}

struct Refresh {
    rows: Rc<Rows>,
    request: Request,
    started: Instant,
    keep: bool,
    matching: std::collections::HashMap<i64, bool>,
}

fn begin_refresh(ui: &Panel, state: &Rc<RefCell<State>>, keep: bool) {
    let started = Instant::now();
    let now = now_ms();
    let (store, filter, base, metrics, keys, generation, keep) = {
        let state = state.borrow();
        state.typing.stop();
        state.pointing.stop();
        let filter = filter_of(&state);
        let search = Search::of(&state);
        let keep = keep && state.shown.as_ref() == Some(&search);
        (
            state.store.clone(),
            filter,
            base_filter_of(&state),
            state.metrics,
            state.tags.clone(),
            state.generation.fetch_add(1, Ordering::SeqCst) + 1,
            keep,
        )
    };
    ui.invoke_results_changed();
    let rows = Rows::open(
        store,
        filter.clone(),
        now,
        metrics,
        Some(crate::model::by_the_quarter(here::utc_offset_at)),
    );
    advance_refresh(
        ui,
        state,
        Refresh {
            rows,
            request: Request {
                generation,
                base,
                full: filter,
                keys,
            },
            started,
            keep,
            matching: Default::default(),
        },
    );
}

fn advance_refresh(ui: &Panel, state: &Rc<RefCell<State>>, mut next: Refresh) {
    if next.request.generation != state.borrow().generation.load(Ordering::SeqCst) {
        return;
    }
    let place = next
        .keep
        .then(|| {
            state
                .borrow()
                .rows
                .as_ref()
                .map(|rows| Place::capture(ui, rows))
        })
        .flatten();
    if let Some(place) = &place {
        let mut needs_more = next.rows.loaded() < place.loaded;
        for id in [place.selected, place.anchor.map(|(id, _)| id)]
            .into_iter()
            .flatten()
        {
            if next.rows.index_of(id).is_some() {
                continue;
            }
            let matches = *next.matching.entry(id).or_insert_with(|| {
                state
                    .borrow()
                    .store
                    .matches(&next.request.full, id)
                    .unwrap_or_else(|why| {
                        note(&format!(
                            "whether {id} still matches could not be read: {why}"
                        ));
                        false
                    })
            });
            needs_more |= matches;
        }
        if needs_more && next.rows.load_more() > 0 {
            let later = ui.as_weak();
            let mine = Rc::downgrade(state);
            slint::Timer::single_shot(Duration::from_millis(1), move || {
                if let (Some(ui), Some(state)) = (later.upgrade(), mine.upgrade()) {
                    advance_refresh(&ui, &state, next);
                }
            });
            return;
        }
    }
    finish_refresh(ui, state, next, place);
}

fn finish_refresh(ui: &Panel, state: &Rc<RefCell<State>>, next: Refresh, place: Option<Place>) {
    let rows = if place.is_some() {
        let rows = state.borrow().rows.clone().expect("the list being kept");
        rows.replace_from(&next.rows);
        rows
    } else {
        next.rows
    };
    let layout = crate::layout::layout_for(&next.request.full.kinds);
    ui.set_layout(layout.as_str().into());
    let mut kinds = next.request.full.kinds.clone();
    kinds.dedup();
    ui.set_one_kind(kinds.len() == 1);
    state.borrow().pointing.stop();
    ui.set_reveal_selection(place.is_none());
    ui.invoke_results_changed();
    ui.set_opened(false);
    {
        let state = state.borrow();
        ui.set_filtered(!state.query.trim().is_empty() || !state.tags.is_empty() || state.pinned);
        ui.set_tags(ModelRc::from(Rc::new(slint::VecModel::from(tags_of(
            &state,
        )))));
    }
    if rows.loaded() == 0 {
        let (title, hint) = {
            let state = state.borrow();
            empty_of(&state.query, state.pinned, !state.tags.is_empty())
        };
        ui.set_empty_title(title.into());
        ui.set_empty_hint(hint.into());
    }
    if let Some(place) = place {
        place.restore(ui, &rows);
    } else {
        ui.set_current(if rows.loaded() > 0 { 0 } else { -1 });
        ui.set_scroll_y(0.0);
        ui.set_cards(ModelRc::from(rows.clone()));
        ui.set_grid_lines(ModelRc::from(crate::paired::Paired::over(rows.clone())));
        ui.set_grid_scroll_y(0.0);
    }
    let mut state = state.borrow_mut();
    state.rows = Some(rows);
    state.shown = Some(Search::of(&state));
    state.last_refresh = next.started.elapsed();
    let _ = state.counter.send(next.request);
    if state.options.measure {
        eprintln!(
            "refresh «{}»: list {:.1} ms",
            state.query,
            state.last_refresh.as_secs_f64() * 1000.0
        );
    }
}

fn spawn_counter(
    db: std::path::PathBuf,
    ui: slint::Weak<Panel>,
    generation: Arc<AtomicU64>,
) -> mpsc::Sender<Request> {
    let (tx, rx) = mpsc::channel::<Request>();
    let spawned = std::thread::Builder::new()
        .name(crate::note::COUNTER.to_owned())
        .spawn(move || {
            let store = match Store::open(&db) {
                Ok(store) => store,
                Err(why) => {
                    note(&format!("the counter could not open the store: {why}"));
                    return;
                }
            };
            while let Ok(mut request) = rx.recv() {
                while let Ok(newer) = rx.try_recv() {
                    request = newer;
                }
                if request.generation != generation.load(Ordering::SeqCst) {
                    continue;
                }
                let pinned = store
                    .count_matching(&Filter {
                        pinned_only: true,
                        ..request.base.clone()
                    })
                    .unwrap_or(0);
                let facets = store.facets(&request.base).unwrap_or_default();
                let shown = store.count_matching(&request.full).unwrap_or(0);
                if request.generation != generation.load(Ordering::SeqCst) {
                    continue;
                }
                let chips = chips_of(&facets, &request.keys);
                let footer = count_text(shown);
                let anchored = compact(pinned);
                let only_anchored = request.full.pinned_only;
                let mine = request.generation;
                let clock = generation.clone();
                let _ = ui.upgrade_in_event_loop(move |panel| {
                    if mine != clock.load(Ordering::SeqCst) {
                        return;
                    }
                    panel.set_chips(ModelRc::from(Rc::new(slint::VecModel::from(chips))));
                    panel.set_count_text(footer.into());
                    panel.set_complaining(false);
                    panel.set_pinned_count(anchored.into());
                    panel.set_pinned_on(only_anchored);
                });
            }
        });
    if let Err(why) = spawned {
        note(&format!("nobody counts what the history holds: {why}"));
    }
    tx
}

fn written_filter(state: &State) -> Filter {
    let now = now_ms();
    let clock = Clock {
        now,
        day_start: now - now.rem_euclid(cp_store::query::DAY),
    };
    let mut written = cp_store::parse(&state.query, &clock);
    if written.broken == cp_store::Broken::Hidden {
        written.broken = cp_store::Broken::Shown;
    }
    written
}

fn base_filter_of(state: &State) -> Filter {
    let mut filter = written_filter(state);
    filter.kinds.clear();
    filter
}

fn filter_of(state: &State) -> Filter {
    let mut filter = written_filter(state);
    for tag in &state.tags {
        filter.kinds.extend_from_slice(kinds_of_group(tag));
    }
    if state.pinned {
        filter.pinned_only = true;
    }
    filter
}

fn pick_tag(state: &Rc<RefCell<State>>, key: &str, adding: bool) {
    let next = crate::tags::after(&state.borrow().tags, key, adding);
    state.borrow_mut().tags = next;
}

fn tags_of(state: &State) -> Vec<Chip> {
    state
        .tags
        .iter()
        .map(|tag| Chip {
            key: tag.as_str().into(),
            label: label_of_group(tag).into(),
            count: Default::default(),
            selected: true,
        })
        .collect()
}

type Aimed = (Rc<Store>, Option<Rc<crate::engine::Engine>>, Towards);

fn aimed(state: &Rc<RefCell<State>>) -> Aimed {
    let state = state.borrow();
    let towards = here::towards(state.ahead.load(Ordering::Relaxed));
    (state.store.clone(), state.engine.clone(), towards)
}

pub fn busy() -> &'static str {
    if crate::here::read_stuck() {
        return crate::say::pick(
            "有应用在剪贴板操作中无响应，请重启 Klipu",
            "an app stopped answering about what it copied; restart Klipu",
        );
    }
    crate::say::pick(
        "无法粘贴：剪贴板被占用",
        "could not paste: the clipboard is busy",
    )
}
fn complain(ui: &Panel, said: &str) {
    ui.set_count_text(said.into());
    ui.set_complaining(true);
    let weak = ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(2_200), move || {
        if let Some(ui) = weak.upgrade() {
            ui.set_complaining(false);
            ui.invoke_reopened();
        }
    });
}

fn blink(ui: &Panel) {
    ui.set_fade(0.35);
    let weak = ui.as_weak();
    slint::Timer::single_shot(NEXT_FRAME, move || {
        if let Some(ui) = weak.upgrade() {
            ui.set_fade(1.0);
        }
    });
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn deliver(ui: &Panel, state: &Rc<RefCell<State>>) {
    let ahead = state.borrow().ahead.swap(0, Ordering::Relaxed);
    let (weak, later, state) = (ui.as_weak(), ui.as_weak(), state.clone());
    let hide = move || {
        if let Some(ui) = weak.upgrade() {
            let _ = ui.hide();
        }
    };
    let paste = move || {
        here::paste_into(ahead, hide, move |sent| {
            let Some(ui) = later.upgrade() else {
                return;
            };
            match sent {
                here::Sent::Nobody => {
                    note("no window was left to paste into, it stays on the clipboard");
                    vanish(&ui);
                }
                here::Sent::Done => {}
                here::Sent::Degraded(why) => {
                    let _ = state.borrow().ahead.compare_exchange(
                        0,
                        ahead,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    );
                    let said = crate::excuse::why_not(why, crate::say::in_english());
                    note(&format!(
                        "it stays on the clipboard, unpasted: {why:?}: {said}"
                    ));
                    if ui.show().is_ok() {
                        forward(&ui);
                        appear(&ui);
                        watch_leaving(&ui, &state);
                        complain(&ui, said);
                    }
                }
            }
        })
    };
    slint::Timer::single_shot(Duration::ZERO, paste);
}

fn step_aside(ui: &Panel, state: &Rc<RefCell<State>>, leaving: Leaving) {
    let back = crate::aside::handed_back(&state.borrow().ahead, leaving, here::activated);
    let _ = ui.hide();
    if let Some(ahead) = back {
        here::hand_back(ahead);
    }
}

pub fn forward(panel: &Panel) {
    if let Some(handle) = handle_of(panel.window()) {
        here::forward(handle);
    }
}

pub fn dress_theme(theme: crate::Theme<'_>, kept: crate::kept::Kept) {
    let (light, look) = (kept.light(here::system_is_light()), kept.look);
    theme.set_light(light);
    let swatch = look.accent.swatch(light);
    theme.set_accent(opaque(swatch.accent));
    theme.set_accent_dim(opaque(swatch.dim));
    theme.set_focus(opaque(swatch.dim));
    theme.set_selected(opaque(swatch.selected));
    theme.set_selected_edge(opaque(swatch.edge));
    theme.set_grip(opaque(swatch.edge));
    theme.set_font(typeface(kept).into());
    theme.set_mono_font(look.mono.into());
    theme.set_zoom(look.size.zoom());
}

fn typeface(kept: crate::kept::Kept) -> &'static str {
    let plain = cp_config::look::fonts_here().text.family;
    if !kept.english && kept.look.font == plain {
        return CHINESE_FACE;
    }
    kept.look.font
}

fn opaque(rgb: u32) -> slint::Color {
    slint::Color::from_argb_encoded(0xFF00_0000 | rgb)
}

fn watch_leaving(ui: &Panel, state: &Rc<RefCell<State>>) {
    let held = state.borrow();
    held.leaving.stop();
    if !held.kept.get().hides {
        return;
    }
    let weak = ui.as_weak();
    let mine = Rc::downgrade(state);
    let mut was_ours = false;
    held.leaving
        .start(slint::TimerMode::Repeated, LOOKS, move || {
            let (Some(ui), Some(state)) = (weak.upgrade(), mine.upgrade()) else {
                return;
            };
            if !ui.window().is_visible() {
                state.borrow().leaving.stop();
                return;
            }
            match here::ours_up_front() {
                Some(true) => {
                    was_ours = true;
                    return;
                }
                None => return,
                Some(false) => {}
            }
            if was_ours {
                state.borrow().leaving.stop();
                vanish(&ui);
            }
        });
}

#[cfg(test)]
#[path = "app_test.rs"]
mod tests;
