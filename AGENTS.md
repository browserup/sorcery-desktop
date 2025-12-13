# AGENTS.md

Instructions for AI agents working on this codebase.

## Project Overview

Sorcery Desktop is a cross-platform hyperlinker for editors/IDEs. The srcuri:// protocol lets developers share editor-independent code references (file + line) that open in each recipient's preferred editor.

## Key Directories

- `src-tauri/src/` - Rust backend (Tauri app)
  - `editors/` - Editor manager implementations
  - `protocol_handler/` - srcuri:// URL parsing and handling
  - `settings/` - Configuration management
  - `workspace_mru/` - Workspace tracking
- `public/` - HTML UI components
- `dev/` - Development documentation

## Code Standards

- Act as a principal architect level developer
- Reuse existing implementations before creating new ones
- No fallback mechanisms that mask errors
- No comments that restate what the code says
- Remove dead code rather than commenting it out

## Documentation Maintenance

When adding a new feature, as a final step:
1. Update `dev/features.md` - add the feature under the appropriate section
2. Update `dev/use-cases.md` - if the feature represents a new use case (use "As a..." format)
