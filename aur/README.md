# AUR Package for Sorcery Desktop

This directory contains the PKGBUILD for the [Arch User Repository (AUR)](https://aur.archlinux.org/).

## Installation (once published)

```bash
# Using yay
yay -S sorcery-desktop-bin

# Using paru
paru -S sorcery-desktop-bin

# Manual installation
git clone https://aur.archlinux.org/sorcery-desktop-bin.git
cd sorcery-desktop-bin
makepkg -si
```

## Package Details

- **Package name**: `sorcery-desktop-bin`
- **Type**: Binary package (downloads AppImage from GitHub releases)
- **Architectures**: x86_64, aarch64

## Publishing to AUR

1. Create an AUR account at https://aur.archlinux.org
2. Add your SSH public key to your AUR account
3. Clone the AUR repo:
   ```bash
   git clone ssh://aur@aur.archlinux.org/sorcery-desktop-bin.git
   ```
4. Copy PKGBUILD and .SRCINFO to the cloned repo
5. Update SHA256 sums with actual values
6. Commit and push

## Updating the Package

1. Update `pkgver` in PKGBUILD
2. Update source URLs with new version
3. Calculate new SHA256 sums:
   ```bash
   sha256sum sorcery-desktop_0.1.1_amd64.AppImage
   sha256sum sorcery-desktop_0.1.1_aarch64.AppImage
   ```
4. Regenerate .SRCINFO:
   ```bash
   makepkg --printsrcinfo > .SRCINFO
   ```
5. Commit and push to AUR

## Testing Locally

```bash
makepkg -si
```
