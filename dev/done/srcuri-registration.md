# srcuri:// Protocol Registration

Platform-specific instructions for registering the `srcuri://` protocol handler.

---

## macOS Registration

**Method:** Application bundle configuration

**File:** `Info.plist` (embedded in `.app` bundle)

**Configuration:**

```xml
<key>CFBundleURLTypes</key>
<array>
    <dict>
        <key>CFBundleURLSchemes</key>
        <array>
            <string>srcuri</string>
        </array>
        <key>CFBundleURLName</key>
        <string>com.srcuri.app</string>
        <key>CFBundleTypeRole</key>
        <string>Editor</string>
    </dict>
</array>
<key>LSUIElement</key>
<true/>
```

**Key Components:**

- `CFBundleURLSchemes`: Protocol name (`srcuri`)
- `CFBundleURLName`: Unique identifier
- `CFBundleTypeRole`: Application role
- `LSUIElement`: Hide from Dock/Cmd+Tab (optional, for silent operation)

**Registration Process:**

1. Install application to `/Applications/`
2. Launch application once
3. macOS LaunchServices automatically registers protocol
4. Verify with: `defaults read /Applications/sorcery.app/Contents/Info CFBundleURLTypes`

**Testing Registration:**

```bash
# Check registration
defaults read /Applications/sorcery.app/Contents/Info CFBundleURLTypes

# Test protocol
open "srcuri:///etc/hosts:1"
```

**Troubleshooting:**

```bash
# Reset Launch Services database (if registration fails)
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -kill -r -domain local -domain system -domain user

# Re-register application
open -a /Applications/sorcery.app
```

---

## Linux Registration

**Method:** XDG Desktop Entry + MIME type association

**File:** `~/.local/share/applications/srcuri.desktop`

**Desktop Entry Format:**

```ini
[Desktop Entry]
Version=1.0
Type=Application
Name=Sorcery
Comment=Editor hyperlink protocol handler
Exec=/usr/local/bin/srcuri %u
Icon=srcuri
Terminal=false
Categories=Development;
MimeType=x-scheme-handler/srcuri;
NoDisplay=true
```

**Key Components:**

- `Exec`: Path to executable with `%u` (URL placeholder)
- `MimeType`: Protocol handler MIME type
- `NoDisplay`: Hide from application menus (optional)

**Registration Process:**

```bash
# 1. Create desktop entry
cat > ~/.local/share/applications/srcuri.desktop << 'EOF'
[Desktop Entry]
Version=1.0
Type=Application
Name=Sorcery
Exec=/usr/local/bin/srcuri %u
MimeType=x-scheme-handler/srcuri;
NoDisplay=true
EOF

# 2. Register MIME type
xdg-mime default srcuri.desktop x-scheme-handler/srcuri

# 3. Update desktop database
update-desktop-database ~/.local/share/applications
```

**Verification:**

```bash
# Check default handler
xdg-mime query default x-scheme-handler/srcuri
# Should output: srcuri.desktop

# Test protocol
xdg-open "srcuri:///etc/hosts:1"
```

**Distribution-Specific Notes:**

**Ubuntu/Debian:**
```bash
# Install to system location (requires root)
sudo cp srcuri.desktop /usr/share/applications/
sudo update-desktop-database
```

**Fedora/RHEL:**
```bash
# SELinux may require context labels
sudo semanage fcontext -a -t bin_t '/usr/local/bin/srcuri'
sudo restorecon -v /usr/local/bin/srcuri
```

**Arch Linux:**
```bash
# Use pacman hook for automatic registration
# Place in /etc/pacman.d/hooks/srcuri.hook
```

---

## Windows Registration

**Method:** Registry key creation

**Registry Location:** `HKEY_CLASSES_ROOT\srcuri`

**Registry Structure:**

```
HKEY_CLASSES_ROOT
└── srcuri
    ├── (Default) = "URL:Sorcery Protocol"
    ├── URL Protocol = ""
    ├── DefaultIcon
    │   └── (Default) = "C:\Program Files\Sorcery\srcuri.exe,0"
    └── shell
        └── open
            └── command
                └── (Default) = "C:\Program Files\Sorcery\srcuri.exe" "%1"
```

**Registry Script (.reg file):**

```reg
Windows Registry Editor Version 5.00

[HKEY_CLASSES_ROOT\srcuri]
@="URL:Sorcery Protocol"
"URL Protocol"=""

[HKEY_CLASSES_ROOT\srcuri\DefaultIcon]
@="C:\\Program Files\\Sorcery\\srcuri.exe,0"

[HKEY_CLASSES_ROOT\srcuri\shell]

[HKEY_CLASSES_ROOT\srcuri\shell\open]

[HKEY_CLASSES_ROOT\srcuri\shell\open\command]
@="\"C:\\Program Files\\Sorcery\\srcuri.exe\" \"%1\""
```

**Manual Registration:**

```powershell
# PowerShell script (run as Administrator)
$protocolName = "srcuri"
$executablePath = "C:\Program Files\Sorcery\srcuri.exe"

# Create protocol key
New-Item -Path "HKCR:\$protocolName" -Force
Set-ItemProperty -Path "HKCR:\$protocolName" -Name "(Default)" -Value "URL:Sorcery Protocol"
Set-ItemProperty -Path "HKCR:\$protocolName" -Name "URL Protocol" -Value ""

# Create icon key
New-Item -Path "HKCR:\$protocolName\DefaultIcon" -Force
Set-ItemProperty -Path "HKCR:\$protocolName\DefaultIcon" -Name "(Default)" -Value "$executablePath,0"

# Create command key
New-Item -Path "HKCR:\$protocolName\shell\open\command" -Force
Set-ItemProperty -Path "HKCR:\$protocolName\shell\open\command" -Name "(Default)" -Value "`"$executablePath`" `"%1`""
```

**Installer Integration (MSI/WiX):**

```xml
<Component Id="ProtocolRegistration" Guid="YOUR-GUID-HERE">
  <RegistryKey Root="HKCR" Key="srcuri">
    <RegistryValue Type="string" Value="URL:Sorcery Protocol" />
    <RegistryValue Type="string" Name="URL Protocol" Value="" />

    <RegistryKey Key="DefaultIcon">
      <RegistryValue Type="string" Value="[INSTALLDIR]srcuri.exe,0" />
    </RegistryKey>

    <RegistryKey Key="shell\open\command">
      <RegistryValue Type="string" Value="&quot;[INSTALLDIR]srcuri.exe&quot; &quot;%1&quot;" />
    </RegistryKey>
  </RegistryKey>
</Component>
```

**Verification:**

```batch
REM Check registry key
reg query HKEY_CLASSES_ROOT\srcuri\shell\open\command

REM Test protocol
start srcuri:///C:/Windows/System32/drivers/etc/hosts:1
```

**User vs System Registration:**

- **HKEY_CLASSES_ROOT**: System-wide, requires admin rights
- **HKEY_CURRENT_USER\Software\Classes**: Per-user, no admin required

```powershell
# Per-user registration (no admin)
$userClassesRoot = "HKCU:\Software\Classes"
New-Item -Path "$userClassesRoot\srcuri" -Force
# ... (same structure as above)
```
