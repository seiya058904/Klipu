use super::*;

fn seen<'a>(kind: Option<Kind>, body: &'a str) -> Seen<'a> {
    Seen {
        kind,
        found: false,
        thumb: false,
        paints: false,
        body,
        most: 2,
    }
}

#[test]
fn a_search_hit_a_picture_and_a_colour_win_over_the_kind() {
    let mut one = seen(Some(Kind::Code), "fn main() {}");
    one.found = true;
    assert_eq!(Face::of(&one), Face::Found);
    one.thumb = true;
    assert_eq!(
        Face::of(&one),
        Face::FoundThumb,
        "a picture found by its words keeps its thumbnail"
    );
    one.found = false;
    one.found = false;
    one.thumb = true;
    assert_eq!(Face::of(&one), Face::Thumb);
    one.thumb = false;
    one.paints = true;
    assert_eq!(Face::of(&one), Face::Paint);
}

#[test]
fn a_short_text_takes_one_line_and_a_long_one_two() {
    assert_eq!(
        Face::of(&seen(Some(Kind::Text), "git push")),
        Face::Words(1)
    );
    let long = "Hola equipo: el despliegue del jueves queda a las 10:00, revisen la lista.";
    assert_eq!(Face::of(&seen(Some(Kind::Text), long)), Face::Words(2));
    assert_eq!(Face::of(&seen(None, "4")), Face::Words(1));
}

#[test]
fn code_and_tokens_open_a_block_as_tall_as_their_first_two_lines() {
    assert_eq!(
        Face::of(&seen(Some(Kind::Code), "let a = 1;")),
        Face::Block(1)
    );
    assert_eq!(
        Face::of(&seen(Some(Kind::Code), "let a = 1;\n\n  let b = 2;")),
        Face::Block(2)
    );
    assert_eq!(
        Face::of(&seen(Some(Kind::Token), "eyJhbGciOi")),
        Face::Block(1)
    );
}

#[test]
fn the_opening_skips_blank_lines_and_drops_the_indent_they_share() {
    let (one, two) = opening_of("\n    if ready {\n        go();\n    }\n");
    assert_eq!(one, "if ready {");
    assert_eq!(two, "    go();");
    assert_eq!(opening_of("solo"), ("solo".to_owned(), String::new()));
    assert_eq!(opening_of(""), (String::new(), String::new()));
}

#[test]
fn the_lines_are_counted_only_when_more_than_are_shown() {
    assert_eq!(lines_said(1, 1, false), "");
    assert_eq!(lines_said(2, 2, false), "");
    assert_eq!(lines_said(12, 2, false), "共 12 行");
    assert_eq!(lines_said(12, 2, true), "12 lines");
}

#[test]
fn the_aside_joins_what_is_there_and_skips_what_is_not() {
    assert_eq!(aside_said(&["", " firefox ", "×3", ""]), "firefox · ×3");
    assert_eq!(aside_said(&[]), "");
}

#[test]
fn a_colour_sits_on_a_single_row_while_closed() {
    assert_eq!(Face::Paint.shut_px(), TOP + META + BOTTOM + BETWEEN);
    assert_eq!(Face::FoundThumb.body_px(), THUMB);
    assert_eq!(Face::FoundThumb.as_str(), "found-thumb");
}

#[test]
fn a_closed_text_shows_as_many_lines_as_the_chosen_height_allows() {
    let long = "palabra ".repeat(40);
    let text = seen(Some(Kind::Text), &long);
    assert_eq!(Face::of(&Seen { most: 1, ..text }), Face::Words(1));
    assert_eq!(Face::of(&Seen { most: 2, ..text }), Face::Words(2));
    assert_eq!(Face::of(&Seen { most: 3, ..text }), Face::Words(3));
    assert_eq!(Face::of(&Seen { most: 9, ..text }), Face::Words(3));
    let short = seen(Some(Kind::Text), "git push");
    assert_eq!(Face::of(&Seen { most: 3, ..short }), Face::Words(1));
}

#[test]
fn code_never_shows_more_than_its_two_opening_lines() {
    let code = seen(Some(Kind::Code), "fn main() {\n    run();\n    stop();\n}");
    assert_eq!(Face::of(&Seen { most: 1, ..code }), Face::Block(1));
    assert_eq!(Face::of(&Seen { most: 3, ..code }), Face::Block(2));
}

#[test]
fn the_chosen_height_is_kept_between_one_and_three_lines() {
    assert_eq!(shut_lines_within(0), 1);
    assert_eq!(shut_lines_within(2), 2);
    assert_eq!(shut_lines_within(7), 3);
}
