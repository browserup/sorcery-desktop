# First-Run Setup Experience

## Problem
Currently, on first install:
- Default editor is hardcoded to `vscode` (might not be user's preference)
- Workspaces folder is auto-detected (might pick wrong folder)
- User has to manually find and change settings

## Proposed Solution

### First-Run Detection
- Check if settings file exists at startup
- If not, show setup wizard before loading main app

### Setup Wizard UI
A modal/page with two steps:

1. **Default Editor Selection**
   - Show grid of installed editors (only installed ones)
   - Pre-select most recently used editor if detectable
   - If no detection possible, show alphabetically with a suggestion

2. **Workspaces Folder Selection**
   - Show auto-detected folder with repo count
   - Show other candidates with their repo counts
   - Allow custom folder selection via file picker

### Implementation Steps
- [ ] Add `is_first_run` check in Rust backend (settings file doesn't exist)
- [ ] Create `setup.html` page with wizard UI
- [ ] Add Tauri command to get setup suggestions (detected folder, installed editors)
- [ ] On wizard completion, save settings and redirect to main app
- [ ] Ensure normal app startup skips wizard if settings exist

### UX Considerations
- Keep it simple: 2 questions max
- Show sensible defaults so user can just click "Continue"
- Allow skipping (use defaults) for quick setup
- Make it clear this can be changed later in Settings
