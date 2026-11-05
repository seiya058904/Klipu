use super::*;
use crate::drop::drop_of;
use crate::virtual_files::descriptor_of;
use cp_store::store::Store;
use std::cell::Cell;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, MutexGuard};

static REAL_CLIPBOARD: Mutex<()> = Mutex::new(());

fn the_real_clipboard_alone() -> MutexGuard<'static, ()> {
    REAL_CLIPBOARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn awaiting(name: &str, size: Option<u64>) -> Item {
    Item {
        kind: Some(Kind::File),
        formats: vec![
            Format {
                id: DESCRIPTOR.into(),
                payload: Payload::Inline(descriptor_of(&[(name, size, false)])),
            },
            Format {
                id: virtual_files::contents_id(0),
                payload: Payload::Absent,
            },
        ],
    }
}

fn with_bytes(mut item: Item, bytes: &[u8]) -> Item {
    item.formats[1].payload = Payload::stored(bytes.to_vec());
    item
}

#[test]
fn a_name_and_bytes_of_one_copy_are_kept() {
    let got = of_one_generation(
        awaiting("a.txt", None),
        Some(7),
        || Some(7),
        |item| with_bytes(item, b"of a"),
    );
    assert_eq!(
        got,
        Captured::Kept(with_bytes(awaiting("a.txt", None), b"of a"))
    );
}

#[test]
fn a_copy_made_after_the_lock_closed_is_refused_before_its_bytes_are_asked_for() {
    let asked = Cell::new(false);
    let got = of_one_generation(
        awaiting("a.txt", None),
        Some(7),
        || Some(8),
        |item| {
            asked.set(true);
            with_bytes(item, b"of b")
        },
    );
    assert_eq!(got, Captured::Superseded);
    assert!(!asked.get(), "the bytes of another copy are never read");
}

#[test]
fn a_copy_made_during_the_read_of_the_bytes_is_refused_after_it() {
    let now = Cell::new(7);
    let got = of_one_generation(
        awaiting("a.txt", None),
        Some(7),
        || Some(now.get()),
        |item| {
            now.set(8);
            with_bytes(item, b"half of a, half of b")
        },
    );
    assert_eq!(got, Captured::Superseded);
}

#[test]
fn an_unknown_generation_proves_nothing_and_is_refused() {
    let read = |item| with_bytes(item, b"of who knows");
    assert_eq!(
        of_one_generation(awaiting("a.txt", None), None, || None, read),
        Captured::Superseded,
        "two unknowns are not the same copy"
    );
    assert_eq!(
        of_one_generation(awaiting("a.txt", None), Some(7), || None, read),
        Captured::Superseded
    );
}

fn slowly(switches_to: i64, counter: &'static std::sync::atomic::AtomicI64) -> Captured {
    of_one_generation(
        awaiting("a.txt", None),
        Some(1),
        || Some(counter.load(std::sync::atomic::Ordering::SeqCst)),
        |item| {
            std::thread::sleep(std::time::Duration::from_millis(60));
            counter.store(switches_to, std::sync::atomic::Ordering::SeqCst);
            with_bytes(item, b"late")
        },
    )
}

#[test]
fn a_read_that_finishes_late_still_answers_for_the_copy_it_began_on() {
    static SAME: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    let pending = reading::begin(|| slowly(1, &SAME));
    assert_eq!(
        pending.wait(std::time::Duration::from_millis(1)),
        None,
        "the wait gives up before the read is done"
    );
    assert_eq!(
        pending.wait(std::time::Duration::from_secs(10)),
        Some(Captured::Kept(with_bytes(awaiting("a.txt", None), b"late")))
    );

    static MOVED: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    let pending = reading::begin(|| slowly(2, &MOVED));
    assert_eq!(
        pending.wait(std::time::Duration::from_secs(10)),
        Some(Captured::Superseded),
        "the copy moved on before the late read finished"
    );
}

#[test]
fn an_empty_answer_fills_an_empty_or_unsized_file_but_not_one_promised_bytes() {
    let described = |size| {
        vec![virtual_files::Described {
            name: "a.txt".into(),
            size,
            is_dir: false,
        }]
    };
    for size in [Some(0), None] {
        let got = filled_with(
            awaiting("a.txt", size),
            &described(size),
            &[Some(Vec::new())],
        );
        assert_eq!(got.formats[1].payload, Payload::Inline(Vec::new()));
    }
    let got = filled_with(
        awaiting("a.txt", Some(5)),
        &described(Some(5)),
        &[Some(Vec::new())],
    );
    assert_eq!(
        got.formats[1].payload,
        Payload::Absent,
        "a file said to hold five bytes did not arrive empty, it did not arrive"
    );
    let got = filled_with(awaiting("a.txt", Some(0)), &described(Some(0)), &[None]);
    assert_eq!(
        got.formats[1].payload,
        Payload::Absent,
        "a failure never becomes an empty file"
    );
    let got = filled_with(
        awaiting("a.txt", Some(5)),
        &described(Some(5)),
        &[Some(b"hello".to_vec())],
    );
    assert_eq!(got.formats[1].payload, Payload::Inline(b"hello".to_vec()));
}

struct VirtualOwner(Child);

impl Drop for VirtualOwner {
    fn drop(&mut self) {
        self.0.kill().ok();
        self.0.wait().ok();
    }
}

const NATIVE_OWNER: &str = "using System; using System.Text; using System.Runtime.InteropServices; \
using CT = System.Runtime.InteropServices.ComTypes; \
public static class NativeOwner { \
[DllImport(\"shell32.dll\")] static extern int SHCreateDataObject(IntPtr f, uint n, IntPtr a, CT.IDataObject i, ref Guid g, out CT.IDataObject o); \
[DllImport(\"ole32.dll\")] static extern int OleInitialize(IntPtr r); \
[DllImport(\"ole32.dll\")] static extern int OleSetClipboard(CT.IDataObject d); \
[DllImport(\"ole32.dll\")] static extern int CreateStreamOnHGlobal(IntPtr h, bool f, out IntPtr s); \
[DllImport(\"user32.dll\", CharSet = CharSet.Unicode)] static extern ushort RegisterClipboardFormatW(string n); \
[DllImport(\"kernel32.dll\")] static extern IntPtr GlobalAlloc(uint f, UIntPtr n); \
[DllImport(\"kernel32.dll\")] static extern IntPtr GlobalLock(IntPtr h); \
[DllImport(\"kernel32.dll\")] static extern bool GlobalUnlock(IntPtr h); \
static IntPtr Global(byte[] b) { IntPtr h = GlobalAlloc(2, (UIntPtr)b.Length); \
if (b.Length > 0) { Marshal.Copy(b, 0, GlobalLock(h), b.Length); GlobalUnlock(h); } return h; } \
static void Offer(CT.IDataObject d, string n, CT.TYMED t, IntPtr h) { \
CT.FORMATETC f = new CT.FORMATETC(); f.cfFormat = (short)RegisterClipboardFormatW(n); \
f.dwAspect = CT.DVASPECT.DVASPECT_CONTENT; f.lindex = -1; f.tymed = t; \
CT.STGMEDIUM m = new CT.STGMEDIUM(); m.tymed = t; m.unionmember = h; d.SetData(ref f, ref m, true); } \
public static int Put(string name, string text) { OleInitialize(IntPtr.Zero); byte[] b = Encoding.UTF8.GetBytes(text); \
byte[] f = new byte[596]; BitConverter.GetBytes(1).CopyTo(f, 0); BitConverter.GetBytes(0x40).CopyTo(f, 4); \
BitConverter.GetBytes(b.Length).CopyTo(f, 72); Encoding.Unicode.GetBytes(name).CopyTo(f, 76); \
Guid g = new Guid(\"0000010e-0000-0000-C000-000000000046\"); CT.IDataObject d; \
int made = SHCreateDataObject(IntPtr.Zero, 0, IntPtr.Zero, null, ref g, out d); if (made != 0) return made; \
Offer(d, \"FileGroupDescriptorW\", CT.TYMED.TYMED_HGLOBAL, Global(f)); IntPtr s; \
int opened = CreateStreamOnHGlobal(Global(b), true, out s); if (opened != 0) return opened; \
Offer(d, \"FileContents\", CT.TYMED.TYMED_ISTREAM, s); return OleSetClipboard(d); } }";

fn owner_of_a_virtual_file(name: &str, bytes: &str) -> VirtualOwner {
    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms; \
Add-Type -TypeDefinition '{NATIVE_OWNER}'; \
if ([NativeOwner]::Put('{name}', '{bytes}') -ne 0) {{ exit 1 }}; \
[Console]::Out.WriteLine('placed'); [Console]::Out.Flush(); \
while ($true) {{ [System.Windows.Forms.Application]::DoEvents(); Start-Sleep -Milliseconds 5 }}"
    );
    let mut child = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-STA", "-Command", &script])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .expect("PowerShell starts");
    let stdout = child.stdout.take().expect("its output");
    let owner = VirtualOwner(child);
    let mut said = String::new();
    BufReader::new(stdout)
        .read_line(&mut said)
        .expect("it says");
    assert_eq!(
        said.trim(),
        "placed",
        "the virtual file is on the clipboard"
    );
    owner
}

fn write_text(text: &str) {
    let clipboard = Clipboard::to_write().expect("the clipboard opens to write");
    let units = cp_win_sys::writing::utf16_of(text);
    assert_eq!(
        clipboard.replace(&[(cp_win_sys::formats::CF_UNICODETEXT, &units)]),
        cp_win_sys::writing::Written::Placed { formats: 1 }
    );
}

fn locked_capture() -> (Item, Option<i64>) {
    let clipboard = Clipboard::open().expect("the clipboard opens");
    let item = capture(&clipboard).kept().expect("something was captured");
    let generation = clipboard.generation();
    (item, generation)
}

fn described_name(item: &Item) -> String {
    let descriptor = bytes_of(&item.formats, DESCRIPTOR).expect("the descriptor is kept");
    virtual_files::described_in(descriptor)[0].name.clone()
}

#[test]
fn on_the_real_clipboard_a_delayed_render_is_kept_and_another_copy_is_refused() {
    let _alone = the_real_clipboard_alone();
    let owner = owner_of_a_virtual_file("a.txt", "of a");
    let kept = capture_now();
    let item = kept
        .kept()
        .expect("a rendered copy of one generation is kept");
    assert_eq!(described_name(&item), "a.txt");
    assert_eq!(
        item.format(&virtual_files::contents_id(0))
            .map(|one| &one.payload),
        Some(&Payload::Inline(b"of a".to_vec()))
    );

    let counted = clipboard::reading();
    let mut other_owner = None;
    let got = capture_counted_then(&counted, || {
        other_owner = Some(owner_of_a_virtual_file("b.txt", "of b"));
    });
    drop(counted);
    assert_eq!(got, Captured::Superseded, "A's name never meets B's bytes");
    drop(other_owner);
    drop(owner);

    let owner = owner_of_a_virtual_file("a.txt", "of a");
    let (item, generation) = locked_capture();
    let got = of_one_generation(item, generation, clipboard::sequence, |item| {
        let read = with_virtual_contents(item);
        write_text("b replaced a during the read");
        read
    });
    assert_eq!(got, Captured::Superseded);
    drop(owner);
}

#[test]
fn an_empty_virtual_file_and_a_full_one_survive_the_whole_way_to_a_paste() {
    let _alone = the_real_clipboard_alone();
    let store = Store::in_memory().expect("a store");
    for (name, text) in [("klp-002-empty.txt", ""), ("klp-002-full.txt", "hello")] {
        let bytes = text.as_bytes();
        let owner = owner_of_a_virtual_file(name, text);
        let captured = capture_now().kept().expect("it was captured");
        drop(owner);
        let id = store
            .insert_item(name, &captured, name, 1)
            .expect("it is stored");
        let stored = store.item(id).expect("it reads").expect("it is there");
        assert_eq!(
            stored
                .format(&virtual_files::contents_id(0))
                .map(|one| &one.payload),
            Some(&Payload::Inline(bytes.to_vec())),
            "{name} keeps exactly its bytes"
        );
        let ready = crate::restore::ready_for(&stored);
        assert!(ready.offers(cp_win_sys::formats::CF_HDROP), "{name} pastes");
        let root = cp_win_sys::paths::pastes_dir().join(format!("{:016x}", stored.fingerprint()));
        let on_disk = root.join(name);
        assert_eq!(std::fs::read(&on_disk).expect("on disk"), bytes);
        std::fs::remove_dir_all(&root).ok();
    }
}

#[test]
fn the_whole_capture_has_a_ceiling_well_under_the_thirty_seconds() {
    assert!(PATIENCE < std::time::Duration::from_secs(1));
    assert!(PATIENCE > cp_win_sys::reading::PATIENCE);
}

#[test]
fn a_capture_that_does_not_finish_in_time_is_abandoned() {
    let seen = reading::anything_within(std::time::Duration::from_millis(20), || {
        std::thread::sleep(std::time::Duration::from_secs(30));
        Captured::Nothing
    });
    assert_eq!(seen, None, "the thread is abandoned and not waited for");
}

#[test]
fn the_class_of_a_drop_comes_from_its_first_path() {
    let formats = vec![Format {
        id: "CF_HDROP".into(),
        payload: Payload::Inline(drop_of(&[r"C:\video.mkv"])),
    }];
    assert_eq!(refine(Some(Family::Files), &formats), Some(Kind::Video));
}

#[test]
fn a_folder_is_told_apart_by_its_trailing_separator() {
    let formats = vec![Format {
        id: "CF_HDROP".into(),
        payload: Payload::Inline(drop_of(&[r"C:\Documents\\"])),
    }];
    assert_eq!(refine(Some(Family::Files), &formats), Some(Kind::Folder));
}

#[test]
fn text_is_refined_by_what_it_says() {
    let formats = vec![Format {
        id: "CF_UNICODETEXT".into(),
        payload: Payload::Inline(cp_win_sys::writing::utf16_of("someone@example.test")),
    }];
    assert_eq!(refine(Some(Family::Text), &formats), Some(Kind::Email));
}

#[test]
fn text_that_could_not_be_read_is_still_text() {
    let formats = vec![Format {
        id: "CF_UNICODETEXT".into(),
        payload: Payload::Absent,
    }];
    assert_eq!(refine(Some(Family::Text), &formats), Some(Kind::Text));
}

#[test]
fn an_image_needs_no_refining() {
    assert_eq!(refine(Some(Family::Image), &[]), Some(Kind::Image));
}

#[test]
fn nothing_offered_has_no_class() {
    assert_eq!(refine(None, &[]), None);
}

#[test]
fn a_virtual_file_is_classed_by_the_name_its_descriptor_gives() {
    let formats = vec![Format {
        id: DESCRIPTOR.into(),
        payload: Payload::Inline(descriptor_of(&[("capture.png", Some(9), false)])),
    }];
    assert_eq!(refine(Some(Family::Files), &formats), Some(Kind::Image));
    let folder = vec![Format {
        id: DESCRIPTOR.into(),
        payload: Payload::Inline(descriptor_of(&[("attachments", None, true)])),
    }];
    assert_eq!(refine(Some(Family::Files), &folder), Some(Kind::Folder));
    let real_drop_wins = vec![
        Format {
            id: "CF_HDROP".into(),
            payload: Payload::Inline(drop_of(&[r"C:\video.mkv"])),
        },
        folder[0].clone(),
    ];
    assert_eq!(
        refine(Some(Family::Files), &real_drop_wins),
        Some(Kind::Video)
    );
}

#[test]
fn only_a_virtual_item_with_something_absent_waits_for_the_ole_read() {
    let waiting = Item {
        kind: Some(Kind::File),
        formats: vec![
            Format {
                id: DESCRIPTOR.into(),
                payload: Payload::Inline(descriptor_of(&[("a.txt", Some(1), false)])),
            },
            Format {
                id: virtual_files::contents_id(0),
                payload: Payload::Absent,
            },
        ],
    };
    assert!(awaits_virtual_contents(&waiting));
    let too_big = Item {
        kind: Some(Kind::File),
        formats: vec![Format {
            id: virtual_files::contents_id(0),
            payload: Payload::TooBig { size: 1 << 40 },
        }],
    };
    assert!(
        !awaits_virtual_contents(&too_big),
        "what does not fit is not asked for"
    );
    let plain_absent = Item {
        kind: Some(Kind::Text),
        formats: vec![Format {
            id: "CF_UNICODETEXT".into(),
            payload: Payload::Absent,
        }],
    };
    assert!(!awaits_virtual_contents(&plain_absent));
    assert!(!awaits_virtual_contents(&Item::plain("hello")));
}

#[test]
fn a_drop_with_no_paths_is_still_a_file() {
    let formats = vec![Format {
        id: "CF_HDROP".into(),
        payload: Payload::Absent,
    }];
    assert_eq!(refine(Some(Family::Files), &formats), Some(Kind::File));
}

fn a_dib_of_one_colour() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&2i32.to_le_bytes());
    out.extend_from_slice(&2i32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&[0u8; 16]);
    out.extend_from_slice(&[0x20, 0x60, 0xA0, 0xFF].repeat(4));
    out
}

#[test]
fn a_picture_on_the_clipboard_crosses_as_a_png_and_not_as_its_raw_bitmap() {
    let _alone = the_real_clipboard_alone();
    let raw = a_dib_of_one_colour();
    {
        let clipboard = Clipboard::to_write().expect("the clipboard opens to write");
        assert_eq!(
            clipboard.replace(&[(cp_win_sys::formats::CF_DIB, &raw)]),
            cp_win_sys::writing::Written::Placed { formats: 1 },
            "the picture never reached the clipboard, so what follows would test nothing"
        );
    }
    let clipboard = Clipboard::open().expect("the clipboard opens");
    let item = capture(&clipboard).kept().expect("a picture was captured");
    assert_eq!(item.kind, Some(Kind::Image));
    let picture = item
        .formats
        .iter()
        .find(|one| one.id == SYNTHETIC_IMAGE)
        .expect("the picture is carried under its own name");
    let bytes = match &picture.payload {
        Payload::Inline(bytes) => bytes.clone(),
        Payload::Blob(bytes) => bytes.clone(),
        other => panic!("the picture arrived as {other:?}"),
    };
    assert_eq!(
        &bytes[..8],
        b"\x89PNG\r\n\x1a\n",
        "a bitmap off the clipboard is transcoded, because nothing else reads a raw DIB"
    );
}
