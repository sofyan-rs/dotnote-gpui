//! Reading the GTK theme's window button colors from a real config folder.

use std::fs;

use dotnote_core::gtk_theme::read_ring_colors_from;

#[test]
fn follows_imports_and_tolerates_a_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(read_ring_colors_from(dir.path()), None);

    // adw-gtk3's sync tool writes the buttons to their own file and imports it.
    fs::write(
        dir.path().join("gtk.css"),
        "@import url(\"buttons.css\");\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("buttons.css"),
        "button.close > image { box-shadow: inset 0 0 0 6px #e86150; }
         button.maximize > image { box-shadow: inset 0 0 0 6px #75c2b3; }
         button.minimize > image { box-shadow: inset 0 0 0 6px #d7c781; }",
    )
    .unwrap();
    let rings = read_ring_colors_from(dir.path()).unwrap();
    assert_eq!(rings.close, 0xe86150);
    assert_eq!(rings.minimize, 0xd7c781);
}
