use cp_core::capture::Captured;
use cp_core::item::Item;
use cp_core::paste::Failure;
use cp_core::paste_as::Content;
use raw_window_handle::RawWindowHandle;
use std::path::{Path, PathBuf};

pub use platform::{
    Dragged, Listening, THUMBNAILS_FILES, Watching, activated, ahead_now, capture_insisting,
    content_of, data_dir, drag_out, dress, forward, front, ground, hand_back, holds_front, host,
    in_front, keep_awake, media_of, ocr_available, on_foreground, open_link, open_path,
    ours_up_front, paste_into, pointer, read_stuck, rebuilds_whole, screen_of, sequence,
    system_is_light, text_in, thumb_of_file, thumbs_dir, to_clipboard, towards, unreadable,
    utc_offset_at, watch_start, worn_sequence,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pointer {
    pub at: (f64, f64),
    pub area: crate::placing::Area,
    pub physical: bool,
    pub scale: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Landed {
    Nothing,
    Short { placed: usize, wanted: usize },
    Whole,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sent {
    Nobody,
    Done,
    Degraded(Failure),
}

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use cp_win_sys::clipboard::Clipboard;
    use std::time::Duration;

    pub use cp_core::thumbnail::THUMBNAILS_FILES;
    pub use cp_core::watching::Watching;
    pub fn watch_start(on_fresh: impl FnMut() + Send + 'static) -> Watching {
        cp_win::watching::every(cp_core::watching::EVERY, on_fresh)
    }

    pub fn sequence() -> Option<i64> {
        cp_win_sys::clipboard::sequence()
    }

    pub fn front() -> isize {
        cp_win_sys::frontmost::foreground().map_or(0, |window| window.0 as isize)
    }

    pub fn screen_of(handle: RawWindowHandle) -> Option<super::Pointer> {
        let RawWindowHandle::Win32(win32) = handle else {
            return None;
        };
        let spot = cp_win_sys::pointer::spot_of(win32.hwnd.get())?;
        Some(super::Pointer {
            at: (f64::from(spot.x), f64::from(spot.y)),
            area: crate::placing::Area {
                left: f64::from(spot.left),
                top: f64::from(spot.top),
                right: f64::from(spot.right),
                bottom: f64::from(spot.bottom),
            },
            physical: true,
            scale: spot.scale,
        })
    }

    pub fn holds_front(handle: RawWindowHandle) -> bool {
        let RawWindowHandle::Win32(win32) = handle else {
            return true;
        };
        cp_win_sys::frontmost::target_at(win32.hwnd.get())
            .is_some_and(|ours| cp_win_sys::frontmost::foreground() == Some(ours.window))
    }

    pub fn worn_sequence() -> Option<i64> {
        let sequence = cp_win_sys::clipboard::sequence()?;
        let ours = cp_win_sys::clipboard::holds(cp_win::restore::WORN);
        (ours && cp_win_sys::clipboard::sequence() == Some(sequence)).then_some(sequence)
    }

    pub fn rebuilds_whole(item: &Item) -> bool {
        cp_win::restore::rebuilt_whole(item)
    }

    pub type Listening = cp_win_sys::foreground::Listening;

    pub fn on_foreground(heard: impl Fn(isize) + 'static) -> Option<Listening> {
        cp_win_sys::foreground::listen(heard)
    }

    pub fn data_dir() -> Option<PathBuf> {
        cp_win_sys::paths::data_dir()
    }

    pub fn thumbs_dir() -> Option<PathBuf> {
        cp_win_sys::paths::thumbs_dir()
    }

    pub fn unreadable() -> Option<cp_core::capture::Unreadable> {
        None
    }

    pub struct Awake;

    pub fn keep_awake() -> Awake {
        Awake
    }

    pub fn capture_insisting() -> Captured {
        cp_win::capture::capture_insisting(cp_win::capture::PATIENCE, cp_core::watch::RETRY)
    }

    pub fn in_front() -> Option<String> {
        cp_win_sys::source::in_front()
    }

    pub fn open_path(path: &Path) -> bool {
        cp_win_sys::files::open(path)
    }

    pub fn open_link(url: &str) -> bool {
        cp_win_sys::files::open_link(url)
    }

    pub fn content_of<'a>(item: &'a Item, ocr: Option<&'a str>) -> Content<'a> {
        cp_win::content::content_of(item, ocr)
    }

    pub fn ocr_available() -> bool {
        cp_win_sys::ocr::is_available()
    }

    pub fn text_in(image: &[u8]) -> Option<String> {
        cp_win_sys::ocr::text_in(image)
    }

    pub fn thumb_of_file(path: &Path, side: i32) -> Option<Vec<u8>> {
        let dib = cp_win_sys::thumbnail::dib_of_file(path, side)?;
        cp_core::dib::to_png(&dib)
    }

    pub fn utc_offset_at(millis: i64) -> i64 {
        cp_win_sys::clock::utc_offset_at(millis)
    }

    pub fn towards(ahead: isize) -> crate::landing::Towards {
        cp_win_sys::source::described(ahead).map_or(
            crate::landing::Towards::Elsewhere,
            |(process, class)| {
                let hosts = cp_win_sys::source::hosts_a_pseudoconsole(ahead);
                crate::landing::towards_of(&process, &class, hosts)
            },
        )
    }

    pub fn to_clipboard(
        item: &Item,
        landing: &crate::landing::Landing,
        starting: impl FnOnce(),
        ours: impl FnOnce(),
        abandon: impl FnOnce(),
    ) -> Landed {
        let mut ready = cp_win::restore::ready_for(item);
        match landing.offer() {
            crate::landing::Offer::Files(paths) => ready.offer_files(&paths),
            crate::landing::Offer::Text(text) => ready.offer_text(&text),
            crate::landing::Offer::Nothing => {}
        }
        if landing.only_over.is_some() {
            ready.unrecorded();
        }
        starting();
        let Some(clipboard) = Clipboard::to_write() else {
            ours();
            return Landed::Nothing;
        };
        if let Some(over) = landing.only_over
            && cp_win_sys::clipboard::sequence() != Some(over)
        {
            drop(clipboard);
            abandon();
            return Landed::Nothing;
        }
        let landed = match cp_win::restore::place(&clipboard, &ready) {
            cp_win::restore::Restored::Written {
                formats,
                incomplete: true,
            } => {
                let (placed, wanted) = ready.fitted(formats);
                Landed::Short { placed, wanted }
            }
            cp_win::restore::Restored::Written { .. } => Landed::Whole,
            _ => Landed::Nothing,
        };
        drop(clipboard);
        ours();
        landed
    }

    pub fn read_stuck() -> bool {
        cp_win_sys::clipboard::read_stuck_for().is_some()
    }

    pub fn media_of(path: &Path) -> Vec<(&'static str, String)> {
        crate::media::said_of(
            cp_win_sys::media::info_for(path).map(|one| crate::media::Facts {
                duration: one.duration,
                width: one.width,
                height: one.height,
            }),
        )
    }

    pub fn system_is_light() -> bool {
        cp_win_sys::theme::wants_light().unwrap_or(false)
    }

    pub fn host() -> u32 {
        let ours = std::process::id();
        cp_win_sys::source::parent_of(ours).unwrap_or(ours)
    }

    pub fn ahead_now(host: u32) -> isize {
        cp_win_sys::frontmost::ahead_besides(&[std::process::id(), host])
    }

    pub fn ours_up_front() -> Option<bool> {
        cp_win_sys::frontmost::foreground().map(cp_win_sys::frontmost::process_of_is_ours)
    }

    pub fn dress(handle: RawWindowHandle, wanted: &str, light: bool) {
        let RawWindowHandle::Win32(win32) = handle else {
            return;
        };
        let backdrop = cp_win_sys::backdrop::Backdrop::from_name(wanted)
            .unwrap_or(cp_win_sys::backdrop::Backdrop::Mica);
        cp_win_sys::backdrop::apply(win32.hwnd.get(), backdrop, !light);
    }

    pub fn forward(handle: RawWindowHandle) {
        let RawWindowHandle::Win32(win32) = handle else {
            return;
        };
        let Some(target) = cp_win_sys::frontmost::target_at(win32.hwnd.get()) else {
            return;
        };
        if !cp_win_sys::frontmost::pull_forward(target.window) {
            crate::note::note(
                "Windows would not bring our window forward, so its keys stay behind",
            );
        }
    }

    pub use cp_win_sys::dragging::Dragged;

    pub fn drag_out(handle: RawWindowHandle, paths: &[&Path]) -> Dragged {
        let RawWindowHandle::Win32(win32) = handle else {
            return Dragged::Elsewhere;
        };
        cp_win_sys::dragging::from_window(win32.hwnd.get(), paths)
    }

    pub fn paste_into(ahead: isize, hide: impl FnOnce(), finished: impl FnOnce(Sent) + 'static) {
        let Some(target) = cp_win_sys::frontmost::target_at(ahead) else {
            return finished(Sent::Nobody);
        };
        match cp_win::paste::Pasting::start(target, hide) {
            Ok(mut pasting) => keep_pasting(move || pasting.advance(), finished, Duration::ZERO),
            Err(outcome) => finished(sent_of(outcome)),
        }
    }

    pub fn keep_pasting(
        mut advance: impl FnMut() -> cp_win::paste::Advance + 'static,
        finished: impl FnOnce(Sent) + 'static,
        after: Duration,
    ) {
        slint::Timer::single_shot(after, move || match advance() {
            cp_win::paste::Advance::Done(outcome) => finished(sent_of(outcome)),
            cp_win::paste::Advance::After(next) => keep_pasting(advance, finished, next),
        });
    }

    pub fn sent_of(outcome: cp_win::paste::Outcome) -> Sent {
        match outcome {
            cp_win::paste::Outcome::Sent { .. } => Sent::Done,
            cp_win::paste::Outcome::Degraded(why) => Sent::Degraded(why),
        }
    }

    pub fn activated() -> bool {
        ours_up_front() == Some(true)
    }

    pub fn hand_back(ahead: isize) {
        if let Some(target) = cp_win_sys::frontmost::target_at(ahead) {
            cp_win_sys::frontmost::bring_forward(target.window);
        }
    }

    pub fn ground(_handle: RawWindowHandle) {}

    pub fn pointer() -> Option<super::Pointer> {
        let spot = cp_win_sys::pointer::spot()?;
        Some(super::Pointer {
            at: (f64::from(spot.x), f64::from(spot.y)),
            area: crate::placing::Area {
                left: f64::from(spot.left),
                top: f64::from(spot.top),
                right: f64::from(spot.right),
                bottom: f64::from(spot.bottom),
            },
            physical: true,
            scale: spot.scale,
        })
    }
}

#[cfg(test)]
#[path = "here_test.rs"]
mod tests;
