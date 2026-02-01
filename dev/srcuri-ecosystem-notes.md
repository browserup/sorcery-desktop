# Sorcery protocol and ecosystem notes (draft)

Status: draft. Protocol is unreleased and can change.

## Scope

This doc captures flow, protocol shapes, and open questions. It links into the three repos for fast auditing.

## Components and roles

- Sorcery Desktop: registers `srcuri://`, parses requests, and resolves workspace/line/column. srcuri://sorcery-desktop/src-tauri/src/protocol_handler/parser.rs@L165
- Sorcery Server: HTTPS gateway for web contexts and provider passthrough. srcuri://sorcery-server/README.md@L7
- Sorcery Extension: detects file references in web pages and opens `srcuri://` links with a modifier click. srcuri://sorcery-extension/README.md@L44 and srcuri://sorcery-extension/src/content/cs.ts@L25

## Interaction flow

### 1. Direct protocol (desktop)

- `srcuri://...` is parsed and dispatched by the desktop handler. srcuri://sorcery-desktop/src-tauri/src/protocol_handler/parser.rs@L165

### 2. Web gateway (srcuri.com)

- Direct gateway: JS reads the hash payload and builds a `srcuri://` redirect. srcuri://sorcery-server/src/static/app.js@L90
- Mirror mode: server parses the path and builds a `srcuri://` URL (with `@L`). srcuri://sorcery-server/src/routes/passthrough.rs@L473
- Provider passthrough: client JS reads `window.location.hash`, parses the provider URL, and redirects. srcuri://sorcery-server/src/templates/provider.html@L945
- Provider passthrough JS converts `@L` (or `:N`) into `#L<N>` if no fragment is present. srcuri://sorcery-server/src/templates/provider.html@L974

### 3. Browser extension

- The extension scans the DOM, tags detected elements, and opens a `srcuri://` URL on modifier click. srcuri://sorcery-extension/src/content/cs.ts@L25
- It emits `srcuri://any/...` for local paths and `srcuri://ext/...` for provider URLs. srcuri://sorcery-extension/src/shared/srcuri.ts@L105
- Detection rules are YAML-driven. srcuri://sorcery-extension/detection/sites.yaml@L1

## Protocol format (draft spec)

- Canonical format: `srcuri://<authority>/<path>[@Lline[Ccol]][?<query>][#<fragment>]` with legacy `:line[:col]` accepted. srcuri://sorcery-desktop/dev/srcuri-protocol-spec-v1.md@L79
- Modes: implicit workspace, `wks`, `rel`, `any`, `abs`, `ext`. srcuri://sorcery-desktop/dev/srcuri-protocol-spec-v1.md@L110
- `ext` mode encodes a provider URL, and its `?query`/`#fragment` belong to the upstream URL. srcuri://sorcery-desktop/dev/srcuri-protocol-spec-v1.md@L481
- Line/column rules prefer `@Lline[Ccol]` on the final path segment. srcuri://sorcery-desktop/dev/srcuri-protocol-spec-v1.md@L580

## Provider parsing and line extraction

- `srcuri-core` parses provider URLs and extracts `@L`, `:N`, and `#L` fragments. srcuri://sorcery-desktop/srcuri-core/src/parser.rs@L4
- Desktop also parses provider fragments (`#L10`, `#L10C5`, `#lines-5`). srcuri://sorcery-desktop/src-tauri/src/protocol_handler/parser.rs@L477

## Sources and formats we handle

- Provider URLs: GitHub, GitLab, Bitbucket, Gitea, Codeberg, Azure DevOps. srcuri://sorcery-server/README.md@L49
- Web stack traces, error pages, CI logs, APM traces, and code-host UIs via extension detectors. srcuri://sorcery-extension/README.md@L100

## Anchors and fragments

- Browser fragments are not sent to servers; provider passthrough must read `window.location.hash` client-side. srcuri://sorcery-desktop/srcuri-core/README.md@L36
- Server JS parses `window.location.hash` and builds `srcuri://` URLs. srcuri://sorcery-server/src/templates/provider.html@L945

## Linkification surfaces

- Slack/Jira/Teams/markdown: `https://srcuri.com/...@L` is the shareable shape; OG previews can include `@L` for direct gateway links, but not for provider passthrough fragments. srcuri://sorcery-server/README.md@L225
- Chrome DevTools console: observed to drop `:line` (legacy). `@L` should avoid port parsing; verify.
- Terminals: `file:line` is common, so keep `:line` parsing for compatibility.

## Notes from current debate

- Canonical line syntax is `@L` (emit uppercase). Accept `@l`, `%40L`, and legacy `:line` on input.
- Empty marker `@L` is accepted to allow template-based link construction without conditionals.
- Columns: emit `@LlineCcol`; accept `@Lline:col` and legacy `:line:col`.
- `ext` remains the one-step provider on-ramp; `@L` inside the path is converted to `#L` when needed.

## Broad-strokes changes to explore

These are first-principles options. None are decisions.

1. Split protocols: keep `srcuri://` for local/workspace/abs/rel/any only, and introduce a second scheme for provider URLs (or move provider-only parsing to `srcuri.com`).
2. Move `ext` to a separate gateway path or scheme to reduce fragment coupling while keeping one-step prefixing.
3. Line as query (`?line=42&col=10`) for shareable URLs, while still accepting `:line` for terminal input.
4. Unified canonicalization: accept many inputs, always emit `@L` (desktop, server, extension).

## Open questions

- Which tools URL-encode `@` into `%40` in practice (Slack, Jira, Notion, Chrome)?
- Do we want line previews for internal links (requires workspace access or integrations)?
- Can we keep the one-step provider prefix while decoupling line handling from fragments?
