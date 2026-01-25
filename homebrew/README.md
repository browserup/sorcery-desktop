# Homebrew Tap for Sorcery Desktop

This directory contains Homebrew cask formulas for Sorcery Desktop.

## Installation

```bash
# Add the tap
brew tap ebeland/sorcery https://github.com/ebeland/sorcery-desktop

# Install Sorcery Desktop
brew install --cask ebeland/sorcery/sorcery-desktop
```

Or install directly without adding the tap:

```bash
brew install --cask https://raw.githubusercontent.com/ebeland/sorcery-desktop/main/homebrew/Casks/sorcery-desktop.rb
```

## Updating

```bash
brew upgrade --cask sorcery-desktop
```

## Uninstalling

```bash
brew uninstall --cask sorcery-desktop
```

## Development

To test locally:

```bash
brew install --cask ./homebrew/Casks/sorcery-desktop.rb
```

## Automation

The cask formula is automatically updated on new releases via GitHub Actions.
The workflow updates the version and SHA256 hashes in the formula.
