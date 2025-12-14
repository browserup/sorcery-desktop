# Distribution & CI/CD Plan

Based on `dev/todo/distribution.md` spec. Goal: Automated builds for macOS, Windows, Linux with GitHub Releases.

## Current State

- No CI/CD exists (no `.github/workflows/`)
- `tauri.conf.json` lacks Linux bundle configuration
- No package.json - pure Rust + static HTML frontend
- Uses: tray-icon, deep-link, single-instance, dialog, fs plugins
- Repository: `https://github.com/ebeland/sorcery-desktop`

---

## Phase 1: Tauri Configuration

- [ ] Update `tauri.conf.json` with Linux bundle config
  - Add `bundle.linux.deb.depends` (libwebkit2gtk-4.1-0, libgtk-3-0, libappindicator3-1)
  - Add `bundle.linux.rpm.depends` (webkit2gtk3, gtk3)
  - Add `bundle.linux.appimage.bundleMediaFramework: true`
- [ ] Add required icons for all platforms
  - Check existing `icons/icon.png` dimensions
  - Generate: 32x32, 128x128, 256x256, 512x512 PNGs for Linux
  - Generate: .icns for macOS (if missing)
  - Generate: .ico for Windows (if missing)
- [ ] Update `bundle.category` and metadata fields

---

## Phase 2: GitHub Actions - Build Workflow

Create `.github/workflows/build.yml`:

- [ ] Set up workflow triggers (push to main, PRs)
- [ ] Linux job (ubuntu-latest)
  - Install deps: libwebkit2gtk-4.1-dev, libgtk-3-dev, libappindicator3-dev, librsvg2-dev
  - Install Rust toolchain
  - Build with `cargo tauri build`
  - Output: .deb, .rpm, .AppImage
- [ ] macOS job (macos-latest)
  - Install Rust toolchain
  - Build with `cargo tauri build`
  - Output: .app, .dmg
- [ ] Windows job (windows-latest)
  - Install Rust toolchain
  - Build with `cargo tauri build`
  - Output: .msi, .exe

---

## Phase 3: GitHub Actions - Release Workflow

Create `.github/workflows/release.yml`:

- [ ] Trigger on version tags (v*)
- [ ] Build all platforms (reuse build jobs)
- [ ] Create GitHub Release
- [ ] Upload artifacts:
  - `sorcery-desktop_X.Y.Z_amd64.deb`
  - `sorcery-desktop-X.Y.Z-1.x86_64.rpm`
  - `sorcery-desktop_X.Y.Z_amd64.AppImage`
  - `Sorcery Desktop.dmg` (macOS)
  - `Sorcery Desktop_X.Y.Z_x64_en-US.msi` (Windows)
- [ ] Generate release notes from commits

---

## Phase 4: Code Signing (Optional but Recommended)

- [ ] macOS: Apple Developer certificate for notarization
  - Add `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD` secrets
  - Configure `bundle.macOS.signingIdentity` in tauri.conf.json
- [ ] Windows: Code signing certificate
  - Add certificate secrets
  - Configure in tauri.conf.json
- [ ] Linux: No signing required (GPG optional for apt repos)

---

## Phase 5: Testing in CI

- [ ] Add `cargo test` step before build
- [ ] Add `cargo clippy` for linting
- [ ] Add `cargo fmt --check` for formatting
- [ ] Consider smoke test for AppImage (run in headless mode if possible)

---

## Phase 6: Documentation

- [ ] Update README.md with installation instructions per platform
- [ ] Add INSTALL.md download links pointing to GitHub Releases
- [ ] Add release badge to README

---

## Phase 7: Future (Per distribution.md)

Defer to later:
- [ ] AUR package (PKGBUILD)
- [ ] Snap package
- [ ] Flatpak / Flathub
- [ ] apt/dnf repositories

---

## Implementation Order

1. **Icons** - Required before any bundling works properly
2. **tauri.conf.json** - Add Linux bundle config
3. **build.yml** - Get CI builds working on all platforms
4. **release.yml** - Automate releases
5. **Tests in CI** - Quality gates
6. **Docs** - User-facing instructions
7. **Code signing** - Can be added later

---

## Key Decisions Needed

1. **Version source of truth**: Use `Cargo.toml` or `tauri.conf.json`? (Recommend: keep in sync, or use tauri.conf.json as primary)
2. **Branch strategy**: Build on `main` only, or also feature branches?
3. **Code signing priority**: Do it now or defer?
4. **macOS architectures**: Build for both x86_64 and arm64 (Apple Silicon)?
