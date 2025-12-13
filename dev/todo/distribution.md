Tauri Linux Distribution Spec
1. Goals

Primary goal: Ship a Tauri v2 desktop app for Linux users across:

Ubuntu (LTS and current)

Debian (stable)

Fedora

Arch Linux

User experience goals:

Provide native-feeling install paths for each distro.

Provide at least one “works almost everywhere” artifact that can be downloaded from the website (AppImage).

Keep updates manageable: either OS-level (apt/dnf/pacman) or Tauri’s self-updater.

Dev experience goals:

Single Linux CI pipeline that builds all Linux artifacts.

Tauri configuration lives in tauri.conf.json (or .toml), versioned with code.
Tauri
+1

Non-goals (for now):

Official distro inclusion (Debian/Fedora main repos).

Enterprise-y features like code-signing enforcement on Linux.

2. Target Platforms & Packaging Strategy
   2.1 Distros & minimum versions

Ubuntu: Latest two LTS releases (e.g., 22.04, 24.04).

Debian: “stable” (current).

Fedora: Latest two releases.

Arch: Rolling.

2.2 Packaging Matrix
Distro	Primary format	Secondary / fallback
Ubuntu	.deb via Tauri bundler	AppImage; Snap (later)
Debian	.deb via Tauri bundler	AppImage
Fedora	.rpm via Tauri bundler	AppImage; Flatpak (later)
Arch	AUR package (PKGBUILD)	AppImage (direct download)

Tauri’s bundler can generate .deb, .rpm, .AppImage from the same build.
GitHub
+2
Tauri
+2

Tauri v2 docs explicitly list Linux distribution options: Debian, Snap, AppImage, Flatpak, RPM, AUR.
Tauri
+1

3. Build & CI Pipeline
   3.1 Build environment

Use Linux-native CI (e.g., GitHub Actions ubuntu-latest) because Tauri’s Linux bundles (.deb, AppImage, etc.) are built on Linux, not cross-compiled.
Tauri
+2
Tauri
+2

Install Tauri build prerequisites for Ubuntu (which covers our CI image):

libwebkit2gtk-4.1-dev

build-essential

curl, wget, libssl-dev, pkg-config

plus any Node toolchain required by the frontend.
Tauri

3.2 Build commands (CI steps)

High level CI steps:

Checkout repo.

Install Node + Rust toolchains.

Install Tauri CLI:
cargo install tauri-cli (unless using npx tauri).
Jonas Krückenberg
+1

Build frontend (e.g., npm install && npm run build).

Build + bundle Tauri app:

npm run tauri build or cargo tauri build

Optionally: tauri bundle as a separate step for re-bundling.
Tauri
+1

Output artifacts (in src-tauri/target/release/bundle):

deb/yourapp_X.Y.Z_amd64.deb

rpm/yourapp-X.Y.Z-1.x86_64.rpm

appimage/yourapp_X.Y.Z_amd64.AppImage
DEV Community
+2
Jonas Krückenberg
+2

Upload those artifacts to:

GitHub Releases (tag-based),

or your own download server.

4. Tauri Configuration Requirements

We standardize on tauri.conf.json for configuration. Key sections to define:

4.1 Metadata

Under app:

name, version, identifier (reverse-DNS style, e.g. com.srcuri.app).
Tauri
+1

description, category, authors.

4.2 Bundle configuration

In Tauri v2, bundle is a top-level config object; OS-specific configs live under bundle.linux.
Tauri
+1

Example sketch (JSON-ish):

{
"bundle": {
"identifier": "com.srcuri.app",
"icon": ["icons/32x32.png", "icons/128x128.png"],
"linux": {
"deb": {
"depends": [
"libwebkit2gtk-4.1-0",
"libgtk-3-0",
"libappindicator3-1"
]
},
"appimage": {
"bundleMediaFramework": true
},
"rpm": {
"depends": [
"webkit2gtk3",
"gtk3"
]
}
}
}
}


Dependencies for .deb are derived from the official Debian docs: libwebkit2gtk-4.1-0, libgtk-3-0, libappindicator3-1 (if tray is used).
Tauri

For RPM, we map to Fedora’s equivalent packages (e.g., webkit2gtk3, gtk3), confirmed in distro testing.

5. Packaging Per Distro
   5.1 Ubuntu & Debian – .deb packages

Strategy: Use Tauri’s .deb bundles as the primary distribution format; provide both direct download and optional apt repository.

5.1.1 What we rely on from Tauri

Tauri generates .deb that:

Declares the correct dependencies (libwebkit2gtk-4.1-0, libgtk-3-0, and optionally libappindicator3-1).
Tauri
+1

Installs a .desktop file + icons.
Tauri
+1

5.1.2 What we need to build

Testing matrix:

Install .deb on:

Ubuntu 22.04/24.04

Debian stable

Validate:

App launches

System tray (if used)

Single-instance plugin (if used; no Snap/Flatpak quirks here).
Tauri

Docs:

Installation instructions for .deb:

Direct: sudo dpkg -i yourapp_X.Y.Z_amd64.deb

Handle missing deps: sudo apt-get -f install.

Troubleshooting: missing libwebkit2gtk-4.1-0 or similar.

Optional (Phase 2):

Provide apt repository or PPA:

Use Launchpad, Cloudsmith, or a static apt repo.

CI step to push .deb + metadata.

5.2 AppImage – cross-distro fallback

Strategy: Always ship an AppImage as a “works on almost any distro” fallback.

AppImage bundles most dependencies and runs without installation; users just chmod +x and execute.
Tauri
+1

5.2.1 What we need

Build via Tauri (already done in CI).

Testing:

Confirm the AppImage runs on:

Ubuntu LTS

Debian stable

Fedora latest

Arch current

Smoke test: opening windows, menus, tray.

Docs:

Download + run steps:

chmod +x yourapp_X.Y.Z_amd64.AppImage
./yourapp_X.Y.Z_amd64.AppImage


Mention potential integration with AppImageLauncher, if users want menu entries.

5.3 Fedora – .rpm & Flatpak (later)

Strategy: Use Tauri’s RPM output for Fedora users; optionally augment with Flatpak/Flathub later.

5.3.1 RPM

Tauri bundler can create .rpm packages similar to .deb.
GitHub
+1

What we need:

Ensure bundle.linux.rpm is configured in tauri.conf.json (dependencies, summary, description).

CI builds .rpm along with .deb + AppImage.

Test on:

Fedora stable (latest, and previous if we care).

Install via:

sudo dnf install ./yourapp-X.Y.Z-1.x86_64.rpm


Verify WebView dependencies (typically webkit2gtk3, gtk3) are pulled in automatically.

Docs:

Installation instructions & troubleshooting for missing libs.

5.3.2 Flatpak / Flathub (Phase 2)

Tauri doesn’t output Flatpak directly; you create a Flatpak manifest that builds or consumes the .deb/binary.
GitHub
+2
Reddit
+2

Phase 2 tasks:

Create Flatpak manifest (com.srcuri.App.yml) that:

Uses SDK like org.gnome.Sdk + org.freedesktop.Sdk.Extension.rust-stable.

Builds or extracts app from .deb.

Sets required finish-args, especially if using SingleInstance plugin (DBus --talk-name/--own-name).
Tauri

Submit to Flathub or host locally.

5.4 Arch – AUR package

Strategy: Provide a PKGBUILD in a dedicated repo, publish to AUR.

Tauri docs provide a guide for publishing to AUR.
Tauri
+1

What we need:

Packaging choice:

Option A: AUR builds from source (Rust + Node toolchains).

Option B: AUR package that just downloads & installs AppImage or .tar.gz containing the binary.
(A is more “Arch-y”; B is simpler for us but heavier for users at build time.)

Create PKGBUILD:

Defines pkgname, pkgver, source, depends (e.g., webkit2gtk, gtk3 if using system libs; or minimal if wrapping AppImage).

Test on Arch:

makepkg -si from the PKGBUILD repo.

AUR publishing:

Follow AUR workflow: git push to AUR git URL, as per Tauri’s AUR doc.
Tauri

6. Optional: Snap (Ubuntu) & Others

Not required for initial distro support, but good to acknowledge.

6.1 Snap

Tauri provides a Snapcraft guide with a sample snapcraft.yaml.
Tauri
+1

Strategy:

Provide Snap only after .deb and AppImage are stable.

Ensure snap size is acceptable; Snap can bloat if configured poorly.

6.2 Standalone binary

Some users may want just the raw binary (no AppImage). It’s possible but requires documenting dependencies and path layout; this is an extra and not central to our distro story.
GitHub
+1

7. Release Process

For each tagged release (vX.Y.Z):

CI builds:

.deb, .rpm, .AppImage for x86_64.

Attach artifacts to GitHub Release (or similar).

Update:

Download page on the website with:

Ubuntu/Debian: .deb (fallback AppImage).

Fedora: .rpm (fallback AppImage).

Arch: “Install from AUR: yay -S srcuri” (or similar) + AppImage fallback.

Optional:

Push .deb to apt repo, .rpm to your dnf repo, update AUR PKGBUILD version.

8. Risks & Open Questions

WebKit dependency churn:

On Debian/Ubuntu, package names (libwebkit2gtk-4.1-0) may evolve; we must keep an eye on the Tauri docs that define default deb dependencies.
Tauri
+1

AppImage quirks:

The HN thread you linked shows some people dropping AppImage in favor of .deb due to stability/infra issues; we should treat AppImage as a convenient fallback, not the only path.

TODO: Auto-updates vs OS updates:

Decide whether Linux users get updates primarily via:
OS package managers (apt/dnf/pacman), or
Tauri’s built-in auto-updater (more common on Windows/macOS).
Docs.rs