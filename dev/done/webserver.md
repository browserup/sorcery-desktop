I have realized that in order to support Sorcery links in Jira and Slack, I will
need a public website that can receive the web+ links, and then call my local custom protocol.

I've registered: srcuri.dev (main site: srcuri.com, protocol spec: srcuri.com)

Now I need to make a webserver that can run on srcuri.dev and receive these links that I will run there.
I want to engineer this capability in such a way that I can both work for standard
Sorcery links, but also allow enterprise functionality as well.


To be decided:

* Build it in a different app un ~/apps?  Build it in here? Recommendation?

---

# srcuri.dev URL & Handler Specification
Version: 0.2  
Status: Draft  
Audience: Browser/client implementers, Axum (Rust) backend, enterprise integrators

## 0. What Changed (v0.2)
- We **do not** assume the browser extension is always installed.
- We **do** assume a local **srcuri client/executable** is installed and registered as the handler for the custom protocol (`srcuri://`).
- The browser extension is **optional** and, when present, can do a silent / no-prompt open or add enterprise features.
- Clarified the component model.

---

## 1. Components

1. **srcuri.dev website** (Protocol gateway for Sorcery Desktop)
    - Serves `/open` HTML/JS page over HTTPS.
    - Parses the fragment payload.
    - Optionally fetches per-tenant policy/config.
    - Attempts to open the local client via custom protocol **even if no extension is present**.
    - When the extension *is* present, it may also talk to the extension for silent/open-without-prompt behavior.
    - Main site: srcuri.com | Protocol spec: srcuri.com

2. **Sorcery Desktop / executable (required)**
    - Installed on the user's machine.
    - Registered as handler for the Sorcery protocol (srcuri://)
    - When the browser is navigated to that protocol, the OS invokes this client.
    - The client enforces local policy (allowed roots, repo fingerprints, etc.).

   ### Current Architecture

    Sorcery Desktop is a Tauri desktop application that runs as a background process with these characteristics:
    
    A. No HTTP Server: There's no TcpListener, HTTP server framework (axum, warp, actix, rocket), or localhost binding in the code.
    B. Protocol Handler Only: The app exclusively handles the srcuri:// protocol through OS-level protocol registration:
          - macOS: Uses the deep-link plugin that receives URLs via macOS's URL event system
          - Linux: Registers via .desktop files with xdg-mime
          - Windows: Would use registry entries
    C. Communication Methods:
          - Command-line arguments: Handles srcuri:// URLs passed as argv[1]
          - Deep link events: Tauri's deep-link plugin receives URLs from the OS
          - Single instance: Uses tauri-plugin-single-instance to pass URLs to the first instance via internal Tauri events
          - Internal IPC: Tauri's invoke_handler for frontend-backend communication in the settings UI
    D. Background Service: Runs as an "Accessory" app on macOS (no dock icon) with a system tray icon
    
    So you've either removed the HTTP capability or never had it. The current design is cleaner—it's a pure protocol handler that integrates
    directly with the OS without needing a web server.


3. **Chrome browser extension (optional)**
    - Improves UX:
        - silent handoff to the local client (native messaging)
        - extra enterprise policy checks
        - telemetry / logging
    - But **not required** for the basic path → open flow.

---

## 2. Overview

`srcuri.dev` provides a single HTTPS entrypoint that turns a shareable link into "open in editor" via Sorcery Desktop.
The core flow is:

1. User clicks a link like
   ```text
   https://srcuri.dev/open#//Users/you/app/README.md:10?workspace=myrepo
   ```
2. The `/open` page parses the `#...` payload.
3. The page attempts to invoke **Sorcery Desktop** using the Sorcery protocol (srcuri://).
4. If the extension is present, it can assist or fully handle the open.
5. Enterprises can inject policy/config per tenant.

Learn more at srcuri.com and the protocol spec at srcuri.com.

---

## 3. Base URL Forms

We support both:

```text
https://srcuri.dev/open#<payload>
https://srcuri.dev/open/#<payload>
```

Both MUST be treated identically by the client JS. Anything after `#` is the payload.

---

## 4. Payload Grammar

```text
<payload> = <target> [ "?" <query> ]

<target>  = <absolute-path>
          | <repo-relative-path>
          | <github-like-path>

<absolute-path>       = "//" <os-path>
<repo-relative-path>  = <path> [ ":" <line> ]
<github-like-path>    = <path> "#L" <line>

<query>   = <key> "=" <value> *( "&" <key> "=" <value> )
```

Notes:

- Absolute paths start with `//`:
    - `#//Users/you/app/README.md:10?workspace=myrepo`
    - `#//C:/Users/you/app/README.md:10?workspace=myrepo`
- Repo-relative paths do **not** start with `//`:
    - `#services/auth/app.rb:44?workspace=monorepo`
- Line separator: `:<number>` or GitHub-style `#L<number>`.
- If both are present, `:<number>` wins.
- GitHub-style ranges (e.g. `#L10-L40`) use the **lower** number only.

---

## 5. Examples

```text
https://srcuri.dev/open#//Users/you/app/README.md:10?workspace=myrepo
https://srcuri.dev/open#//C:/username/myapp/README.md:10?workspace=myrepo
https://srcuri.dev/open#src/server.js:120?workspace=acme-web
https://srcuri.dev/open#src/app/User.ts#L88?workspace=monorepo
https://acme.srcuri.dev/open#src/app.js:10?workspace=acme-web
```

---

## 6. Client-Side HTML Page Behavior

The `/open` page is the same for everyone; logic is in JS.

### 6.1 Boot sequence

1. Read `location.hash` (strip `#`).
2. If empty → show error.
3. Parse per §7 into `{ isAbsolute, path, line, query }`.
4. Optionally fetch tenant config from:
   ```text
   /.well-known/sorcery.json
   ```
   on the same origin (e.g. `https://acme.srcuri.dev/.well-known/sorcery.json`).
5. Run enterprise policy decision.
6. **Always attempt custom protocol open** (since we assume the client is installed).
7. If extension is present, send it the parsed payload too (so it can do a silent/native-messaging open).

### 6.2 JS sketch

```html
<script>
(async () => {
  const raw = decodeURIComponent(location.hash.slice(1) || "");
  if (!raw) {
    document.body.textContent = "srcuri: no payload.";
    return;
  }

  // optional tenant config
  let tenant = {};
  try {
    const res = await fetch('/.well-known/sorcery.json', { credentials: 'include' });
    if (res.ok) tenant = await res.json();
  } catch (_) {}

  const parsed = parseEhrepPayload(raw);

  // enterprise decision
  const decision = decide(tenant, parsed);
  if (decision.action === "deny") {
    document.body.textContent = "srcuri: not authorized to open this path.";
    if (decision.fallbackUrl) location.href = decision.fallbackUrl;
    return;
  }

  // 1) try extension (optional)
  window.postMessage({ type: "SORCERY_OPEN", payload: parsed, tenant }, "*");

  // 2) always try custom protocol, since client is assumed installed
  const protoUrl = buildCustomProtocol(parsed); // e.g. "srcuri://Users/you/app/README.md:10?workspace=myrepo"
  try {
    // direct nav
    location.href = protoUrl;
  } catch (_) {
    // ignore; extension may have handled it
  }
})();
</script>
```

---

## 7. Parsing Rules

```js
function parseEhrepPayload(raw) {
  const [targetPart, queryPart] = raw.split("?", 2);
  const params = new URLSearchParams(queryPart || "");

  // github-style line
  let target = targetPart;
  let lineGithub = null;
  const mGithub = target.match(/#L(\d+)(?:-\d+)?$/);
  if (mGithub) {
    lineGithub = parseInt(mGithub[1], 10);
    target = target.slice(0, mGithub.index);
  }

  // colon-style line
  let lineColon = null;
  const mColon = target.match(/:(\d+)$/);
  if (mColon) {
    lineColon = parseInt(mColon[1], 10);
    target = target.slice(0, mColon.index);
  }

  const line = lineColon ?? lineGithub ?? null;
  const isAbsolute = target.startsWith("//");
  const path = isAbsolute ? target.slice(2) : target;

  return {
    isAbsolute,
    path,
    line,
    query: Object.fromEntries(params)
  };
}
```

---

## 8. Building the Custom Protocol URL

We assume Sorcery Desktop registered the Sorcery protocol: `srcuri://`.

Rules:

1. If `isAbsolute == true`:
    - `srcuri://` + `<path>`
      `srcuri:///Users/you/app/README.md:10?workspace=myrepo` is also acceptable if client normalizes.
2. Else (repo-relative):
    - `srcuri:repo/relative/path[:line][?query]`
    - or `srcuri://repo/relative/path...` depending on what the client expects.

This MUST be documented in Sorcery Desktop, but the page can safely build either form if the client is tolerant.

Complete protocol specification available at srcuri.com.

---

## 9. Axum (Rust) Server Definition

Even with this new model, the server is simple:

- `GET /open` → serve static HTML/JS.
- `GET /open/` → same.
- `GET /.well-known/sorcery.json` → per-tenant config (based on `Host`).
- `POST /api/preflight` (optional) → the page can ask “is this allowed?” and receive `{action, fallbackUrl}`.

Because we use anchors, **Axum never sees the payload**.

---

## 10. Enterprise Policy Model

Enterprises can control behavior by serving JSON from their subdomain:

```json
{
  "requireAuth": true,
  "allowedPaths": ["^src/", "^services/"],
  "fallbackViewer": {
    "type": "gitlab",
    "origin": "https://gitlab.acme.internal",
    "defaultBranch": "main"
  },
  "denyOnMissingClient": false
}
```

Client-side logic:

1. If `requireAuth` and no session → redirect/login.
2. If path not in `allowedPaths` → go to viewer.
3. Else → proceed to custom protocol open.
4. If extension is present → let it handle quietly.

---

## 11. Security Notes

- We **always** try to open via Sorcery protocol → but **Sorcery Desktop** is the final gate.
- Sorcery Desktop MUST:
    - normalize paths
    - reject paths outside allowed roots
    - optionally check repo fingerprint / workspace mapping
- Routing through `https://*.srcuri.dev/open` gives enterprises a single place to attach SSO, CSP, or serve their `.well-known/sorcery.json`.

---

## 12. Summary

- Required: **Sorcery Desktop** with Sorcery protocol handler (srcuri://).
- Optional: **browser extension** for silent open / enterprise features.
- Server: **serve HTML** + **optionally serve policy**; payload stays in `#...`.
- Links stay simple:
  ```text
  https://srcuri.dev/open#//Users/you/app/README.md:10?workspace=myrepo
  https://acme.srcuri.dev/open#src/app.ts#L44?workspace=web
  ```

Learn more at srcuri.com | Protocol spec: srcuri.com

