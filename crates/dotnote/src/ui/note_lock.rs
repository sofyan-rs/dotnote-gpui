//! Locking a note (encrypting it as `Name.md.age`) and removing a lock, for
//! one note or for every note in a folder.
//!
//! Both need the vault unlocked first, so they go through the password dialog
//! when it isn't. Open notes keep their tabs and text: the tab follows the
//! new file name, and the old file goes to the trash.

use std::path::PathBuf;

use dotnote_core::lock;
use gpui_kit::component::WindowExt as _;
use gpui_kit::{App, AppContext as _, Entity, Window};

use crate::ui::editor_area::EditorArea;
use crate::ui::unlock_dialog;
use crate::vault_lock::VaultLock;
use crate::vault_store::VaultStore;

/// Locks `path` if it's a normal note, or removes the lock if it's locked.
pub fn toggle(
    path: PathBuf,
    vault: Entity<VaultStore>,
    vault_lock: Entity<VaultLock>,
    editor_area: Entity<EditorArea>,
    window: &mut Window,
    cx: &mut App,
) {
    let lock_for_later = vault_lock.clone();
    let then = Box::new(move |window: &mut Window, cx: &mut App| {
        run(path, vault, lock_for_later, editor_area, window, cx);
    });
    unlock_dialog::open(vault_lock, Some(then), window, cx);
}

/// The vault is unlocked: do it.
fn run(
    path: PathBuf,
    vault: Entity<VaultStore>,
    vault_lock: Entity<VaultLock>,
    editor_area: Entity<EditorArea>,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(key) = vault_lock.read(cx).key() else {
        return;
    };
    // The note on disk must have the latest typing before it's converted.
    editor_area.update(cx, |area, cx| area.save_all(cx));
    let locking = !lock::is_locked_note(&path);
    let task_path = path.clone();
    let task = cx.background_spawn(async move {
        if locking {
            lock::lock_note(&key, &task_path)
        } else {
            lock::unlock_note(&key, &task_path)
        }
    });
    window
        .spawn(cx, async move |cx| {
            let result = task.await;
            let _ = cx.update(|window, cx| match result {
                Ok(new_path) => {
                    vault.update(cx, |vault, cx| vault.replace_note(&path, &new_path, cx));
                    let message = if locking {
                        "Note locked. The unlocked copy is in the trash; older versions \
                         may still be in the git history."
                    } else {
                        "Lock removed. The note is plain text again."
                    };
                    window.push_notification(message, cx);
                }
                Err(error) => window.push_notification(error.to_string(), cx),
            });
        })
        .detach();
}

/// Locks every note in `folder` (and its subfolders) if `locking`, or removes
/// their locks.
pub fn set_folder_lock(
    folder: PathBuf,
    locking: bool,
    vault: Entity<VaultStore>,
    vault_lock: Entity<VaultLock>,
    editor_area: Entity<EditorArea>,
    window: &mut Window,
    cx: &mut App,
) {
    let lock_for_later = vault_lock.clone();
    let then = Box::new(move |window: &mut Window, cx: &mut App| {
        run_folder(
            folder,
            locking,
            vault,
            lock_for_later,
            editor_area,
            window,
            cx,
        );
    });
    unlock_dialog::open(vault_lock, Some(then), window, cx);
}

/// The vault is unlocked: convert the folder's notes off the UI thread.
fn run_folder(
    folder: PathBuf,
    locking: bool,
    vault: Entity<VaultStore>,
    vault_lock: Entity<VaultLock>,
    editor_area: Entity<EditorArea>,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(key) = vault_lock.read(cx).key() else {
        return;
    };
    editor_area.update(cx, |area, cx| area.save_all(cx));
    let task = cx.background_spawn(async move { lock::convert_folder(&key, &folder, locking) });
    window
        .spawn(cx, async move |cx| {
            let change = task.await;
            let _ = cx.update(|window, cx| {
                if !change.converted.is_empty() {
                    vault.update(cx, |vault, cx| vault.replace_notes(&change.converted, cx));
                }
                window.push_notification(folder_message(&change, locking), cx);
            });
        })
        .detach();
}

/// The notification after a folder was locked or unlocked.
fn folder_message(change: &lock::FolderChange, locking: bool) -> String {
    let notes = |count: usize| if count == 1 { "note" } else { "notes" };
    let done = change.converted.len();
    let mut parts = Vec::new();
    if done > 0 && locking {
        parts.push(format!(
            "{done} {} locked. The unlocked copies are in the trash; older versions \
             may still be in the git history.",
            notes(done)
        ));
    } else if done > 0 {
        parts.push(format!("Lock removed from {done} {}.", notes(done)));
    }
    if let Some((path, error)) = change.failed.first() {
        let failed = change.failed.len();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let verb = if locking {
            "lock"
        } else {
            "remove the lock of"
        };
        parts.push(format!(
            "Couldn't {verb} {failed} {} (“{name}”: {error}).",
            notes(failed)
        ));
    }
    if parts.is_empty() {
        let none = if locking {
            "No notes to lock"
        } else {
            "No locked notes"
        };
        parts.push(format!("{none} in this folder."));
    }
    parts.join(" ")
}
