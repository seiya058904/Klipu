use super::*;

#[test]
fn everything_hangs_from_one_folder() {
    let root = data_dir().expect("LOCALAPPDATA");
    for path in [
        database().expect("base"),
        blobs_dir().expect("blobs"),
        thumbs_dir().expect("miniaturas"),
    ] {
        assert!(path.starts_with(&root), "{path:?} stepped outside {root:?}");
    }
}

#[test]
fn the_data_folder_is_local_and_never_roaming() {
    let root = data_dir().expect("LOCALAPPDATA");
    let text = root.to_string_lossy().to_ascii_lowercase();
    assert!(text.contains("local"), "{text}");
    assert!(
        !text.contains("roaming"),
        "a roaming profile would upload the history to the network: {text}"
    );
}
