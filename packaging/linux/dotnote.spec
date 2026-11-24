# Fedora package for the Copr repository (kurodev/dotnote), built from source.
# Copr makes the source rpm with `.copr/Makefile`, which fills in @VERSION@
# from Cargo.toml. Builds need network access (cargo downloads the crates):
# turn on "Enable internet access during builds" in the Copr project settings.
#
# DOTNOTE_PACKAGED_BY=copr tells the app that dnf updates it, so its updater
# only says how to update instead of installing anything itself.

# A release build has no debug symbols to split off.
%global debug_package %{nil}

Name:           dotnote
Version:        @VERSION@
Release:        1%{?dist}
Summary:        A minimal, cross-platform markdown notes app
License:        MIT
URL:            https://github.com/sofyan-rs/dotnote-gpui
Source0:        dotnote-%{version}.tar.gz
ExclusiveArch:  x86_64 aarch64

# rust-version in Cargo.toml.
BuildRequires:  rust >= 1.92
BuildRequires:  cargo >= 1.92
# The same list as docs/CONTRIBUTION.md (Linux) and the release workflow.
BuildRequires:  gcc
BuildRequires:  gcc-c++
BuildRequires:  clang
BuildRequires:  cmake
BuildRequires:  pkgconf-pkg-config
BuildRequires:  fontconfig-devel
BuildRequires:  wayland-devel
BuildRequires:  webkit2gtk4.1-devel
BuildRequires:  libxkbcommon-x11-devel
BuildRequires:  libxcb-devel
BuildRequires:  libX11-devel
BuildRequires:  openssl-devel
BuildRequires:  libzstd-devel
BuildRequires:  vulkan-loader-devel
BuildRequires:  desktop-file-utils

Requires:       vulkan-loader
# The update check runs curl; git sync runs git (optional).
Requires:       curl
Recommends:     git

%description
Dotnote opens a folder on disk as a workspace, shows it in a sidebar, and
edits the markdown files in it directly: Live, Source, Split and Preview
modes, tabs, locked (encrypted) notes, and optional sync through your own
git repository. Files on disk are the only source of truth.

%prep
%autosetup -n dotnote-%{version}

%build
export DOTNOTE_PACKAGED_BY=copr
cargo build --release --locked -p dotnote

%install
install -Dm755 target/release/dotnote %{buildroot}%{_bindir}/dotnote
install -Dm644 packaging/linux/id.sofyan.Dotnote.desktop \
    %{buildroot}%{_datadir}/applications/id.sofyan.Dotnote.desktop
for size in 16 32 48 64 128 256 512; do
    install -Dm644 packaging/linux/icons/hicolor/${size}x${size}/apps/dotnote.png \
        %{buildroot}%{_datadir}/icons/hicolor/${size}x${size}/apps/dotnote.png
done

%check
desktop-file-validate %{buildroot}%{_datadir}/applications/id.sofyan.Dotnote.desktop

%files
%license LICENSE
%doc README.md
%{_bindir}/dotnote
%{_datadir}/applications/id.sofyan.Dotnote.desktop
%{_datadir}/icons/hicolor/*/apps/dotnote.png

%changelog
* Wed Nov 25 2026 Sofyan R. - @VERSION@-1
- Built from the v@VERSION@ tag; see the GitHub release for changes.
