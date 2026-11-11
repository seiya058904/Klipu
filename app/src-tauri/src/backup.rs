use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::Emitter;

#[derive(Default)]
pub struct Crossing {
    busy: Arc<AtomicBool>,
    prepared: Mutex<Option<Waiting>>,
}

struct Waiting {
    token: String,
    name: String,
    prepared: cp_store::backup::Prepared,
    preview: cp_store::backup::Preview,
}

pub struct Alone(Arc<AtomicBool>);

impl Crossing {
    pub(crate) fn claim(&self) -> Option<Alone> {
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
            .then(|| Alone(self.busy.clone()))
    }

    #[cfg(test)]
    pub fn underway(&self) -> bool {
        self.busy.load(Ordering::Acquire)
    }
}

impl Drop for Alone {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[derive(serde::Serialize)]
pub struct Saved {
    path: String,
    items: i64,
    missing: i64,
    bytes: u64,
}

#[derive(serde::Serialize)]
pub struct Brought {
    added: i64,
    already: i64,
    refused: i64,
    partial: i64,
    repaired: i64,
    #[serde(rename = "fromElsewhere")]
    from_elsewhere: bool,
}

#[tauri::command(async)]
pub fn save_backup(
    alone: tauri::State<'_, Crossing>,
    path: String,
    at: i64,
) -> Result<Saved, String> {
    let _busy = alone.claim().ok_or_else(busy)?;
    let store = crate::settings::open_history()?;
    let to = PathBuf::from(&path);
    let made = cp_store::backup::write(&store, &to, at).map_err(|why| why.to_string())?;
    crate::note::note(&format!("a copy written to {path}: {} items", made.items));
    Ok(Saved {
        path,
        items: made.items,
        missing: made.missing,
        bytes: made.bytes,
    })
}

#[derive(serde::Serialize)]
pub struct Review {
    token: String,
    name: String,
    #[serde(flatten)]
    preview: cp_store::backup::Preview,
}

impl Waiting {
    fn review(&self) -> Review {
        Review {
            token: self.token.clone(),
            name: self.name.clone(),
            preview: self.preview,
        }
    }
}

fn busy() -> String {
    "the history is already being written to".to_owned()
}

fn policy() -> Result<cp_store::Policy, String> {
    let kept = crate::settings::settings()?;
    Ok(cp_store::Policy::keeping(
        kept.keeps_days,
        kept.images_quota_mb,
    ))
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as i64)
}

#[tauri::command(async)]
pub fn peek_backup(alone: tauri::State<'_, Crossing>, path: String) -> Result<Review, String> {
    let _busy = alone.claim().ok_or_else(busy)?;
    let mut held = alone.prepared.lock().map_err(|_| busy())?;
    held.take();
    let from = PathBuf::from(path);
    let prepared = cp_store::backup::Prepared::open(&from).map_err(|why| why.to_string())?;
    let preview = prepared
        .preview(&crate::settings::open_history()?, &policy()?, now())
        .map_err(|why| why.to_string())?;
    static TURN: AtomicU64 = AtomicU64::new(0);
    let waiting = Waiting {
        token: format!(
            "{}-{}",
            std::process::id(),
            TURN.fetch_add(1, Ordering::Relaxed)
        ),
        name: from
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        prepared,
        preview,
    };
    let review = waiting.review();
    *held = Some(waiting);
    Ok(review)
}

#[tauri::command(async)]
pub fn cancel_backup(alone: tauri::State<'_, Crossing>, token: String) -> Result<(), String> {
    let mut held = alone.prepared.lock().map_err(|_| busy())?;
    if held.as_ref().is_some_and(|waiting| waiting.token == token) {
        held.take();
    }
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Loaded {
    Review { review: Review },
    Done { result: Brought },
}

#[tauri::command(async)]
pub fn load_backup(
    app: tauri::AppHandle,
    alone: tauri::State<'_, Crossing>,
    token: String,
) -> Result<Loaded, String> {
    let _busy = alone.inner().claim().ok_or_else(busy)?;
    let mut held = alone.prepared.lock().map_err(|_| busy())?;
    if !held.as_ref().is_some_and(|waiting| waiting.token == token) {
        return Err("the preview has expired; select the backup again".into());
    }
    let mut waiting = held.take().expect("the preview was just checked");
    crate::panel::quit(&app);
    let landed = (|| {
        let store = crate::settings::open_history()?;
        let at = now();
        let preview = waiting
            .prepared
            .preview(&store, &policy()?, at)
            .map_err(|why| why.to_string())?;
        if preview.needs_review_after(&waiting.preview) {
            waiting.preview = preview;
            let review = waiting.review();
            *held = Some(waiting);
            return Ok(Loaded::Review { review });
        }
        let brought = waiting
            .prepared
            .bring(&store, at, &|what| crate::note::note(what))
            .map_err(|why| why.to_string())?;
        crate::note::note(&format!(
            "{} items imported, {} already present, {} incomplete and {} refused",
            brought.added, brought.already, brought.partial, brought.refused
        ));
        let _ = app.emit("crossed", ());
        Ok(Loaded::Done {
            result: Brought {
                added: brought.added,
                already: brought.already,
                refused: brought.refused,
                partial: brought.partial,
                repaired: brought.repaired,
                from_elsewhere: brought.from_elsewhere,
            },
        })
    })();
    if let Err(why) = crate::panel::relight(&app) {
        crate::note::note(&format!(
            "the panel did not come back after the copy: {why}"
        ));
    }
    landed
}

#[tauri::command(async)]
pub fn repair_search(
    app: tauri::AppHandle,
    alone: tauri::State<'_, Crossing>,
) -> Result<usize, String> {
    let _busy = alone.claim().ok_or_else(busy)?;
    let repaired = crate::settings::open_history()?
        .repair_search()
        .map_err(|why| why.to_string())?;
    let _ = app.emit("crossed", ());
    Ok(repaired)
}

#[cfg(test)]
#[path = "backup_test.rs"]
mod tests;
