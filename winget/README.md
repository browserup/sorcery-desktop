# WinGet Manifests for Sorcery Desktop

This directory contains WinGet manifest files for publishing to the
[Windows Package Manager Community Repository](https://github.com/microsoft/winget-pkgs).

## Installation (once published)

```powershell
winget install ebeland.SorceryDesktop
```

## Publishing Process

1. Update manifests in this directory with new version
2. Validate with `winget validate --manifest <path>`
3. Submit PR to [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs)

## Automation

The `update-winget.yml` workflow automatically:
1. Creates new version manifests on release
2. Calculates SHA256 hash from MSI
3. Opens a PR to winget-pkgs (requires PAT with repo scope)

## Manual Submission

To submit manually using wingetcreate:

```powershell
wingetcreate update ebeland.SorceryDesktop --version 0.1.1 --urls https://github.com/ebeland/sorcery-desktop/releases/download/v0.1.1/Sorcery.Desktop_0.1.1_x64-setup.msi --submit
```

## Manifest Structure

```
manifests/e/ebeland/SorceryDesktop/<version>/
├── ebeland.SorceryDesktop.yaml           # Version manifest
├── ebeland.SorceryDesktop.installer.yaml # Installer details
└── ebeland.SorceryDesktop.locale.en-US.yaml # Localized metadata
```
