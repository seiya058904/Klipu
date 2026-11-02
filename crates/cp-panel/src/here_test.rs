use super::*;

#[test]
fn what_the_platform_promises_about_file_thumbnails_is_what_it_does() {
    if !THUMBNAILS_FILES {
        assert!(
            thumb_of_file(Path::new("/"), 128).is_none(),
            "si no dibuja miniaturas de archivos, no puede devolver una"
        );
    }
}

#[test]
fn the_folders_the_panel_writes_into_hang_from_the_same_root() {
    let Some(root) = data_dir() else {
        return;
    };
    assert!(thumbs_dir().expect("miniaturas").starts_with(&root));
}

#[test]
fn asking_the_system_for_its_theme_answers_the_same_twice() {
    assert_eq!(system_is_light(), system_is_light());
}

#[test]
fn nobody_in_front_reads_as_zero_and_pasting_there_reaches_nobody() {
    let said = std::rc::Rc::new(std::cell::Cell::new(None));
    let heard = said.clone();
    paste_into(0, || {}, move |sent| heard.set(Some(sent)));
    assert_eq!(said.take(), Some(Sent::Nobody));
}

#[test]
fn nobody_in_front_never_hides_the_panel() {
    let mut hidden = false;
    paste_into(0, || hidden = true, |_| {});
    assert!(!hidden);
}

#[test]
fn a_handle_from_another_system_is_left_as_it_is_when_the_panel_goes() {
    use raw_window_handle::XlibWindowHandle;
    ground(RawWindowHandle::Xlib(XlibWindowHandle::new(1)));
}

#[test]
fn a_handle_from_a_system_this_one_is_not_is_no_window_to_drag_from() {
    use raw_window_handle::XlibWindowHandle;
    assert_eq!(
        drag_out(
            RawWindowHandle::Xlib(XlibWindowHandle::new(1)),
            &[Path::new("/etc/hosts")]
        ),
        Dragged::Elsewhere
    );
}

#[cfg(target_os = "windows")]
#[test]
fn nothing_to_drag_never_starts_a_drag() {
    use raw_window_handle::Win32WindowHandle;
    let hwnd = std::num::NonZeroIsize::new(1).expect("one is not zero");
    let handle = RawWindowHandle::Win32(Win32WindowHandle::new(hwnd));
    assert_ne!(drag_out(handle, &[]), Dragged::Started);
}

#[test]
fn no_window_is_aimed_at_nowhere_in_particular() {
    assert_eq!(super::towards(0), crate::landing::Towards::Elsewhere);
}

#[test]
fn nobody_in_front_is_pasted_to_as_anywhere_else() {
    assert_eq!(towards(0), crate::landing::Towards::Elsewhere);
    assert_eq!(towards(-1), crate::landing::Towards::Elsewhere);
}

#[test]
fn the_pointer_lands_on_a_work_area_that_holds_it() {
    let Some(pointer) = pointer() else {
        return;
    };
    let area = pointer.area;
    assert!(
        area.right > area.left && area.bottom > area.top,
        "{pointer:?}"
    );
    assert!(pointer.scale >= 1.0, "{pointer:?}");
}

#[test]
fn the_reading_permission_always_has_an_answer_or_none() {
    let _ = unreadable();
}

#[test]
fn the_panel_can_stay_awake_while_it_watches() {
    let _awake = keep_awake();
}

#[test]
fn the_host_and_what_is_ahead_can_always_be_asked() {
    let host = host();
    let _ = ahead_now(host);
    let _ = activated();
}

#[test]
fn handing_back_to_nobody_does_nothing() {
    hand_back(0);
}
