//! Build script: on Windows, embeds the app icon into `dotnote.exe`
//! so Explorer and the taskbar show it. Does nothing on other platforms.

fn main() {
    // `CARGO_CFG_TARGET_OS` is the platform we build *for* (works when cross-compiling too).
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_windows_icon();
    }
}

#[cfg(windows)]
fn embed_windows_icon() {
    println!("cargo:rerun-if-changed=../../packaging/windows/dotnote.ico");
    let mut resource = winresource::WindowsResource::new();
    resource.set_icon("../../packaging/windows/dotnote.ico");
    resource.set("ProductName", "Dotnote");
    resource.set("FileDescription", "Dotnote");
    if let Err(error) = resource.compile() {
        // A missing icon must not break the build; the exe just keeps the default icon.
        println!("cargo:warning=couldn't embed the Windows icon: {error}");
    }
}

#[cfg(not(windows))]
fn embed_windows_icon() {
    // Cross-compiling to Windows from another OS: skipped (needs the Windows resource compiler).
}
