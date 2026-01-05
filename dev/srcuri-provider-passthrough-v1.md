# srcuri External URL Specification v1.0

This specification extends the [srcuri Protocol Specification](srcuri-protocol-spec-v1.md) to support direct conversion from git hosting provider URLs using the `ext` mode.

---

## Introduction

### Purpose

External URL mode (`ext`) enables srcuri links to be created directly from git hosting provider URLs (GitHub, GitLab, Bitbucket, etc.) with mechanical transformation. The `ext` authority marks these links, creating a bridge between web-based code browsing and local editor workflows.

### Use Cases

- **Viral sharing**: Developers share srcuri links that open in Sorcery for those who have it, with graceful fallback for those who don't
- **Fallback viewing**: Users without Sorcery installed can still view the code through the provider's web interface via an interstitial page
- **Fork disambiguation**: Links carry the full repository identity (`owner/repo`), eliminating ambiguity when forks exist
- **Mechanical conversion**: Transform any provider URL to srcuri by restructuring scheme and host into path segments

### Prerequisites

Readers should be familiar with the core srcuri protocol, particularly:
- Workspace mode (`srcuri://workspace/path`)
- Query parameters (`?remote=`, `?workspaceHint=`)
- Line/column handling
- The linkification requirement (`srcuri://` for clickable links)

---

## Key Goals

These goals guide design decisions for external URL mode:

1. **Mechanical conversion** — Converting from a provider URL to srcuri requires only moving `scheme://host` into path segments.

2. **Round-trip fidelity** — Converting to srcuri and back to the original provider URL should be lossless.

3. **Graceful degradation** — Links should work for everyone: Sorcery users open locally, others view on the web.

4. **Provider agnostic** — The format works with any git hosting provider, not just the major ones.

5. **Local-first resolution** — When a matching local repository exists, open there without network requests.

6. **Unambiguous detection** — The `ext` authority clearly distinguishes external URLs from other path types.

---

## URL Format

### The `ext` Authority

External URL mode uses the reserved `ext` authority to indicate an embedded upstream URL:

```
srcuri://ext/https/github.com/owner/repo/blob/main/src/lib.rs#L42
         ^^^
         The ext authority marks this as external URL mode
```

This fits into the complete authority system:

| Authority | Mode | Example |
|-----------|------|---------|
| `<workspace>` | Implicit workspace | `srcuri://myrepo/src/lib.rs:42` |
| `workspace` | Explicit workspace | `srcuri://workspace/myrepo/src/lib.rs:42` |
| `match` | Search | `srcuri://match/README.md:1` |
| `abs` | Absolute path | `srcuri://abs/etc/hosts:1` |
| `ext` | External URL | `srcuri://ext/https/github.com/owner/repo/blob/main/file.rs#L42` |

### URL Structure

External URLs are encoded by placing the scheme and host as path segments:

```
srcuri://ext/<scheme>/<host>/<path...>[?<upstream-query>][#<upstream-fragment>]
```

**Critical rule:** The `?query` and `#fragment` belong to the **upstream URL**, not to srcuri options.

### Conversion Pattern

External URLs restructure the original provider URL, separating scheme and host:

| Form | Format |
|------|--------|
| **Original** | `https://github.com/owner/repo/blob/main/src/lib.rs#L42` |
| **Protocol** | `srcuri://ext/https/github.com/owner/repo/blob/main/src/lib.rs#L42` |
| **Web** | `https://srcuri.com/ext/https/github.com/owner/repo/blob/main/src/lib.rs#L42` |

**Conversion rules:**

To convert a provider URL to srcuri protocol:
```
https://github.com/...  →  srcuri://ext/https/github.com/...
^^^^^   ^^^^^^^^^^         ^^^^^^^^^^^^ ^^^^^ ^^^^^^^^^^
scheme  host               prefix       scheme host
```

To convert a provider URL to srcuri.com:
```
https://github.com/...  →  srcuri.com/ext/https/github.com/...
^^^^^   ^^^^^^^^^^         ^^^^^^^^^^^^^^ ^^^^^ ^^^^^^^^^^
scheme  host               prefix         scheme host
```

### URL Reconstruction

To reconstruct the original URL from ext mode:

```
srcuri://ext/https/github.com/owner/repo/blob/main/file.rs?plain=1#L42
                   └───────────────────────────────────────────────────┘
                   ↓
         https://github.com/owner/repo/blob/main/file.rs?plain=1#L42
```

Algorithm:
1. Extract scheme from first path segment after `ext/` (e.g., `https`)
2. Extract host from second path segment (e.g., `github.com`)
3. Reconstruct: `{scheme}://{host}/{remaining-path}`
4. Append query and fragment if present

### Detection Rules

Detection is unambiguous based on the authority:

| Authority | Mode |
|-----------|------|
| `workspace` | Explicit workspace |
| `match` | Search all workspaces |
| `abs` | Absolute path |
| `ext` | External URL |
| Anything else | Implicit workspace |

**Examples:**
```
srcuri://ext/https/github.com/owner/repo/blob/main/file.rs#L42
         ^^^
         Authority = "ext" → External URL mode

srcuri://myworkspace/src/lib.rs:42
         ^^^^^^^^^^^
         Not a reserved token → Implicit workspace

srcuri://match/README.md:1
         ^^^^^
         Authority = "match" → Search mode
```

### Line Number Handling

External mode preserves the **fragment** (`#L42`) rather than using the colon-based format (`:42`) because:

1. It matches provider URL conventions exactly (trivial conversion)
2. Fragments survive HTTP redirects
3. The srcuri.com interstitial page can read fragments via JavaScript

**Supported fragment formats:**

| Provider | Fragment Format | Example |
|----------|-----------------|---------|
| GitHub | `#L{line}` or `#L{start}-L{end}` | `#L42`, `#L10-L20` |
| GitLab | `#L{line}` or `#L{start}-{end}` | `#L42`, `#L10-20` |
| Bitbucket | `#lines-{line}` or `#lines-{start}:{end}` | `#lines-42`, `#lines-10:20` |
| Gitea | `#L{line}` or `#L{start}-L{end}` | `#L42`, `#L10-L20` |
| Azure DevOps | `&line={line}` (query param) | `?path=/file&line=42` |

When opening locally, these fragments are normalized to the standard `:line:column` format.

---

## Supported Providers

External URL mode works with any git hosting provider. The following are explicitly tested:

### GitHub

```
Original:    https://github.com/rust-lang/rust/blob/master/src/lib.rs#L42
Protocol:    srcuri://ext/https/github.com/rust-lang/rust/blob/master/src/lib.rs#L42
Web:         https://srcuri.com/ext/https/github.com/rust-lang/rust/blob/master/src/lib.rs#L42
```

**URL structure:** `/owner/repo/blob/{ref}/path/to/file`

**Ref location:** After `/blob/` — can be branch name, tag, or commit SHA

### GitLab

```
Original:    https://gitlab.com/gitlab-org/gitlab/-/blob/main/lib/api.rb#L100
Protocol:    srcuri://ext/https/gitlab.com/gitlab-org/gitlab/-/blob/main/lib/api.rb#L100
Web:         https://srcuri.com/ext/https/gitlab.com/gitlab-org/gitlab/-/blob/main/lib/api.rb#L100
```

**URL structure:** `/owner/repo/-/blob/{ref}/path/to/file`

**Note:** GitLab uses `/-/blob/` (with leading hyphen segment)

### Bitbucket

```
Original:    https://bitbucket.org/atlassian/python-bitbucket/src/main/README.md#lines-5
Protocol:    srcuri://ext/https/bitbucket.org/atlassian/python-bitbucket/src/main/README.md#lines-5
Web:         https://srcuri.com/ext/https/bitbucket.org/atlassian/python-bitbucket/src/main/README.md#lines-5
```

**URL structure:** `/owner/repo/src/{ref}/path/to/file`

**Note:** Bitbucket uses `/src/` instead of `/blob/`, and `#lines-N` for line fragments

### Gitea / Forgejo

```
Original:    https://codeberg.org/forgejo/forgejo/src/branch/main/README.md#L10
Protocol:    srcuri://ext/https/codeberg.org/forgejo/forgejo/src/branch/main/README.md#L10
Web:         https://srcuri.com/ext/https/codeberg.org/forgejo/forgejo/src/branch/main/README.md#L10
```

**URL structure:** `/owner/repo/src/branch/{branch}/path` or `/owner/repo/src/commit/{sha}/path`

### Azure DevOps

```
Original:    https://dev.azure.com/org/project/_git/repo?path=/src/main.rs&line=42
Protocol:    srcuri://ext/https/dev.azure.com/org/project/_git/repo?path=/src/main.rs&line=42
Web:         https://srcuri.com/ext/https/dev.azure.com/org/project/_git/repo?path=/src/main.rs&line=42
```

**URL structure:** `/org/project/_git/repo?path=/file&line=N`

**Note:** Azure DevOps uses query parameters instead of path segments for file and line

### Self-Hosted Instances

Self-hosted GitLab, Gitea, GitHub Enterprise, etc. work automatically:

```
srcuri://ext/https/gitlab.mycompany.com/team/project/-/blob/main/src/app.rs#L50
srcuri://ext/https/github.enterprise.corp/org/repo/blob/develop/lib/utils.py#L100
srcuri://ext/https/gitea.internal/user/repo/src/branch/main/README.md#L1
```

The `ext` mode works with any domain—no special detection needed.

---

## Resolution Behavior

When an external URL is opened, the following resolution process occurs:

### Step 1: Parse URL

Identify external mode by the `ext` authority, then extract components:
- `provider`: Full provider path (e.g., `github.com/owner/repo`)
- `repo_name`: Repository name (e.g., `repo`)
- `path`: File path within repository (e.g., `src/lib.rs`)
- `line`: Line number from fragment (e.g., `42`)
- `git_ref`: Branch, tag, or commit from URL path (e.g., `main`)

### Step 2: Local Workspace Lookup

Search configured workspaces for a match using **two strategies**:

1. **By git remote** (preferred): Check if any workspace has a git remote URL matching the provider (e.g., `github.com/owner/repo`). This handles cases where the local workspace name differs from the repository name.

2. **By name** (fallback): Is there a workspace named `{repo_name}`?

Git remote matching is preferred because it works regardless of what the user named their local clone.

### Step 3: Resolution

**If workspace found:**
```
Open {workspace_path}/{file_path} at line {line} in the configured editor
```

**If workspace NOT found:**
```
Open https://srcuri.com/ext/https/{provider_url} in the default browser
```

### Flow Diagram

```
User clicks: srcuri://ext/https/github.com/owner/repo/blob/main/file.rs#L42
                                    │
                                    ▼
                    ┌───────────────────────────────┐
                    │  Detect ext authority         │
                    │  Reconstruct upstream URL     │
                    │  provider = "github.com/      │
                    │              owner/repo"      │
                    │  path = "file.rs"             │
                    │  line = 42                    │
                    └───────────────────────────────┘
                                    │
                                    ▼
                    ┌───────────────────────────────┐
                    │  Any workspace with matching  │
                    │  git remote?                  │
                    └───────────────────────────────┘
                           │              │
                          YES             NO
                           │              │
                           ▼              ▼
                    ┌─────────────┐  ┌───────────────────────┐
                    │ Open local  │  │ Workspace named       │
                    │ workspace   │  │ "repo" exists?        │
                    │ /file.rs:42 │  └───────────────────────┘
                    └─────────────┘         │           │
                                          YES          NO
                                           │           │
                                           ▼           ▼
                                    ┌───────────┐ ┌─────────────────┐
                                    │Open local │ │Open browser to  │
                                    │~/code/repo│ │srcuri.com/ext/  │
                                    │/file.rs:42│ │https/...        │
                                    └───────────┘ └─────────────────┘
```

### Git Remote Matching Details

When checking git remotes, the handler:

1. For each configured workspace, reads `.git/config` or runs `git remote -v`
2. Extracts remote URLs (typically `origin`)
3. Normalizes URLs for comparison:
   - `git@github.com:owner/repo.git` → `github.com/owner/repo`
   - `https://github.com/owner/repo.git` → `github.com/owner/repo`
   - `https://github.com/owner/repo` → `github.com/owner/repo`
4. Compares against the provider path from the srcuri URL

This means a link to `srcuri://ext/https/github.com/rust-lang/rust/blob/master/src/lib.rs#L42` will find your local clone whether you named it `rust`, `rustc`, `rust-lang-rust`, or anything else.

---

## srcuri.com Interstitial

When an external URL is opened in the browser (because no local workspace was found), srcuri.com displays an interstitial page.

### Purpose

The interstitial serves users who:
- Don't have Sorcery installed yet
- Have Sorcery but haven't cloned this repository
- Want to view the code without cloning

### Interstitial Content

The page displays:

1. **Repository information**
   - Provider and repository name
   - File path and line number

2. **Action buttons**
   - **"Open in Sorcery"** — Triggers the srcuri protocol (for users who have it installed)
   - **"View on {Provider}"** — Redirects to the original provider URL
   - **"Clone & Open"** — Triggers clone flow with `?remote=` parameter

3. **Install CTA** (for new users)
   - Link to download Sorcery
   - Brief explanation of what Sorcery does

### Fragment Preservation

The interstitial page uses JavaScript to read `window.location.hash`, ensuring line number fragments (`#L42`) are preserved even though they're not sent to the server.

### Redirect Flow

```
srcuri.com/ext/https/github.com/owner/repo/blob/main/file.rs#L42
                          │
                          ▼
              ┌───────────────────────┐
              │   Interstitial Page   │
              │                       │
              │  [Open in Sorcery]────┼──► srcuri://ext/https/github.com/owner/repo/...#L42
              │                       │
              │  [View on GitHub]─────┼──► https://github.com/owner/repo/...#L42
              │                       │
              │  [Clone & Open]───────┼──► srcuri://repo/file.rs:42?remote=github.com/owner/repo
              └───────────────────────┘
```

---

## Git Ref Extraction

Provider URLs embed the git reference (branch, tag, or commit) in the path:

| Provider | Ref Location | Example |
|----------|--------------|---------|
| GitHub | `/blob/{ref}/` | `/blob/main/`, `/blob/v1.0.0/`, `/blob/abc123/` |
| GitLab | `/-/blob/{ref}/` | `/-/blob/main/` |
| Bitbucket | `/src/{ref}/` | `/src/main/` |
| Gitea | `/src/branch/{ref}/` or `/src/commit/{ref}/` | `/src/branch/main/` |

### Extraction

When parsing an external URL, the git ref is extracted and stored:

```rust
git_ref: Option<GitRef>  // Branch("main"), Tag("v1.0.0"), or Commit("abc123")
```

### Local Resolution with Ref

When opening locally with a git ref:

1. **If working tree is on the same ref** → Open file directly
2. **If working tree is on different ref** → Show dialog offering to:
   - View file from that ref in a temporary read-only buffer
   - Checkout the ref (if working tree is clean)
   - Open current version anyway

### Spawning Standard srcuri Links

When the "Clone & Open" flow creates a new workspace, it spawns a standard srcuri link:

```
External URL:
srcuri://ext/https/github.com/owner/repo/blob/main/src/lib.rs#L42

Spawned standard link (after clone):
srcuri://repo/src/lib.rs:42?branch=main
```

---

## Combining with Query Parameters

External URLs can include query parameters that belong to the upstream URL:

### Upstream Query Parameters

Query parameters in ext mode belong to the **upstream URL**, not srcuri:

```
srcuri://ext/https/github.com/owner/repo/blob/main/file.rs?plain=1#L42
                                                           ^^^^^^^^
                                                           Upstream query param
```

This reconstructs to:
```
https://github.com/owner/repo/blob/main/file.rs?plain=1#L42
```

### Parameter + Fragment Ordering

Query parameters come before the fragment (standard URL ordering):

```
srcuri://ext/https/github.com/owner/repo/blob/main/file.rs?param=value#L42
                                                           ^^^^^^^^^^^  ^^^
                                                           query params fragment
```

---

## Practical Examples

### Sharing Code in Slack

```
Hey team, found the bug! It's in the auth handler:
srcuri://ext/https/github.com/our-org/backend/blob/main/src/handlers/auth.rs#L156

The token validation is missing a null check.
```

For teammates with Sorcery: Opens directly in their editor
For others: Opens srcuri.com interstitial with "View on GitHub" option

### Issue Tracker Reference

```
## Bug Report

**Location:** srcuri://ext/https/github.com/project/app/blob/v2.1.0/lib/parser.rs#L89-L95

The parser fails on malformed input. See lines 89-95 where the bounds
check is missing.
```

### Documentation Links

```markdown
## Architecture

The request routing is handled in the [router module](srcuri://ext/https/github.com/our-org/api/blob/main/src/router/mod.rs#L1).

Authentication middleware is defined in [auth.rs](srcuri://ext/https/github.com/our-org/api/blob/main/src/middleware/auth.rs#L25).
```

### Cross-Repository References

```
This is similar to how rustc handles it:
srcuri://ext/https/github.com/rust-lang/rust/blob/master/compiler/rustc_parse/src/parser/mod.rs#L100

We should follow the same pattern in our implementation:
srcuri://ext/https/github.com/our-org/our-compiler/blob/main/src/parser.rs#L50
```

### Converting Existing Links

Before (standard GitHub link):
```
https://github.com/tokio-rs/tokio/blob/master/tokio/src/runtime/mod.rs#L1
```

After (srcuri — restructure scheme/host):
```
srcuri://ext/https/github.com/tokio-rs/tokio/blob/master/tokio/src/runtime/mod.rs#L1
```

Or as a web link:
```
https://srcuri.com/ext/https/github.com/tokio-rs/tokio/blob/master/tokio/src/runtime/mod.rs#L1
```

---

## Security Considerations

### Authority Requirement

The `ext` authority provides explicit opt-in to external URL mode. Without it, paths are never interpreted as provider URLs, preventing accidental external resolution.

### Domain Validation

External mode accepts any domain after `ext/https/`. Implementations should:
- Not automatically clone from unknown domains without user confirmation
- Display the full provider URL in the interstitial for transparency
- Consider maintaining an allowlist of known safe providers for auto-clone

### Path Traversal

Standard path traversal protections apply. The file path extracted from provider URLs is validated:
- No `..` traversal outside repository root
- Symlinks resolved and validated
- Dangerous characters rejected

### Clone Destination

When cloning via the interstitial:
- Clone destination is displayed to user before confirmation
- User must explicitly approve clone operations
- Cloned repositories are added to workspace configuration automatically

---

## Appendix: Grammar

```
external-url         = "srcuri://" "ext" "/" scheme "/" provider-url [ "?" query-string ] [ "#" fragment ]

scheme               = "https" / "http"

provider-url         = hostname "/" path-segments
hostname             = label *( "." label )
label                = 1*( ALPHA / DIGIT / "-" )

path-segments        = segment *( "/" segment )
segment              = 1*( unreserved / pct-encoded )

query-string         = query-param *( "&" query-param )
query-param          = key "=" value
key                  = 1*( ALPHA / DIGIT / "_" )
value                = *( unreserved / pct-encoded )

fragment             = line-fragment
line-fragment        = "L" line-number [ "-" [ "L" ] line-number ]  ; GitHub/GitLab style
                     / "lines-" line-number [ ":" line-number ]     ; Bitbucket style
line-number          = 1*DIGIT
```

### Authority Summary

The complete srcuri authority system:

```
srcuri-link          = "srcuri://" authority "/" path

authority            = reserved-token / workspace-name

reserved-token       = "workspace" / "match" / "abs" / "ext"

; When authority = reserved token:
workspace-mode       = "workspace" "/" workspace-name "/" relative-path [ location ]
match-mode           = "match" "/" relative-path [ location ]
abs-mode             = "abs" "/" absolute-path [ location ]
ext-mode             = "ext" "/" scheme "/" provider-url [ "?" query ] [ "#" fragment ]

; When authority = workspace name (implicit workspace):
implicit-workspace   = workspace-name "/" relative-path [ location ]
```
