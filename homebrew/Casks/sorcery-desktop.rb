cask "sorcery-desktop" do
  arch arm: "aarch64", intel: "x64"

  version "0.1.1"
  sha256 arm:   "PLACEHOLDER_ARM64_SHA256",
         intel: "PLACEHOLDER_X64_SHA256"

  url "https://github.com/ebeland/sorcery-desktop/releases/download/v#{version}/Sorcery.Desktop_#{version}_#{arch}.dmg"
  name "Sorcery Desktop"
  desc "Editor-independent code linking via srcuri:// protocol"
  homepage "https://getsorcery.com"

  livecheck do
    url :url
    strategy :github_latest
  end

  depends_on macos: ">= :high_sierra"

  app "Sorcery Desktop.app"

  postflight do
    system_command "/usr/bin/open",
                   args: ["-a", "Sorcery Desktop"],
                   sudo: false
  end

  zap trash: [
    "~/Library/Application Support/com.srcuri.desktop",
    "~/Library/Caches/com.srcuri.desktop",
    "~/Library/Preferences/com.srcuri.desktop.plist",
    "~/.config/sorcery-desktop",
  ]
end
