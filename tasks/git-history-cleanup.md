# Git History Cleanup Plan - Detailed Breakdown

## Strategy
Work from the final state (HEAD of main) and create commits by carefully staging files in logical groups. Each commit should compile if possible.

## Commit Sequence

### Phase 1: October 14-22 - Core Development

- [ ] **1. Initial project scaffold** (Oct 14, 7:30pm)
  - src-tauri/Cargo.toml (minimal, no lib section yet)
  - src-tauri/build.rs
  - src-tauri/tauri.conf.json (basic)
  - src-tauri/src/main.rs (minimal - just fn main() {})
  - MIT-LICENSE
  - .gitignore
  - README.md (basic)
  - CLAUDE.md

- [ ] **2. Settings module** (Oct 15, 8:00pm)
  - src-tauri/src/settings/mod.rs
  - src-tauri/src/settings/models.rs
  - src-tauri/src/settings/manager.rs
  - Update main.rs with `mod settings;`

- [ ] **3. Path validator** (Oct 16, 7:45pm)
  - src-tauri/src/path_validator/mod.rs
  - Update main.rs with `mod path_validator;`

- [ ] **4. Editor framework** (Oct 17, 8:30pm)
  - src-tauri/src/editors/mod.rs
  - src-tauri/src/editors/traits.rs
  - Update main.rs with `mod editors;`

- [ ] **5. VSCode editor support** (Oct 18, 9:00pm)
  - src-tauri/src/editors/vscode.rs
  - Update editors/mod.rs

- [ ] **6. JetBrains editor support** (Oct 19, 2:30pm - weekend)
  - src-tauri/src/editors/jetbrains.rs
  - Update editors/mod.rs

- [ ] **7. Sublime and other editors** (Oct 19, 5:00pm - weekend)
  - src-tauri/src/editors/others.rs
  - Update editors/mod.rs

- [ ] **8. Terminal editors (initial)** (Oct 20, 11:00am - weekend)
  - src-tauri/src/editors/terminal.rs (single file version)
  - Update editors/mod.rs

- [ ] **9. Editor registry** (Oct 20, 3:30pm - weekend)
  - src-tauri/src/editors/registry.rs
  - Update editors/mod.rs

- [ ] **10. Activity tracker** (Oct 21, 7:00pm)
  - src-tauri/src/tracker/mod.rs
  - src-tauri/src/tracker/detector.rs
  - Update main.rs

- [ ] **11. Request dispatcher** (Oct 21, 9:30pm)
  - src-tauri/src/dispatcher/mod.rs
  - Update main.rs

- [ ] **12. Tauri commands** (Oct 22, 7:15pm)
  - src-tauri/src/commands/mod.rs
  - Update main.rs with proper Tauri app setup

### Phase 2: October 23-31 - Protocol & UI Foundation

- [ ] **13. Protocol handler core** (Oct 23, 8:00pm)
  - src-tauri/src/protocol_handler/mod.rs
  - src-tauri/src/protocol_handler/parser.rs
  - Update main.rs

- [ ] **14. Protocol matcher** (Oct 24, 7:30pm)
  - src-tauri/src/protocol_handler/matcher.rs

- [ ] **15. Git protocol support** (Oct 25, 8:45pm)
  - src-tauri/src/protocol_handler/git.rs

- [ ] **16. Protocol registration** (Oct 26, 1:00pm - weekend)
  - src-tauri/src/protocol_registration/mod.rs
  - Update main.rs

- [ ] **17. Main UI pages** (Oct 26, 4:30pm - weekend)
  - public/index.html
  - public/settings.html
  - public/editor-testbed.html

- [ ] **18. Application icons** (Oct 27, 11:30am - weekend)
  - src-tauri/icons/*.png
  - src-tauri/icons/create_icon.py

### Phase 3: November 3-17 - Testing & Refinement

- [ ] **19. Terminal editors refactor** (Nov 3, 7:00pm)
  - src-tauri/src/editors/terminal/ (directory)
  - src-tauri/src/editors/terminal/mod.rs
  - src-tauri/src/editors/terminal/vim.rs
  - src-tauri/src/editors/terminal/neovim.rs
  - src-tauri/src/editors/terminal/emacs.rs
  - src-tauri/src/editors/terminal/terminal_detector.rs

- [ ] **20. Additional terminal editors** (Nov 4, 8:15pm)
  - src-tauri/src/editors/terminal/nano.rs
  - src-tauri/src/editors/terminal/micro.rs
  - src-tauri/src/editors/terminal/kakoune.rs

- [ ] **21. Kate editor** (Nov 5, 7:30pm)
  - src-tauri/src/editors/kate.rs

- [ ] **22. Editor tests** (Nov 6, 8:00pm)
  - src-tauri/tests/editor_launch_tests.rs

- [ ] **23. Protocol tests** (Nov 7, 7:45pm)
  - src-tauri/tests/protocol_handler_tests.rs

- [ ] **24. Docker test infrastructure** (Nov 8, 2:00pm - weekend)
  - tests/docker/Dockerfile
  - tests/docker/run-tests.sh
  - tests/docker/run-editor-tests.sh
  - tests/docker/run-protocol-tests.sh
  - tests/README.md
  - docker-compose.yml
  - .dockerignore

- [ ] **25. Git command logging** (Nov 10, 7:30pm)
  - src-tauri/src/git_command_log/mod.rs
  - Update main.rs

- [ ] **26. UI dialogs** (Nov 11, 8:00pm)
  - public/workspace-chooser.html
  - public/revision-handler.html
  - public/debug-git-log.html

- [ ] **27. Additional UI pages** (Nov 12, 7:15pm)
  - public/clone-dialog.html
  - public/non-workspace-confirm.html
  - public/flash-message.html

- [ ] **28. Tauri schema updates** (Nov 13, 8:30pm)
  - src-tauri/gen/schemas/*.json
  - src-tauri/Info.plist

- [ ] **29. lib.rs exports** (Nov 14, 7:00pm)
  - src-tauri/src/lib.rs
  - Update Cargo.toml with [lib] section

### Phase 4: November 18-26 - Documentation & Workspace

- [ ] **30. Workspace MRU module** (Nov 18, 8:00pm)
  - src-tauri/src/workspace_mru/mod.rs
  - src-tauri/src/workspace_mru/models.rs
  - src-tauri/src/workspace_mru/probe.rs
  - src-tauri/src/workspace_mru/process.rs
  - src-tauri/src/workspace_mru/fs_signal.rs
  - src-tauri/src/workspace_mru/git_signals.rs
  - Update main.rs

- [ ] **31. Development documentation** (Nov 20, 7:30pm)
  - DEVELOPMENT.md
  - URL-FORMATS.md
  - INSTALL.md

- [ ] **32. Protocol specifications** (Nov 21, 8:15pm)
  - dev/srcuri-protocol-spec.md
  - dev/protocol-registration.md
  - dev/srcuri-registration.md

- [ ] **33. Feature documentation** (Nov 22, 9:00pm)
  - dev/features.md
  - dev/use-cases.md
  - dev/advantages.md

- [ ] **34. Install scripts** (Nov 23, 1:30pm - weekend)
  - install-dev.sh
  - install-local.sh
  - scripts/quick-install-macos.sh
  - Makefile

### Phase 5: December 3-20 - Extraction & Polish

- [ ] **35. Root Cargo workspace** (Dec 3, 7:00pm)
  - Cargo.toml (root workspace)
  - Cargo.lock (root)

- [ ] **36. srcuri-core crate extraction** (Dec 5, 8:30pm)
  - srcuri-core/Cargo.toml
  - srcuri-core/src/lib.rs
  - srcuri-core/src/parser.rs
  - srcuri-core/src/types.rs
  - srcuri-core/README.md
  - Update root Cargo.toml workspace members
  - Update src-tauri/Cargo.toml to depend on srcuri-core

- [ ] **37. Research documentation** (Dec 8, 2:00pm - weekend)
  - dev/research/*.md
  - dev/url-patterns/*.yaml

- [ ] **38. Task tracking** (Dec 10, 7:45pm)
  - tasks/*.md

- [ ] **39. Alternative icons** (Dec 12, 8:00pm)
  - src-tauri/icons/options/*.png
  - src-tauri/icons/create_sorcery_icons.py

- [ ] **40. Final configuration** (Dec 15, 7:30pm)
  - .claude/settings.json
  - .claude/settings.local.json
  - .claude/hooks/*.sh
  - AGENTS.md

## Technical Notes

### Ensuring Compilation
- Each commit should at minimum pass `cargo check` in src-tauri/
- main.rs needs mod declarations for each module before it's used
- Cargo.toml dependencies must be added before code using them

### Date Format
```bash
GIT_AUTHOR_DATE="2025-10-14T19:30:00-0400" \
GIT_COMMITTER_DATE="2025-10-14T19:30:00-0400" \
git commit -m "Message"
```

### Verification Commands
```bash
git log --format='%ad %s' --date=format:'%Y-%m-%d %H:%M %a'
cd src-tauri && cargo check
```
