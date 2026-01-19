# Distribution Build Steps

## Files Created/Modified

| File | Purpose |
|------|---------|
| `scripts/build-macos-release.sh` | Local macOS release build (universal binary) |
| `.github/workflows/release.yml` | Automated builds on `v*` tags for all platforms |
| `.github/workflows/ci.yml` | Tests, linting, and build checks on PRs |
| `src-tauri/tauri.conf.json` | Added updater config, Windows wix config |
| `src-tauri/Cargo.toml` | Added `tauri-plugin-updater` |
| `src-tauri/src/main.rs` | Initialize updater plugin |
| `Makefile` | Added `make build-dmg` target |

## Build Matrix

| Platform | Architecture | Artifact |
|----------|--------------|----------|
| macOS | arm64 (Apple Silicon) | `.dmg` |
| macOS | x86_64 (Intel) | `.dmg` |
| Linux | x86_64 | `.deb`, `.rpm`, `.AppImage` |
| Linux | arm64 | `.deb`, `.rpm`, `.AppImage` |
| Windows | x86_64 | `.msi` |

## Auto-Updater

The app includes Tauri's updater plugin. Users receive update notifications when a new release is published.

- **Endpoint**: `https://github.com/ebeland/sorcery-desktop/releases/latest/download/latest.json`
- **Windows install mode**: Passive (quiet install)
- **Requires**: `TAURI_SIGNING_PRIVATE_KEY` secret for signing updates

## Setup Steps

### 1. Generate Updater Signing Key (One-Time)

```bash
# Generate a new key pair
cargo tauri signer generate -w ~/.tauri/sorcery-desktop.key

# This outputs:
# - Private key (save to TAURI_SIGNING_PRIVATE_KEY secret)
# - Public key (replace UPDATER_PUBKEY_PLACEHOLDER in tauri.conf.json)
```

### 2. Apple Developer Setup (Manual)

1. **Enroll** at [developer.apple.com/programs](https://developer.apple.com/programs) ($99/year)
2. **Create certificate**: Keychain Access → Certificate Assistant → "Developer ID Application"
3. **App-specific password**: [appleid.apple.com](https://appleid.apple.com) → Security → App-Specific Passwords
4. **Note Team ID**: Apple Developer membership page

### 3. Configure GitHub Secrets

In your repo → Settings → Secrets and variables → Actions, add:

| Secret | Value |
|--------|-------|
| `APPLE_CERTIFICATE` | Base64-encoded .p12 (`base64 -i cert.p12 -o cert.txt`) |
| `APPLE_CERTIFICATE_PASSWORD` | Password for .p12 |
| `APPLE_ID` | Your Apple ID email |
| `APPLE_PASSWORD` | App-specific password |
| `APPLE_TEAM_ID` | Team ID |
| `APPLE_SIGNING_IDENTITY` | e.g., `Developer ID Application: Your Name (TEAM_ID)` |
| `TAURI_SIGNING_PRIVATE_KEY` | Private key from step 1 |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password for private key (if set) |

### 4. Update Public Key in Config

After generating the signing key, replace `UPDATER_PUBKEY_PLACEHOLDER` in `src-tauri/tauri.conf.json` with the actual public key.

### 5. Test Local Build

```bash
make build-dmg
```

This builds without signing (warns about missing credentials).

### 6. Create a Release

```bash
git tag v0.1.0
git push origin v0.1.0
```

GitHub Actions will:
1. Build for all platforms (5 targets)
2. Sign macOS builds (if secrets configured)
3. Sign update artifacts
4. Create a draft release with all artifacts
5. Publish the release

## Workflow Reference

### release.yml Triggers
- Push tag `v*` (e.g., `v0.1.0`, `v1.2.3`)
- Manual dispatch with optional dry-run

### ci.yml Triggers
- Push to `main`
- Pull requests to `main`

## Artifacts Per Release

Each release includes:
- `Sorcery.Desktop_X.Y.Z_aarch64.dmg` (macOS ARM)
- `Sorcery.Desktop_X.Y.Z_x64.dmg` (macOS Intel)
- `sorcery-desktop_X.Y.Z_amd64.deb` (Linux)
- `sorcery-desktop_X.Y.Z_amd64.rpm` (Linux)
- `sorcery-desktop_X.Y.Z_amd64.AppImage` (Linux)
- `Sorcery.Desktop_X.Y.Z_x64_en-US.msi` (Windows)
- `latest.json` (updater manifest)
