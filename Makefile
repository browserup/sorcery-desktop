.PHONY: help build build-release build-dmg build-linux install install-quick release test-protocol clean dev

help:
	@echo "Sorcery Desktop Development Makefile"
	@echo ""
	@echo "Available targets:"
	@echo "  make build          - Build debug version (no signing)"
	@echo "  make install        - Build and install to /Applications (macOS, no signing)"
	@echo "  make release        - Build signed, install, register, then notarize (macOS)"
	@echo "  make build-release  - Build release version"
	@echo "  make build-dmg      - Build signed/notarized DMG for distribution (macOS)"
	@echo "  make build-linux    - Build .deb/.rpm/.AppImage for distribution (Linux)"
	@echo "  make install-quick  - Install existing build (no rebuild)"
	@echo "  make test-protocol  - Test srcuri:// protocol handler"
	@echo "  make clean          - Clean build artifacts"
	@echo "  make dev            - Build, install, and test"
	@echo ""
	@echo "Windows: Run scripts/build-windows-release.ps1 in PowerShell"

build:
	@echo "==> Building debug version (no signing)..."
	cd src-tauri && env -u APPLE_CERTIFICATE -u APPLE_CERTIFICATE_PASSWORD -u APPLE_ID -u APPLE_PASSWORD -u APPLE_TEAM_ID -u APPLE_SIGNING_IDENTITY -u TAURI_SIGNING_PRIVATE_KEY -u TAURI_SIGNING_PRIVATE_KEY_PASSWORD cargo tauri build --debug --bundles app --config '{"bundle":{"createUpdaterArtifacts":false}}'

build-release:
	@echo "==> Building release version..."
	cd src-tauri && cargo tauri build

build-dmg:
	@echo "==> Building signed/notarized DMG for distribution..."
	@./scripts/build-macos-release.sh

build-linux:
	@echo "==> Building Linux packages..."
	@./scripts/build-linux-release.sh

install: build
	@echo ""
	@echo "==> Installing to /Applications..."
	@./scripts/quick-install-macos.sh

release:
	@echo "==> Building signed release (notarization deferred)..."
	cd src-tauri && env -u APPLE_ID -u APPLE_PASSWORD -u APPLE_TEAM_ID cargo tauri build --bundles app
	@echo ""
	@echo "==> Installing to /Applications..."
	@./scripts/quick-install-macos.sh release
	@echo ""
	@echo "==> Notarizing (this may take a few minutes)..."
	@./scripts/notarize-macos.sh

install-quick:
	@./scripts/quick-install-macos.sh

test-protocol:
	@echo "==> Testing protocol handler..."
	@echo "Opening /etc/hosts at line 1..."
	@open "srcuri:///etc/hosts@L1"
	@sleep 1
	@echo ""
	@echo "If your editor opened /etc/hosts, the protocol handler is working!"
	@echo "If not, check the console output for errors."

clean:
	@echo "==> Cleaning build artifacts..."
	cd src-tauri && cargo clean

dev: install test-protocol
	@echo ""
	@echo "✓ Development build installed and tested!"
