use super::*;
use windows::Win32::Foundation::GlobalFree;
use windows::Win32::System::Com::STGMEDIUM_0;
use windows::Win32::System::Com::StructuredStorage::CreateStreamOnHGlobal;
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows::Win32::UI::Shell::SHCreateDataObject;

const OFFERED: u32 = 0xC123;

fn a_global_holding(bytes: &[u8]) -> HGLOBAL {
    let global = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes.len()) }.expect("memory");
    if !bytes.is_empty() {
        let address = unsafe { GlobalLock(global) };
        assert!(!address.is_null());
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), address.cast(), bytes.len()) };
        let _ = unsafe { GlobalUnlock(global) };
    }
    global
}

fn a_stream_holding(bytes: &[u8]) -> IStream {
    unsafe { CreateStreamOnHGlobal(a_global_holding(bytes), true) }.expect("a stream")
}

fn a_source() -> IDataObject {
    unsafe { SHCreateDataObject(None, None, None::<&IDataObject>) }.expect("a data object")
}

fn offer(data: &IDataObject, lindex: i32, medium: STGMEDIUM) {
    let format = FORMATETC {
        cfFormat: OFFERED as u16,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex,
        tymed: medium.tymed,
    };
    unsafe { data.SetData(&format, &medium, true) }.expect("the source takes it");
}

fn in_stream(bytes: &[u8]) -> STGMEDIUM {
    STGMEDIUM {
        tymed: TYMED_ISTREAM.0 as u32,
        u: STGMEDIUM_0 {
            pstm: std::mem::ManuallyDrop::new(Some(a_stream_holding(bytes))),
        },
        pUnkForRelease: std::mem::ManuallyDrop::new(None),
    }
}

fn in_global(bytes: &[u8]) -> STGMEDIUM {
    STGMEDIUM {
        tymed: TYMED_HGLOBAL.0 as u32,
        u: STGMEDIUM_0 {
            hGlobal: a_global_holding(bytes),
        },
        pUnkForRelease: std::mem::ManuallyDrop::new(None),
    }
}

fn through_a_real_source(medium: STGMEDIUM, up_to: usize) -> Option<Vec<u8>> {
    let _ole = Ole::enter();
    let data = a_source();
    offer(&data, -1, medium);
    one_of(&data, OFFERED, -1, up_to)
}

fn reading_out_of(steps: &[(bool, &[u8])], up_to: usize) -> Option<Vec<u8>> {
    let mut steps = steps.iter();
    drained(
        |chunk| {
            let (failed, bytes) = steps.next().copied().unwrap_or((false, b""));
            chunk[..bytes.len()].copy_from_slice(bytes);
            (failed, bytes.len())
        },
        up_to,
    )
}

#[test]
fn entering_twice_is_not_a_problem() {
    let first = Ole::enter();
    let second = Ole::enter();
    drop(second);
    drop(first);
}

#[test]
fn asking_for_nothing_gets_nothing_and_does_not_touch_the_clipboard() {
    assert!(indexed_contents(0xC000, 0, CHUNK).is_empty());
}

#[test]
fn a_format_nobody_offers_comes_back_absent_for_every_index() {
    let seen = indexed_contents(0xFFFE, 3, CHUNK);
    assert_eq!(seen, vec![None, None, None]);
}

#[test]
fn a_stream_that_ends_at_once_is_an_empty_file_and_not_a_missing_one() {
    let _ole = Ole::enter();
    assert_eq!(
        stream_bytes(&a_stream_holding(b""), CHUNK),
        Some(Vec::new())
    );
    assert_eq!(
        through_a_real_source(in_stream(b""), CHUNK),
        Some(Vec::new())
    );
}

#[test]
fn a_global_of_no_bytes_is_an_empty_file_and_not_a_missing_one() {
    let global = a_global_holding(b"");
    assert_eq!(global_contents(global, CHUNK), Some(Vec::new()));
    let _ = unsafe { GlobalFree(Some(global)) };
}

#[test]
fn bytes_arrive_the_same_way_beside_the_empty_ones() {
    assert_eq!(
        through_a_real_source(in_stream(b"hello"), CHUNK),
        Some(b"hello".to_vec())
    );
    assert_eq!(
        through_a_real_source(in_global(b"hello"), CHUNK),
        Some(b"hello".to_vec())
    );
}

#[test]
fn a_failure_is_never_an_end_however_much_came_before_it() {
    assert_eq!(
        reading_out_of(&[], CHUNK),
        Some(Vec::new()),
        "an honest end"
    );
    assert_eq!(reading_out_of(&[(true, b"")], CHUNK), None);
    assert_eq!(
        reading_out_of(&[(false, b"partial"), (true, b"")], CHUNK),
        None,
        "bytes before the failure do not make a whole file"
    );
    assert_eq!(
        reading_out_of(&[(true, b"last"), (false, b"")], CHUNK),
        Some(b"last".to_vec()),
        "bytes handed over with a warning still count"
    );
}

#[test]
fn a_format_the_source_does_not_offer_is_missing_not_empty() {
    let _ole = Ole::enter();
    let data = a_source();
    offer(&data, -1, in_stream(b""));
    assert_eq!(one_of(&data, 0xC124, -1, CHUNK), None);
}

#[test]
fn the_ceiling_refuses_what_exceeds_it_and_only_that() {
    assert_eq!(through_a_real_source(in_stream(b"hello"), 4), None);
    assert_eq!(through_a_real_source(in_global(b"hello"), 4), None);
    assert_eq!(
        through_a_real_source(in_stream(b"hello"), 5),
        Some(b"hello".to_vec())
    );
    assert_eq!(
        through_a_real_source(in_global(b"hello"), 5),
        Some(b"hello".to_vec())
    );
    assert_eq!(reading_out_of(&[(false, b"")], 0), Some(Vec::new()));
}

#[test]
fn a_handle_that_is_not_one_is_missing_not_empty() {
    assert_eq!(global_contents(HGLOBAL::default(), CHUNK), None);
}

#[test]
fn a_lone_file_may_answer_without_an_index_but_a_missing_one_stays_missing() {
    let _ole = Ole::enter();
    let lone = a_source();
    offer(&lone, -1, in_stream(b""));
    assert_eq!(
        contents_of(&lone, OFFERED, 1, CHUNK),
        vec![Some(Vec::new())]
    );
    let nothing = a_source();
    assert_eq!(contents_of(&nothing, OFFERED, 1, CHUNK), vec![None]);
    assert_eq!(contents_of(&nothing, OFFERED, 2, CHUNK), vec![None, None]);
}
