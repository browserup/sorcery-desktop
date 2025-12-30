.PHONY: help build build-release build-dmg install install-quick test-protocol clean dev

help:
	@echo "Sorcery Desktop Development Makefile"
	@echo ""
	@echo "Available targets:"
	@echo "  make build          - Build debug version"
	@echo "  make build-release  - Build release version"
	@echo "  make build-dmg      - Build signed/notarized DMG for distribution (macOS)"
	@echo "  make install        - Build and install to /Applications (macOS)"
	@echo "  make install-quick  - Install existing build (no rebuild)"
	@echo "  make test-protocol  - Test srcuri:// protocol handler"
	@echo "  make clean          - Clean build artifacts"
	@echo "  make dev            - Build, install, and test"

build:
	@echo "==> Building debug version (app bundle only)..."
	cd src-tauri && cargo tauri build --debug --bundles app

build-release:
	@echo "==> Building release version..."
	cd src-tauri && cargo tauri build

build-dmg:
	@echo "==> Building signed/notarized DMG for distribution..."
	@./scripts/build-macos-release.sh

install: build
	@echo ""
	@echo "==> Installing to /Applications..."
	@./scripts/quick-install-macos.sh

install-quick:
	@./scripts/quick-install-macos.sh

test-protocol:
	@echo "==> Testing protocol handler..."
	@echo "Opening /etc/hosts at line 1..."
	@open "srcuri:///etc/hosts:1"
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
