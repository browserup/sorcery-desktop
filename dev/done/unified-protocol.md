# LLM Review Packet: Align Sorcery Server + Sorcery Desktop with “No `go`” Unified Linking

**Date:** 2025-12-21  
**Audience:** An LLM reviewing two codebases (Sorcery Server + Sorcery Desktop) and proposing concrete alignment changes.

## Goal

Review **both** projects and propose specific changes so they align with our updated “no `go.` subdomain” strategy and unified link-resolution model. Output should be actionable: file-level recommendations, behavior changes, and test updates.

### Repos / Docs to read (paths are illustrative; use the repo’s actual layout)

- Sorcery Server:
  - `./overview.md`
  - `./dev/remote-links-spec.md` (web server protocol spec as it stands)
- Sorcery Desktop:
  - `./dev/srcuri-protocol-spec.md` (local protocol spec)
- This packet (the new learnings and intended behavior): **you are reading it now**.

---

## New Learnings and Key Decisions

### Decision: **No `go.srcuri.com` subdomain**
We are consolidating on:
- **Web:** `https://srcuri.com/...`
- **Protocol:** `srcuri://...`

We still support multiple “payload shapes” under the same host/scheme.

### Core principle: One Target, Multiple Carriers
All inputs should map to a single internal target model:

```text
Target {
  remote: optional "host/org/repo" (canonical identity)
  repo_name: "repo"
  ref: optional branch/tag/sha
  file_path: optional
  line: optional
  column: optional
}
```

### Workspace naming constraint for deterministic parsing
**Workspace IDs MUST NOT contain dots (`.`).**

Rationale: makes it unambiguous to distinguish provider-host payloads from workspace payloads.

Recommended workspace id regex: `[A-Za-z0-9_-]+`

---

## Link Types and Syntax (Canonical)

### 1) Provider-Passthrough Web Link (viral on-ramp)
**Purpose:** Start from existing repo URLs (GitHub/GitLab/etc). Works without the browser extension.

**Syntax:**
```text
https://srcuri.com/<provider-host>/<provider-path>[#fragment]
```

**Examples:**
- Repo: `https://srcuri.com/github.com/ericbeland/enhanced_errors`
- File + line (GitHub): `https://srcuri.com/github.com/owner/repo/blob/main/file.rs#L42`
- Self-hosted GitLab: `https://srcuri.com/gitlab.myco.com/group/proj/-/blob/main/app.py#L10`

**Critical behavior:** provider line references often live in fragments like `#L42` which the server will not see. Therefore, provider-passthrough web requests MUST be served via an HTML+JS interstitial that reads `window.location.hash` client-side and constructs the `srcuri://...` redirect.

### 2) Workspace Mirror Web Link (share everywhere)
**Purpose:** Share in Slack/Teams/Jira: always clickable HTTPS link with install fallback.

**Syntax:**
```text
https://srcuri.com/<workspace>/<path>[:line[:column]]?[query]
```

**Examples:**
- `https://srcuri.com/enhanced_errors/src/lib.rs:42?branch=main&remote=github.com/ericbeland/enhanced_errors`

**Behavior:** HTML+JS interstitial that attempts `srcuri://...` and shows an install/help UI if handler missing.

### 3) Native Protocol Links (desktop handler)
**Purpose:** Fast direct open when Sorcery Desktop installed.

**Syntax A (workspace form):**
```text
srcuri://<workspace>/<path>[:line[:column]]?[query]
```

**Syntax B (provider-passthrough form):**
```text
srcuri://<provider-host>/<provider-path>[#fragment]
```

**Examples:**
- `srcuri://enhanced_errors/src/lib.rs:42?branch=main&remote=github.com/ericbeland/enhanced_errors`
- `srcuri://github.com/owner/repo/blob/main/file.rs#L42`
- `srcuri://github.com/ericbeland/enhanced_errors`

Desktop MUST support both forms.

### 4) Optional / Legacy Query-Based Translator (web)
**Syntax:**
```text
https://srcuri.com/?remote=<encoded provider url>
```

This may remain for programmatic integrations and as an escape hatch.

---

## Deterministic Mode Detection

### Web (`https://srcuri.com/...`)

Given request path `/X/...`:

1. If path is `/` and query has `remote=` → **Query Translator**
2. Else if first segment `X` contains a dot (`.`) → **Provider-Passthrough Web**
3. Else if path is `/` → **Landing Page**
4. Else → **Workspace Mirror Web**

### Desktop (`srcuri://...`)

Given `srcuri://AUTHORITY/...`:

1. If `AUTHORITY` contains a dot (`.`) → **Provider-Passthrough Protocol**
2. Else → **Workspace Protocol**

---

## Behavioral Ownership: Desktop vs Web

### Sorcery Desktop owns
- Parsing **all** `srcuri://` inputs (workspace + provider-passthrough)
- Mapping `remote → local repo path/workspace`
- Disambiguation (multiple local matches, forks/mirrors)
- Clone prompts when missing
- Editor dispatch + open file/line/column
- Persisting mappings and preferences

### Sorcery Server owns
- Making links usable in restricted surfaces (Slack/Teams/Jira)
- Install fallback + “open in browser” options
- Provider URL translation (including fragment-based line numbers via JS interstitial)
- OG unfurl metadata (may be generic for fragment-only line numbers)

---

## Resolution Semantics: What happens on click?

### Protocol click (desktop installed)
For `srcuri://github.com/ericbeland/enhanced_errors`:

1. Desktop parses provider form:
   - `remote = github.com/ericbeland/enhanced_errors`
   - `repo_name = enhanced_errors`
2. Desktop resolves local workspace:
   - Prefer exact mapping cache (remote → local path/workspace)
   - Else scan known repo roots; match `.git/config` remotes
   - Else heuristic search (MRU, common dev folders, etc.)
3. Desktop outcomes:
   - Exactly one match → open repo root in editor
   - Multiple matches → prompt user, then persist mapping
   - No match → prompt to clone / choose destination, or open in browser

### HTTPS click
For `https://srcuri.com/...`:

- Server returns HTML+JS interstitial
- JS constructs `srcuri://...`
- Browser attempts to open the protocol handler
- If not installed → show install UI + fallback actions

**Note:** If the user does not have Sorcery Desktop installed, `srcuri://...` cannot open. That’s why onboarding/invites must use HTTPS links.

---

## Guideline: When to include `remote=`
**Default behavior for generated links:** include `remote=` whenever known (even when using workspace-style links).

Reasons:
- disambiguation (repo might exist on GitHub and GitLab; forks; mirrors)
- onboarding (if recipient doesn’t have repo: clone prompt)
- portability across machines and workspace renames

Example recommended output from IDE / extension / tooling:
```text
https://srcuri.com/enhanced_errors/src/lib.rs:42?remote=github.com/ericbeland/enhanced_errors&branch=main
```

---

# LLM Tasks (What you must produce)

## A) Inventory and Gap Analysis (per repo)
For **Sorcery Server** and **Sorcery Desktop**, report:
1. What link formats are currently supported?
2. Where do they diverge from this packet?
3. What user-visible rules would be required today (and are they too complex)?
4. What behavior belongs in desktop vs web today, and what is misplaced?

## B) Concrete Alignment Plan
Provide a prioritized list of changes, grouped by repo.

For each change:
- **What to change**
- **Why**
- **Where (files/modules)**
- **Impact**
- **Tests to add/update**

## C) Deterministic parsing + collision handling
Specifically address:
- workspace IDs must not contain dots: how to validate/enforce and how to provide an escape hatch if needed
- how to canonicalize and normalize remotes (e.g., strip scheme, handle trailing `.git`, case normalization)
- how to handle ambiguous matches (multiple local clones; fork vs upstream)

## D) Web interstitial behavior (server)
Confirm or propose:
- Provider-passthrough requests must be served via HTML+JS so fragments are preserved
- Mirror workspace requests also served via HTML+JS for consistent install fallback
- Redirect strategy (pure 302 vs HTML page) for each mode
- OG tags: what can be accurate vs what must be generic when line numbers are only in fragments

## E) Desktop resolver behavior
Confirm or propose:
- provider-passthrough `srcuri://github.com/...` parsing supported
- repo discovery approach and caching
- clone prompts / onboarding flow
- how remote identity is stored and used for future opens

## F) Test matrix updates (must be explicit)
Update/expand tests to include at least:
- Web mode detection:
  - provider passthrough (`/github.com/...`)
  - self-hosted GitLab via path pattern + dotted host (`/gitlab.myco.com/.../-/blob/...`)
  - workspace mirror (`/myrepo/...`)
  - query translator (`/?remote=...`)
- Fragment-driven line formats (GitHub `#L42`, ranges `#L10-L20`, GitLab `#L10`, Bitbucket `#lines-5:10`, Azure query params)
- Workspace ids containing dots (should be rejected or require escape hatch)
- Canonicalization of remotes (`https://`, `ssh`, `.git`, trailing slash)

## G) Deliverables format
Your output must include:
1. **Executive summary** (1–2 screens)
2. **Repo-by-repo alignment list**
3. **“Before/After” examples** for at least 8 common scenarios
4. **Test plan** with cases and expected outcomes
5. **Risk notes** (backward compatibility, breaking changes, migration path)

---

# Reference Examples for “Before/After” Section

## Provider → Sorcery Web
- Before: `https://github.com/owner/repo/blob/main/file.rs#L42`
- After:  `https://srcuri.com/github.com/owner/repo/blob/main/file.rs#L42`

## Sorcery Web → Protocol
- Before: `https://srcuri.com/github.com/owner/repo/blob/main/file.rs#L42`
- After:  `srcuri://github.com/owner/repo/blob/main/file.rs#L42`

## Workspace Mirror Web (portable)
- `https://srcuri.com/repo/src/lib.rs:42?remote=github.com/owner/repo&branch=main`

## Workspace Protocol (portable)
- `srcuri://repo/src/lib.rs:42?remote=github.com/owner/repo&branch=main`

## Provider Protocol (portable)
- `srcuri://github.com/owner/repo/blob/main/file.rs#L42`

---

# Notes / Constraints

- Do NOT propose taking over `http` / `https` handling at OS level.
- Favor lowest user mental overhead.
- Prefer behaviors that work **without** the browser extension.
- Defensibility is valuable but secondary to adoptability.

---

## Prompt starter (optional)
You may start your analysis with:

> Read Sorcery Server and Sorcery Desktop source plus the referenced specs. Identify current behavior vs this packet’s decisions. Propose an alignment plan with concrete code changes, tests, and migration notes so both repos implement the “no go subdomain” unified model, including provider-passthrough at `srcuri.com/<host>/...` and desktop support for `srcuri://<host>/...`.

