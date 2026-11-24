//! Putting a downloaded update in place of the running app, and knowing how
//! this copy of Dotnote was installed (which decides how it updates).
//! Finding and downloading releases is in `update.rs`.
//!
//! Installing, per kind (see [`InstallKind`]):
//! - **macOS app:** the new `Dotnote.app` replaces the running one (renaming a
//!   running app is allowed; it keeps running from the old files).
//! - **Windows exe:** the new `Dotnote.exe` replaces the running one, which is
//!   renamed to `Dotnote.exe.old` first (Windows can rename, but not delete, a
//!   running exe). The leftover is removed on the next launch.
//! - **rpm from GitHub:** installed with `pkexec dnf install`, which asks for
//!   the administrator password.
//! - **AppImage:** the new `.AppImage` file replaces the running one (Linux
//!   keeps running the old file's contents until the app quits).
//! - **dnf repository (Copr):** never by Dotnote; `dnf upgrade` updates it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::error::{CoreError, Result};
use crate::update::{last_line, parent_of, remove_if_exists, run};

/// Where the rpm installs Dotnote on Fedora.
const LINUX_INSTALL_PATH: &str = "/usr/bin/dotnote";

/// Set at build time by the Copr package (`packaging/linux/dotnote.spec`):
/// that copy is updated by dnf from the Copr repository.
const PACKAGED_BY: Option<&str> = option_env!("DOTNOTE_PACKAGED_BY");

/// How this copy of Dotnote was installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallKind {
    MacApp,
    WindowsExe,
    /// The rpm attached to a GitHub release, at `/usr/bin/dotnote`.
    GithubRpm,
    /// Installed by dnf from the Copr repository; `dnf upgrade` updates it.
    DnfRepo,
    /// An AppImage; the path is the `.AppImage` file itself.
    AppImage(PathBuf),
    /// A build Dotnote can't update (e.g. `cargo run` on Linux).
    Other,
}

impl InstallKind {
    /// Works out the kind from what the system tells us: the OS name, the
    /// running exe, the `APPIMAGE` variable (set by the AppImage runtime to the
    /// `.AppImage` file), and `packaged_by` ([`PACKAGED_BY`]).
    pub fn detect(
        os: &str,
        current_exe: &Path,
        appimage: Option<PathBuf>,
        packaged_by: Option<&str>,
    ) -> InstallKind {
        match os {
            "macos" => InstallKind::MacApp,
            "windows" => InstallKind::WindowsExe,
            "linux" => match (appimage, packaged_by) {
                (Some(file), _) if !file.as_os_str().is_empty() => InstallKind::AppImage(file),
                (_, Some("copr")) => InstallKind::DnfRepo,
                _ if current_exe == Path::new(LINUX_INSTALL_PATH) => InstallKind::GithubRpm,
                _ => InstallKind::Other,
            },
            _ => InstallKind::Other,
        }
    }

    /// The kind of the copy that is running now.
    pub fn current() -> InstallKind {
        let exe = std::env::current_exe().unwrap_or_default();
        let appimage = std::env::var_os("APPIMAGE").map(PathBuf::from);
        InstallKind::detect(std::env::consts::OS, &exe, appimage, PACKAGED_BY)
    }

    /// The release asset this kind updates from on `arch`, or `None` if it
    /// doesn't update itself.
    pub fn package_name(&self, arch: &str) -> Option<&'static str> {
        match (self, arch) {
            (InstallKind::MacApp, "aarch64") => Some("Dotnote-macos-arm64.zip"),
            (InstallKind::MacApp, "x86_64") => Some("Dotnote-macos-x64.zip"),
            (InstallKind::WindowsExe, "x86_64") => Some("Dotnote-windows-x64.zip"),
            (InstallKind::GithubRpm, "x86_64") => Some("Dotnote-fedora-x86_64.rpm"),
            (InstallKind::AppImage(_), "x86_64") => Some("Dotnote-x86_64.AppImage"),
            _ => None,
        }
    }
}

/// Installs a downloaded package in place of the running app (`current_exe`).
/// Returns the path to start the new version from.
pub fn install(kind: &InstallKind, package: &Path, current_exe: &Path) -> Result<PathBuf> {
    match kind {
        InstallKind::MacApp => install_macos(package, current_exe),
        InstallKind::WindowsExe => install_windows(package, current_exe),
        InstallKind::GithubRpm => install_rpm(package),
        InstallKind::AppImage(file) => install_appimage(package, file),
        InstallKind::DnfRepo => Err(CoreError::Update(
            "This copy is updated by dnf: run `sudo dnf upgrade --refresh dotnote`.".into(),
        )),
        InstallKind::Other => Err(CoreError::Update(
            "This copy of Dotnote can't update itself. Download the new version from the release page.".into(),
        )),
    }
}

fn install_macos(zip: &Path, current_exe: &Path) -> Result<PathBuf> {
    // The exe is at Dotnote.app/Contents/MacOS/dotnote.
    let bundle = current_exe
        .ancestors()
        .find(|path| path.extension().is_some_and(|ext| ext == "app"))
        .ok_or_else(|| {
            CoreError::Update("Only Dotnote.app can update itself (this is a dev build).".into())
        })?
        .to_path_buf();
    let folder = parent_of(&bundle)?;
    // Unpack next to the app, so the swap below is a rename on the same disk.
    let staging = folder.join(".Dotnote-update");
    remove_if_exists(&staging)?;
    run(Command::new("ditto")
        .arg("-x")
        .arg("-k")
        .arg(zip)
        .arg(&staging))?;
    let new_app = staging.join("Dotnote.app");
    if !new_app.join("Contents/MacOS/dotnote").is_file() {
        return Err(CoreError::Update(
            "The update package doesn't contain Dotnote.app.".into(),
        ));
    }
    let old = folder.join(".Dotnote-old.app");
    replace_with(&bundle, &new_app, &old)?;
    remove_if_exists(&old)?;
    remove_if_exists(&staging)?;
    Ok(bundle)
}

fn install_windows(zip: &Path, current_exe: &Path) -> Result<PathBuf> {
    let folder = parent_of(current_exe)?;
    let staging = folder.join(".dotnote-update");
    remove_if_exists(&staging)?;
    fs::create_dir_all(&staging)?;
    // `tar` ships with Windows 10 and later, and reads zip files.
    run(Command::new("tar")
        .arg("-xf")
        .arg(zip)
        .arg("-C")
        .arg(&staging))?;
    let new_exe = staging.join("Dotnote.exe");
    if !new_exe.is_file() {
        return Err(CoreError::Update(
            "The update package doesn't contain Dotnote.exe.".into(),
        ));
    }
    let old = old_exe_path(current_exe);
    remove_if_exists(&old)?; // from an earlier update
    replace_with(current_exe, &new_exe, &old)?;
    remove_if_exists(&staging)?;
    // `old` is still running; it's removed on the next launch.
    Ok(current_exe.to_path_buf())
}

fn install_rpm(rpm: &Path) -> Result<PathBuf> {
    // pkexec shows the system's password prompt, then dnf upgrades the package.
    let output = Command::new("pkexec")
        .args(["dnf", "install", "-y"])
        .arg(rpm)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| CoreError::Update(format!("Couldn't run pkexec: {error}")))?;
    match output.status.code() {
        Some(0) => Ok(PathBuf::from(LINUX_INSTALL_PATH)),
        // pkexec: 126 = the password prompt was dismissed, 127 = not authorized.
        Some(126 | 127) => Err(CoreError::Update("The update was cancelled.".into())),
        _ => Err(CoreError::Update(format!(
            "dnf couldn't install the update: {}",
            last_line(&output.stderr)
        ))),
    }
}

/// Copies the new AppImage next to the running one, makes it executable, then
/// renames it over the old file in one step. The running app keeps the old
/// file's contents open, so it keeps working until it restarts.
pub fn install_appimage(new: &Path, target: &Path) -> Result<PathBuf> {
    let folder = parent_of(target)?;
    // The download may be on another disk (`/tmp`); a rename only works within one.
    let staging = folder.join(".Dotnote-update.AppImage");
    let cant_write = |error: std::io::Error| {
        CoreError::Update(format!(
            "Couldn't write to {} ({error}). Move Dotnote's AppImage to a folder you own, \
             like ~/Applications, or download the new version from the release page.",
            folder.display()
        ))
    };
    fs::copy(new, &staging).map_err(cant_write)?;
    make_executable(&staging)?;
    if let Err(error) = fs::rename(&staging, target) {
        remove_if_exists(&staging)?;
        return Err(cant_write(error));
    }
    Ok(target.to_path_buf())
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    Ok(())
}

/// Other systems have no executable bit (AppImages only run on Linux anyway).
#[cfg(not(unix))]
fn make_executable(_: &Path) -> Result<()> {
    Ok(())
}

/// Puts `new` at `target`, moving what was there to `old`. If the second
/// rename fails, the original is moved back so the app is never left missing.
pub fn replace_with(target: &Path, new: &Path, old: &Path) -> Result<()> {
    fs::rename(target, old)?;
    if let Err(error) = fs::rename(new, target) {
        return Err(match fs::rename(old, target) {
            Ok(()) => error.into(),
            Err(restore) => CoreError::Update(format!(
                "Couldn't install the update ({error}) or put the old version back ({restore}). It's at {}.",
                old.display()
            )),
        });
    }
    Ok(())
}

/// Where Windows keeps the replaced exe until the next launch: `Dotnote.exe.old`.
pub fn old_exe_path(exe: &Path) -> PathBuf {
    let mut name = exe.file_name().unwrap_or_default().to_os_string();
    name.push(".old");
    exe.with_file_name(name)
}

/// Removes what an earlier update left behind (Windows' old exe). Call at startup.
pub fn clean_up_after_update(current_exe: &Path) -> Result<()> {
    remove_if_exists(&old_exe_path(current_exe))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linux(exe: &str, appimage: Option<&str>, packaged_by: Option<&str>) -> InstallKind {
        InstallKind::detect(
            "linux",
            Path::new(exe),
            appimage.map(PathBuf::from),
            packaged_by,
        )
    }

    #[test]
    fn detects_how_linux_copies_were_installed() {
        assert_eq!(
            linux(
                "/tmp/.mount_x/usr/bin/dotnote",
                Some("/home/me/Dotnote.AppImage"),
                None
            ),
            InstallKind::AppImage(PathBuf::from("/home/me/Dotnote.AppImage"))
        );
        assert_eq!(
            linux("/usr/bin/dotnote", None, Some("copr")),
            InstallKind::DnfRepo
        );
        assert_eq!(
            linux("/usr/bin/dotnote", None, None),
            InstallKind::GithubRpm
        );
        assert_eq!(
            linux("/home/me/dotnote/target/debug/dotnote", None, None),
            InstallKind::Other
        );
        // An empty APPIMAGE variable isn't an AppImage.
        assert_eq!(
            linux("/usr/bin/dotnote", Some(""), None),
            InstallKind::GithubRpm
        );
    }

    #[test]
    fn one_package_per_kind() {
        let mac = InstallKind::detect("macos", Path::new("/x"), None, None);
        assert_eq!(mac.package_name("aarch64"), Some("Dotnote-macos-arm64.zip"));
        let windows = InstallKind::detect("windows", Path::new("C:/x.exe"), None, None);
        assert_eq!(
            windows.package_name("x86_64"),
            Some("Dotnote-windows-x64.zip")
        );
        assert_eq!(
            InstallKind::GithubRpm.package_name("x86_64"),
            Some("Dotnote-fedora-x86_64.rpm")
        );
        let appimage = InstallKind::AppImage(PathBuf::from("/a.AppImage"));
        assert_eq!(
            appimage.package_name("x86_64"),
            Some("Dotnote-x86_64.AppImage")
        );
        assert_eq!(appimage.package_name("aarch64"), None);
        // Copies dnf updates, and dev builds, download nothing.
        assert_eq!(InstallKind::DnfRepo.package_name("x86_64"), None);
        assert_eq!(InstallKind::Other.package_name("x86_64"), None);
    }

    #[test]
    fn old_exe_keeps_its_name() {
        assert_eq!(
            old_exe_path(Path::new("C:/Apps/Dotnote.exe")),
            Path::new("C:/Apps/Dotnote.exe.old")
        );
    }
}
