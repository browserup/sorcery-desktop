#!/bin/bash
set -euo pipefail

# Sorcery Desktop Installer
# Usage: curl -fsSL https://getsorcery.com/install.sh | sh

REPO="browserup/sorcery-desktop"
APP_NAME="Sorcery Desktop"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

info() {
    printf "${BLUE}==>${NC} %s\n" "$1"
}

success() {
    printf "${GREEN}==>${NC} %s\n" "$1"
}

warn() {
    printf "${YELLOW}Warning:${NC} %s\n" "$1"
}

error() {
    printf "${RED}Error:${NC} %s\n" "$1" >&2
    exit 1
}

detect_os() {
    case "$(uname -s)" in
        Darwin)
            echo "macos"
            ;;
        Linux)
            echo "linux"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            error "Windows is not supported by this installer. Download the MSI from https://github.com/$REPO/releases"
            ;;
        *)
            error "Unsupported operating system: $(uname -s)"
            ;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)
            echo "x64"
            ;;
        aarch64|arm64)
            echo "arm64"
            ;;
        *)
            error "Unsupported architecture: $(uname -m)"
            ;;
    esac
}

detect_linux_distro() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        case "$ID" in
            debian|ubuntu|pop|linuxmint|elementary|zorin)
                echo "deb"
                ;;
            fedora|rhel|centos|rocky|alma)
                echo "rpm"
                ;;
            arch|manjaro|endeavouros)
                echo "appimage"
                ;;
            *)
                if [ -n "${ID_LIKE:-}" ]; then
                    case "$ID_LIKE" in
                        *debian*|*ubuntu*)
                            echo "deb"
                            ;;
                        *fedora*|*rhel*)
                            echo "rpm"
                            ;;
                        *)
                            echo "appimage"
                            ;;
                    esac
                else
                    echo "appimage"
                fi
                ;;
        esac
    else
        echo "appimage"
    fi
}

get_latest_version() {
    local api_url="https://api.github.com/repos/$REPO/releases/latest"

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$api_url" | grep '"tag_name":' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
    elif command -v wget >/dev/null 2>&1; then
        wget -qO- "$api_url" | grep '"tag_name":' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
    else
        error "Neither curl nor wget found. Please install one of them."
    fi
}

download_file() {
    local url="$1"
    local output="$2"

    info "Downloading from $url"

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL -o "$output" "$url"
    elif command -v wget >/dev/null 2>&1; then
        wget -q -O "$output" "$url"
    else
        error "Neither curl nor wget found. Please install one of them."
    fi
}

get_artifact_url() {
    local version="$1"
    local os="$2"
    local arch="$3"
    local format="$4"

    local version_num="${version#v}"
    local base_url="https://github.com/$REPO/releases/download/$version"

    case "$os" in
        macos)
            if [ "$arch" = "arm64" ]; then
                echo "$base_url/Sorcery.Desktop_${version_num}_aarch64.dmg"
            else
                echo "$base_url/Sorcery.Desktop_${version_num}_x64.dmg"
            fi
            ;;
        linux)
            case "$format" in
                deb)
                    if [ "$arch" = "arm64" ]; then
                        echo "$base_url/sorcery-desktop_${version_num}_arm64.deb"
                    else
                        echo "$base_url/sorcery-desktop_${version_num}_amd64.deb"
                    fi
                    ;;
                rpm)
                    if [ "$arch" = "arm64" ]; then
                        echo "$base_url/sorcery-desktop-${version_num}-1.aarch64.rpm"
                    else
                        echo "$base_url/sorcery-desktop-${version_num}-1.x86_64.rpm"
                    fi
                    ;;
                appimage)
                    if [ "$arch" = "arm64" ]; then
                        echo "$base_url/sorcery-desktop_${version_num}_aarch64.AppImage"
                    else
                        echo "$base_url/sorcery-desktop_${version_num}_amd64.AppImage"
                    fi
                    ;;
            esac
            ;;
    esac
}

install_macos() {
    local dmg_path="$1"
    local mount_point="/Volumes/Sorcery Desktop"

    info "Mounting DMG..."
    hdiutil attach "$dmg_path" -nobrowse -quiet

    info "Installing to /Applications..."
    if [ -d "/Applications/$APP_NAME.app" ]; then
        warn "Removing existing installation..."
        rm -rf "/Applications/$APP_NAME.app"
    fi

    cp -R "$mount_point/$APP_NAME.app" /Applications/

    info "Unmounting DMG..."
    hdiutil detach "$mount_point" -quiet

    rm "$dmg_path"

    success "$APP_NAME installed to /Applications"
}

install_linux_deb() {
    local deb_path="$1"

    info "Installing .deb package..."
    if command -v apt >/dev/null 2>&1; then
        sudo apt install -y "$deb_path"
    elif command -v dpkg >/dev/null 2>&1; then
        sudo dpkg -i "$deb_path"
        sudo apt-get install -f -y 2>/dev/null || true
    else
        error "Neither apt nor dpkg found"
    fi

    rm "$deb_path"
    success "$APP_NAME installed"
}

install_linux_rpm() {
    local rpm_path="$1"

    info "Installing .rpm package..."
    if command -v dnf >/dev/null 2>&1; then
        sudo dnf install -y "$rpm_path"
    elif command -v yum >/dev/null 2>&1; then
        sudo yum install -y "$rpm_path"
    elif command -v rpm >/dev/null 2>&1; then
        sudo rpm -i "$rpm_path"
    else
        error "Neither dnf, yum, nor rpm found"
    fi

    rm "$rpm_path"
    success "$APP_NAME installed"
}

install_linux_appimage() {
    local appimage_path="$1"
    local install_dir="$HOME/.local/bin"
    local app_path="$install_dir/sorcery-desktop"

    mkdir -p "$install_dir"

    info "Installing AppImage to $install_dir..."
    mv "$appimage_path" "$app_path"
    chmod +x "$app_path"

    if [[ ":$PATH:" != *":$install_dir:"* ]]; then
        warn "Add $install_dir to your PATH to run sorcery-desktop from anywhere"
    fi

    success "$APP_NAME installed to $app_path"
}

launch_app() {
    local os="$1"

    info "Launching $APP_NAME..."

    case "$os" in
        macos)
            open -a "$APP_NAME"
            ;;
        linux)
            if command -v sorcery-desktop >/dev/null 2>&1; then
                sorcery-desktop &
            elif [ -x "$HOME/.local/bin/sorcery-desktop" ]; then
                "$HOME/.local/bin/sorcery-desktop" &
            else
                warn "Could not find sorcery-desktop binary to launch"
            fi
            ;;
    esac
}

main() {
    echo ""
    printf "${GREEN}╔════════════════════════════════════════╗${NC}\n"
    printf "${GREEN}║${NC}     ${BLUE}Sorcery Desktop Installer${NC}         ${GREEN}║${NC}\n"
    printf "${GREEN}╚════════════════════════════════════════╝${NC}\n"
    echo ""

    local os
    local arch
    os=$(detect_os)
    arch=$(detect_arch)

    info "Detected: $os ($arch)"

    local version
    version=$(get_latest_version)

    if [ -z "$version" ]; then
        error "Could not determine latest version. Check your internet connection."
    fi

    info "Latest version: $version"

    local format=""
    if [ "$os" = "linux" ]; then
        format=$(detect_linux_distro)
        info "Package format: $format"
    fi

    local artifact_url
    artifact_url=$(get_artifact_url "$version" "$os" "$arch" "$format")

    local temp_dir
    temp_dir=$(mktemp -d)
    local artifact_name
    artifact_name=$(basename "$artifact_url")
    local artifact_path="$temp_dir/$artifact_name"

    download_file "$artifact_url" "$artifact_path"

    case "$os" in
        macos)
            install_macos "$artifact_path"
            ;;
        linux)
            case "$format" in
                deb)
                    install_linux_deb "$artifact_path"
                    ;;
                rpm)
                    install_linux_rpm "$artifact_path"
                    ;;
                appimage)
                    install_linux_appimage "$artifact_path"
                    ;;
            esac
            ;;
    esac

    rmdir "$temp_dir" 2>/dev/null || true

    launch_app "$os"

    echo ""
    success "Installation complete!"
    echo ""
    printf "${BLUE}Next step:${NC} Install the Chrome extension for click-to-open support\n"
    printf "           ${YELLOW}https://getsorcery.com/chrome${NC}\n"
    echo ""
}

main "$@"
