# Security Model

Sorcery Desktop handles external URLs (`srcuri://`) that can open files in local editors. This document describes the security measures in place.

## Core Principle: No Code Execution

Sorcery never executes code from workspaces. Unlike language servers or IDE plugins that may auto-load addons or extensions (e.g., the Ruby LSP addon vulnerability), Sorcery:

- Does not interpret or execute any files from workspaces
- Does not load configuration or plugins from workspace directories
- Does not run scripts based on file patterns
- Does not evaluate any data as code

When opening workspaces with risky editor configurations (`.vscode/tasks.json` with auto-run tasks, `.exrc`/`.vimrc`), Sorcery displays a trust dialog before proceeding. The dialog warns about potential risks but Sorcery itself never executes these files.

## Threat Model

**Primary threats:**
- Malicious URLs attempting to execute arbitrary code
- Path traversal attacks to access files outside workspaces
- Command injection through file paths
- NTLM credential leakage via UNC paths (Windows)

**Trust boundaries:**
- URLs arrive from external sources (browsers, chat apps, documentation)
- Settings file is user-controlled (local filesystem)
- Editor binaries are system-installed

## Path Validation (`path_validator/mod.rs`)

All file paths are validated before being passed to editors:

### Blocked Patterns

| Pattern | Reason |
|---------|--------|
| `../` and `..\` | Path traversal |
| `//` | Protocol injection |
| Shell metacharacters: `; & \| \` $ # ' " { } < >` | Command injection |
| UNC paths `\\server\share` | NTLM credential leakage |
| Binary extensions: `.exe`, `.app`, `.dmg` | Executable files |
| Paths > 4096 characters | Buffer overflow prevention |
| Mid-path `~` characters | Expansion attacks |

### Allowed Patterns

| Pattern | Reason |
|---------|--------|
| `()` parentheses | Common in macOS folder names |
| `[]` brackets | Valid in most filesystems |
| Leading `~` | Standard home directory expansion |
| `@`, `%`, `+`, `=` | Safe special characters |

### No Shell Invocation

Editors are launched via Rust's `Command` API with arguments passed directly, never through a shell. This prevents command injection even if a malicious character bypasses validation.

### Symlink Resolution

All paths are canonicalized (symlinks fully resolved) before validation:

- Symlinks are resolved via `std::fs::canonicalize()` before bounds checking
- The `starts_with(workspace_root)` check uses the canonical (resolved) path
- Symlinks pointing outside the workspace are rejected after resolution

This prevents attacks where a symlink like `/project/tmp -> /malicious/` is used to escape workspace boundaries. The resolved path `/malicious/file.rs` would fail the workspace containment check.

## URL Parsing (`protocol_handler/parser.rs`)

### Git Reference Validation

| Reference Type | Validation |
|----------------|------------|
| Branch names | Max 128 chars, no `..`, alphanumeric + `/-_.` |
| Tag names | Max 128 chars, no `..`, alphanumeric + `/-_.` |
| Commit SHAs | 7-64 hexadecimal characters only |
| Remote URLs | No `..`, no `//`, validated structure |

### Authority Parsing

Reserved authorities (`wks`, `rel`, `abs`, `ext`) are case-insensitive and cannot be used as workspace names, preventing confusion attacks.

## Git Operations (`protocol_handler/git.rs`)

- Revisions are validated with `git rev-parse --verify` before use
- File content from revisions is limited to 10MB
- Output is validated as UTF-8
- Working tree operations require explicit user confirmation via dialogs

## Workspace Discovery (`settings/discovery.rs`)

- Directory scanning is limited to 1000 entries
- Scan operations timeout after 5 seconds
- Only immediate children of the workspaces folder are scanned (no recursion)

## Frontend Security

- Dialog data comes from validated backend state
- User-provided data (workspace names, paths) originates from local settings file
- No external data is rendered without backend validation

## Security Testing

The codebase includes security-focused tests:

- **Parser tests** (`protocol_handler/parser.rs`): Validate rejection of shell metacharacters, path traversal, and malformed inputs
- **Path validator tests** (`path_validator/mod.rs`): Test blocked patterns, symlink handling, and platform-specific cases

Integration tests cover end-to-end attack vectors:
- Malicious srcuri:// URLs through validation to editor launch
- Symlink escape attempts
- Path traversal in git references

## Reporting Vulnerabilities

If you discover a security vulnerability, please report it by emailing the maintainers directly rather than opening a public issue. We will acknowledge receipt within 48 hours and provide a detailed response within 7 days.
