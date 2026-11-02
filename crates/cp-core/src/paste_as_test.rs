use super::*;

#[test]
fn text_is_pasted_as_the_text_it_is() {
    let content = Content {
        text: Some("línea\n中文".into()),
        ..Content::default()
    };
    assert_eq!(plainest(&content).as_deref(), Some("línea\n中文"));
}

#[test]
fn files_are_pasted_as_their_paths() {
    let content = Content {
        paths: vec![r"C:\a.txt".to_owned(), r"C:\b.txt".to_owned()],
        ..Content::default()
    };
    assert_eq!(plainest(&content).as_deref(), Some("C:\\a.txt\nC:\\b.txt"));
}

#[test]
fn a_picture_is_pasted_as_the_text_read_out_of_it() {
    let content = Content {
        image: Some(b"png"),
        ocr: Some("  识别的文字 "),
        ..Content::default()
    };
    assert_eq!(plainest(&content).as_deref(), Some("识别的文字"));
}

#[test]
fn a_picture_nothing_was_read_from_has_no_plain_form() {
    let content = Content {
        image: Some(b"png"),
        ocr: Some("   "),
        ..Content::default()
    };
    assert_eq!(plainest(&content), None);
}

#[test]
fn empty_text_has_no_plain_form() {
    let content = Content {
        text: Some("".into()),
        ..Content::default()
    };
    assert_eq!(plainest(&content), None);
}
