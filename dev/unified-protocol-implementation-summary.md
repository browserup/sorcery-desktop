# Unified Protocol Implementation Summary

## Overview

This document summarizes the changes made to align `sorcery-server` and `sorcery-desktop` with the unified protocol specification (`unified-protocol.md`).

## Key Changes

### 1. Removed `go.srcuri.com` Subdomain

**Before:** Provider-passthrough URLs required the `go.srcuri.com` subdomain:
```
go.srcuri.com/github.com/owner/repo/blob/main/file.rs#L42
```

**After:** Provider-passthrough is now path-based at `srcuri.com`:
```
srcuri.com/github.com/owner/repo/blob/main/file.rs#L42
```

### 2. Server Changes (sorcery-server)

#### Files Modified:
- `src/subdomain.rs` - Removed `RemoteTranslator` variant from `SubdomainMode`
- `src/main.rs` - Removed go.* subdomain handling from routers
- `src/routes/mod.rs` - Replaced `go` module with `provider` module
- `src/routes/translator.rs` - Updated `catchall_handler` to serve HTML interstitial for provider paths

#### Files Added:
- `src/routes/provider.rs` - Handler for provider-passthrough pages
- `src/templates/provider.html` - HTML+JS interstitial for provider URLs

#### Files Removed:
- `src/routes/go.rs` - No longer needed
- `src/templates/go.html` - Replaced by provider.html

#### Key Logic Changes:
1. `is_translatable_path()` detects provider URLs by checking if first path segment contains a dot
2. Provider paths serve HTML+JS interstitial (preserves URL fragments like `#L42`)
3. Workspace paths continue to work as before (`srcuri.com/workspace/path:line`)

### 3. Desktop Changes (sorcery-desktop)

#### Files Modified:
- `src-tauri/src/protocol_handler/parser.rs` - Added `ProviderPassthrough` variant and parsing
- `src-tauri/src/protocol_handler/mod.rs` - Added handler for provider-passthrough
- `src-tauri/src/commands/mod.rs` - Handle `OpenInBrowser` result
- `src-tauri/src/main.rs` - Handle `OpenInBrowser` result
- `src-tauri/Cargo.toml` - Added `open` crate for browser opening

#### New Enum Variants:
```rust
// New request type in parser.rs
SrcuriRequest::ProviderPassthrough {
    provider: String,      // e.g., "github.com/owner/repo"
    repo_name: String,     // e.g., "repo"
    path: String,          // e.g., "src/lib.rs"
    line: Option<usize>,
    column: Option<usize>,
    git_ref: Option<GitRef>,
}

// New result type in mod.rs
HandleResult::OpenInBrowser {
    url: String,
}
```

#### Detection Logic:
- If first path segment contains a dot AND there are 3+ segments → ProviderPassthrough
- Single segments like `README.md` (with dots) remain PartialPath

#### Resolution Logic:
1. Try to find a workspace matching the `repo_name`
2. **If found:** Open the file locally in the editor
3. **If not found:** Open `https://srcuri.com/<provider>/<path>` in browser

## URL Format Examples

| Type | Example URL |
|------|-------------|
| **Workspace (web)** | `https://srcuri.com/myproject/src/lib.rs:42` |
| **Provider (web)** | `https://srcuri.com/github.com/owner/repo/blob/main/file.rs#L42` |
| **Workspace (protocol)** | `srcuri://myproject/src/lib.rs:42` |
| **Provider (protocol)** | `srcuri://github.com/owner/repo/blob/main/file.rs#L42` |

## Provider-Passthrough Flow

```
1. User clicks: srcuri://github.com/owner/repo/blob/main/file.rs#L42

2. Desktop parses URL:
   - Detects "github.com" contains dot → provider-passthrough
   - Extracts: provider="github.com/owner/repo", repo_name="repo", path="file.rs", line=42

3. Desktop checks: Is there a workspace named "repo"?
   - YES → Open ~/code/repo/file.rs:42 in editor
   - NO → Open https://srcuri.com/github.com/owner/repo/blob/main/file.rs#L42 in browser

4. Browser loads srcuri.com page (if no local workspace):
   - HTML+JS reads window.location.hash (#L42)
   - Shows interstitial with clone option
   - Redirects to srcuri://repo/file.rs:42?remote=https://github.com/owner/repo
```

## Test Coverage

### Server Tests (109 passing):
- Subdomain detection (DirectProtocol, WwwRedirect, EnterpriseTenant)
- Provider URL parsing (GitHub, GitLab, Bitbucket, Gitea, Azure DevOps)
- Mirror URL generation
- Integration tests for translator endpoints

### Desktop Tests (48 parser tests passing):
- Provider-passthrough parsing (GitHub, GitLab, Bitbucket, Gitea, self-hosted)
- Fragment parsing (#L42, #lines-5)
- Workspace/PartialPath detection (dots in filenames don't trigger provider mode)
- Line/column extraction

## Workspace Name Validation

Workspace names containing dots (e.g., `my.project`) are **warned** at settings load time because they are ambiguous with provider hostnames (e.g., `github.com`).

**Validation behavior:**
- Logs a warning for each workspace name containing a dot
- Does NOT reject the configuration (existing setups continue to work)
- Suggests using the `?workspace=` escape hatch

**Example warning:**
```
WARN: Workspace 'my.project' contains a dot in its name.
      This may be confused with provider hostnames (e.g., github.com).
      Consider renaming, or use ?workspace=my.project in URLs to reference it explicitly.
```

## Workspace Escape Hatch (`?workspace=`)

For workspace names that contain dots, use the `?workspace=` parameter to explicitly specify which workspace to use:

**Format:**
```
srcuri://github.com/owner/repo/blob/main/file.rs?workspace=my.dotted.workspace#L42
```

**How it works:**
1. Parser extracts `workspace_override` from `?workspace=` parameter
2. Handler uses `workspace_override` instead of `repo_name` for workspace lookup
3. If workspace found → opens locally; if not → opens in browser

**Example usage:**
```
# Repo name is "awesome-project" but local workspace is "my.awesome.project"
srcuri://github.com/owner/awesome-project/blob/main/file.rs?workspace=my.awesome.project#L42
```

## Breaking Changes

- **`go.srcuri.com` removed** - All links using this subdomain will no longer work
- **Workspace names with dots** - Logged as warnings (not rejected), use `?workspace=` escape hatch

## Completed Documentation Updates

1. **srcuri-website-url-design.md** - Updated to path-based detection, removed go.srcuri.com references
2. **translator-mode.md** - Renamed to "Provider Passthrough", updated all examples and user rules
3. **README.md** - Updated with new path-based URL examples
