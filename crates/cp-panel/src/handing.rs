use crate::here;
use crate::landing::{Landing, Towards};
use crate::note::note;
use cp_store::Store;

fn landing_for(store: &Store, item: &cp_core::item::Item, id: i64, towards: Towards) -> Landing {
    if towards == Towards::Elsewhere {
        return Landing::anywhere();
    }
    let label = (towards == Towards::Browser)
        .then(|| store.label_of(id).ok().flatten())
        .flatten();
    let content = here::content_of(item, None);
    let files = crate::dragging::files_for(
        item.kind,
        &content.paths,
        content.image,
        label.as_deref(),
        std::time::SystemTime::now(),
    );
    Landing {
        towards,
        files,
        only_over: None,
    }
}

pub fn hand_over(
    store: &Store,
    engine: Option<&crate::engine::Engine>,
    id: i64,
    towards: Towards,
) -> bool {
    let item = match store.item(id) {
        Ok(Some(item)) => item,
        Ok(None) => {
            note(&format!("pasting {id}: it is no longer in the store"));
            return false;
        }
        Err(why) => {
            note(&format!("pasting {id}: {why}"));
            return false;
        }
    };
    let landing = landing_for(store, &item, id, towards);
    let written = short_of(id, written_over(&item, &landing, engine));
    if written {
        if let Err(why) = store.record_paste(id, crate::app::now_ms()) {
            note(&format!("{id} was pasted and nobody wrote it down: {why}"));
        }
    } else {
        note(&format!(
            "pasting {id}: the write never reached the clipboard"
        ));
    }
    written
}

fn written_over(
    item: &cp_core::item::Item,
    landing: &Landing,
    engine: Option<&crate::engine::Engine>,
) -> here::Landed {
    here::to_clipboard(
        item,
        landing,
        || starting(engine),
        || mark(engine),
        || abandoned(engine),
    )
}

fn worn_over(
    store: &Store,
    engine: &crate::engine::Engine,
    id: i64,
    towards: Towards,
    over: i64,
) -> Option<i64> {
    let item = store.item(id).ok().flatten()?;
    if !here::rebuilds_whole(&item) {
        note(&format!(
            "{id} carries more than its picture, so a terminal gets no path for it"
        ));
        return None;
    }
    let landing = Landing {
        only_over: Some(over),
        ..landing_for(store, &item, id, towards)
    };
    if towards == Towards::Terminal && !matches!(landing.offer(), crate::landing::Offer::Text(_)) {
        note(&format!(
            "{id} has no file a terminal could take, so it gets no path"
        ));
        return None;
    }
    let landed = written_over(&item, &landing, Some(engine));
    if landed == here::Landed::Nothing {
        return None;
    }
    here::worn_sequence()
}

pub struct Shelves {
    pub store: std::rc::Rc<Store>,
    pub engine: std::rc::Rc<crate::engine::Engine>,
}

impl crate::dressing::Closet for Shelves {
    fn place(&self, id: i64, towards: Towards, over: i64) -> Option<i64> {
        worn_over(&self.store, &self.engine, id, towards, over)
    }

    fn sequence(&self) -> Option<i64> {
        here::sequence()
    }

    fn front(&self) -> isize {
        here::front()
    }

    fn terminal(&self, window: isize) -> bool {
        here::towards(window) == Towards::Terminal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    Textless,
    Failed,
}

impl Refused {
    pub fn said(self) -> &'static str {
        match self {
            Refused::Textless => crate::say::pick(
                "no tiene texto para pegarlo en plano",
                "it has no text to paste as plain",
            ),
            Refused::Failed => crate::app::busy(),
        }
    }
}

pub fn paste_plain(
    store: &Store,
    engine: Option<&crate::engine::Engine>,
    id: i64,
) -> Result<(), Refused> {
    let Ok(Some(item)) = store.item(id) else {
        return Err(Refused::Failed);
    };
    let ocr = store.ocr_text(id).ok().flatten();
    let content = here::content_of(&item, ocr.as_deref());
    let Some(text) = cp_core::paste_as::plainest(&content) else {
        note(&format!("pasting {id} as plain text: it has no text"));
        return Err(Refused::Textless);
    };
    let made = cp_core::item::Item::plain(&text);
    let written = short_of(id, written_over(&made, &Landing::anywhere(), engine));
    if !written {
        return Err(Refused::Failed);
    }
    if let Err(why) = store.record_paste(id, crate::app::now_ms()) {
        note(&format!("{id} was pasted and nobody wrote it down: {why}"));
    }
    Ok(())
}

pub fn newest(store: &Store, back: usize) -> Option<(cp_store::Listed, bool)> {
    match store.list(&cp_store::Filter::default(), back + 2, None) {
        Ok(page) => {
            let older = page.rows.len() > back + 1;
            page.rows.into_iter().nth(back).map(|row| (row, older))
        }
        Err(why) => {
            note(&format!(
                "the newest in the history could not be read: {why}"
            ));
            None
        }
    }
}

fn short_of(id: i64, landed: here::Landed) -> bool {
    if let here::Landed::Short { placed, wanted } = landed {
        note(&format!(
            "pasting {id}: only {placed} of {wanted} formats fitted on the clipboard"
        ));
    }
    landed != here::Landed::Nothing
}

fn starting(engine: Option<&crate::engine::Engine>) {
    let Some(engine) = engine else {
        return;
    };
    if !engine.writing() {
        note("the start of the clipboard write could not be marked as ours");
    }
}

fn abandoned(engine: Option<&crate::engine::Engine>) {
    if let Some(engine) = engine
        && !engine.abandoned()
    {
        note("a write given up could not be handed back to the watcher");
    }
}

fn mark(engine: Option<&crate::engine::Engine>) {
    let Some(engine) = engine else {
        return;
    };
    if !engine.ours() {
        note("the clipboard write could not be marked as ours");
    }
}
