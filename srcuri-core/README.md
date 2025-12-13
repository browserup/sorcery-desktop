# srcuri-core

Shared URL parsing library for the `srcuri://` protocol ecosystem.

## Overview

This crate provides URL parsing logic shared between:

- **sorcery-server** - The web gateway that bridges HTTPS to `srcuri://`
- **sorcery-desktop** - The native app that handles `srcuri://` protocol links

## Usage

```rust
use srcuri_core::parse_remote_url;

let result = parse_remote_url("https://github.com/owner/repo/blob/main/src/lib.rs#L42")?;
assert_eq!(result.repo_name, "repo");
assert_eq!(result.file_path, Some("src/lib.rs".to_string()));
assert_eq!(result.line, Some(42));
```

## Supported Providers

- GitHub (including github.dev, Codespaces)
- GitLab (including self-hosted, Web IDE)
- Bitbucket
- Gitea
- Codeberg
- Azure DevOps

## Architecture: Why JavaScript Parsing Also Exists

### The URL Fragment Problem

URL fragments (`#L42`) are **never sent to servers** by browsers. This is a
fundamental browser security feature:

```
User enters:  srcuri.com/github.com/owner/repo/blob/main/file.rs#L42
                                                                 ^^^^
Server sees:  srcuri.com/github.com/owner/repo/blob/main/file.rs
                                                    (fragment stripped)
```

### Two Parsing Implementations

This architectural constraint requires **two URL parsing implementations**:

| Flow | Parser | Why |
|------|--------|-----|
| Path-based passthrough (`srcuri.com/github.com/...#L42`) | JavaScript in browser | Server cannot see fragment |
| Query-based passthrough (`srcuri.com/?remote=...%23L42`) | This crate (Rust) | Fragment is URL-encoded |
| Desktop protocol handler (`srcuri://...`) | This crate (Rust) | Native app, no browser involved |

The JavaScript parser in `sorcery-server/src/templates/provider.html` reads
`window.location.hash` to extract line numbers, then constructs the `srcuri://`
URL client-side.

### Implications

1. **Two implementations must stay in sync** when adding new providers
2. **OpenGraph unfurling cannot include line numbers** for path-based URLs
3. **Testing must cover both paths**

## Future Direction: WebAssembly

A planned improvement is to compile this crate to **WebAssembly (WASM)** for use
in the browser. This would:

- **Eliminate the duplicated JavaScript** - single source of truth
- **Guarantee consistency** - same parsing logic everywhere
- **Simplify maintenance** - add providers in one place

### WASM Preparation

To enable WASM compilation in the future:

1. Keep dependencies minimal and WASM-compatible
2. Avoid platform-specific code in parsing logic
3. Use `#[cfg(target_arch = "wasm32")]` for any browser-specific adaptations

### Example Future Usage

```javascript
// Future: Use WASM-compiled srcuri-core in browser
import init, { parse_remote_url } from 'srcuri-core';

await init();
const result = parse_remote_url(window.location.href + window.location.hash);
```

## License

MIT
