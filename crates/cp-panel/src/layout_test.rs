use super::*;

#[test]
fn only_pictures_alone_get_the_grid() {
    assert_eq!(layout_for(&[Kind::Image]), Layout::Image);
    assert_eq!(layout_for(&[Kind::Image, Kind::Image]), Layout::Image);
}

#[test]
fn everything_else_is_the_list() {
    assert_eq!(layout_for(&[]), Layout::Everything);
    assert_eq!(layout_for(&[Kind::Link]), Layout::Everything);
    assert_eq!(layout_for(&[Kind::Video]), Layout::Everything);
    assert_eq!(layout_for(&[Kind::Image, Kind::Text]), Layout::Everything);
}

#[test]
fn the_names_are_the_ones_the_panel_reads() {
    assert_eq!(Layout::Everything.as_str(), "everything");
    assert_eq!(Layout::Image.as_str(), "image");
}
