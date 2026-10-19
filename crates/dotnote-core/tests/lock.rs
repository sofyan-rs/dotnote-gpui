//! Locked notes on a real disk: password, lock, unlock, wrong password.

use std::fs;

use dotnote_core::lock;

#[test]
fn a_locked_note_is_unreadable_on_disk_and_opens_with_the_password() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let note = root.join("Keys.md");
    fs::write(&note, "api_key = sk-日本-🔑\n").unwrap();

    assert!(!lock::has_password(root));
    assert!(lock::create_password(root, "short").is_err(), "too short");
    let key = lock::create_password(root, "correct horse").unwrap();
    assert!(lock::has_password(root));
    assert!(
        lock::create_password(root, "another password").is_err(),
        "only one"
    );

    let locked = lock::lock_note(&key, &note).unwrap();
    assert_eq!(locked, root.join("Keys.md.age"));
    assert!(
        note.exists(),
        "the app moves the readable copy to the trash"
    );
    fs::remove_file(&note).unwrap(); // (the app trashes it)
    let on_disk = fs::read_to_string(&locked).unwrap();
    assert!(on_disk.starts_with("-----BEGIN AGE ENCRYPTED FILE-----"));
    assert!(!on_disk.contains("sk-"), "the secret isn't on disk");

    // Later (e.g. after a restart): unlock with the password.
    assert!(lock::unlock(root, "wrong password").is_err());
    let key = lock::unlock(root, "correct horse").unwrap();
    assert_eq!(
        lock::read_note(&key, &locked).unwrap(),
        "api_key = sk-日本-🔑\n"
    );

    lock::write_note(&key, &locked, "changed\n").unwrap();
    assert_eq!(lock::read_note(&key, &locked).unwrap(), "changed\n");

    let back = lock::unlock_note(&key, &locked).unwrap();
    assert_eq!(fs::read_to_string(&back).unwrap(), "changed\n");
}

#[test]
fn a_note_from_another_vault_key_is_refused() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let key_a = lock::create_password(a.path(), "password a").unwrap();
    let key_b = lock::create_password(b.path(), "password b").unwrap();
    let note = a.path().join("N.md");
    fs::write(&note, "x").unwrap();
    let locked = lock::lock_note(&key_a, &note).unwrap();
    assert!(lock::read_note(&key_b, &locked).is_err());
}

#[test]
fn changing_the_password_keeps_locked_notes_readable() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let note = root.join("Keys.md");
    fs::write(&note, "pin = 1234 🔑\n").unwrap();
    let key = lock::create_password(root, "old password").unwrap();
    let locked = lock::lock_note(&key, &note).unwrap();
    let locked_bytes = fs::read(&locked).unwrap();

    assert!(lock::change_password(root, "wrong one!", "new password").is_err());
    assert!(
        lock::change_password(root, "old password", "short").is_err(),
        "too short"
    );
    let key = lock::change_password(root, "old password", "new password").unwrap();
    assert_eq!(lock::read_note(&key, &locked).unwrap(), "pin = 1234 🔑\n");
    assert_eq!(fs::read(&locked).unwrap(), locked_bytes, "notes untouched");

    assert!(lock::unlock(root, "old password").is_err());
    let key = lock::unlock(root, "new password").unwrap();
    assert_eq!(lock::read_note(&key, &locked).unwrap(), "pin = 1234 🔑\n");
}

#[test]
fn old_kunotes_key_file_is_renamed() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(".kunotes-lock.age"), "key").unwrap();
    lock::migrate_old_key_file(dir.path()).unwrap();
    assert!(!dir.path().join(".kunotes-lock.age").exists());
    assert_eq!(
        fs::read_to_string(dir.path().join(lock::KEY_FILE)).unwrap(),
        "key"
    );
}

#[test]
fn a_folder_locks_and_unlocks_with_its_subfolders() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let folder = root.join("Secrets");
    let sub = folder.join("日本 🔑");
    fs::create_dir_all(&sub).unwrap();
    fs::create_dir_all(folder.join(".hidden")).unwrap();
    fs::write(folder.join("A.md"), "a").unwrap();
    fs::write(sub.join("B.md"), "b 🔑").unwrap();
    fs::write(folder.join("photo.png"), "png").unwrap();
    fs::write(folder.join(".hidden").join("H.md"), "h").unwrap();
    fs::write(root.join("Outside.md"), "o").unwrap();
    let key = lock::create_password(root, "correct horse").unwrap();
    // Already locked, and a name whose locked form is taken: skipped / failed.
    lock::lock_note(&key, &folder.join("A.md")).unwrap();
    fs::remove_file(folder.join("A.md")).unwrap(); // (the app trashes it)
    fs::write(folder.join("C.md"), "c").unwrap();
    fs::write(folder.join("C.md.age"), "taken").unwrap();

    let change = lock::convert_folder(&key, &folder, true);
    let converted: Vec<_> = change
        .converted
        .iter()
        .map(|(_, new)| new.clone())
        .collect();
    assert_eq!(converted, vec![sub.join("B.md.age")]);
    assert_eq!(change.failed.len(), 1, "C.md.age already exists");
    assert_eq!(change.failed[0].0, folder.join("C.md"));
    assert!(sub.join("B.md").exists(), "the app trashes the plain copy");
    assert_eq!(
        lock::read_note(&key, &sub.join("B.md.age")).unwrap(),
        "b 🔑"
    );
    assert!(
        !root.join("Outside.md.age").exists(),
        "only inside the folder"
    );
    assert!(
        !folder.join(".hidden").join("H.md.age").exists(),
        "hidden skipped"
    );

    fs::remove_file(sub.join("B.md")).unwrap();
    fs::remove_file(folder.join("C.md.age")).unwrap();
    let change = lock::convert_folder(&key, &folder, false);
    let mut converted: Vec<_> = change
        .converted
        .iter()
        .map(|(_, new)| new.clone())
        .collect();
    converted.sort();
    assert_eq!(converted, vec![folder.join("A.md"), sub.join("B.md")]);
    assert!(change.failed.is_empty());
    assert_eq!(fs::read_to_string(sub.join("B.md")).unwrap(), "b 🔑");
}
