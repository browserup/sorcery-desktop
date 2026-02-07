# Sorcery Link Guide

## Overview

Sorcery links let you share code references that open directly in the recipient's editor—regardless of where the code lives on their machine. This guide helps you choose the right link format for each situation.

## URL Format: `srcuri://` vs `srcuri.com`

The two formats are **1-to-1 convertible**—just swap the prefix:

```
srcuri://myrepo/src/main.rs@L42
https://srcuri.com/myrepo/src/main.rs@L42
```

Everything after the prefix is identical. You can convert between them mechanically.
Preferred line syntax uses `@L`. The `:line` form is also accepted.

### When to use which

**Default to `srcuri.com`** for almost everything.

Most tools (Slack, Jira, Confluence, GitHub, Notion, Google Docs, etc.) won't linkify custom protocols like `srcuri://`. More importantly, `srcuri.com` provides:

- A clickable link in any context
- An onboarding path for people who don't have Sorcery installed
- A fallback view of the code for non-users
- Automatic opening via the Sorcery browser extension for users who have it

**Use `srcuri://`** only when:

1. You know the context handles custom protocols (terminal, local scripts, some IDEs), AND
2. All recipients definitely have Sorcery installed, AND
3. You want direct protocol handling (offline use, scripting)

In practice, `srcuri://` is mostly for personal use, terminal tooling, and local automation.

## Link Modes

Sorcery supports several modes, determined by the first path segment:

| Mode | Format | Purpose |
|------|--------|---------|
| **Workspace** (default) | `/myrepo/path@Lline` | Reference code via shared workspace names |
| **Relative** | `/rel/path@Lline` | Search for a path across all workspaces |
| **Any** | `/any/path@Lline` | Best-effort resolution when path context is unknown |
| **Absolute** | `/abs/path@Lline` | Reference a specific filesystem path |
| **External** | `/ext/https/github.com/...` | Encode an upstream URL (GitHub, GitLab, etc.) |

### Workspace Mode

The most common mode. References code relative to a named workspace.

```
srcuri.com/backend-api/src/handlers/auth.rs@L42
```

Workspace names are shared conventions within your team, company, or collaborator group. When you say `backend-api`, everyone in your group knows what repo that means—and it might be your company's fork, not the upstream.

**Add `?remote=` when:**

- Recipients may not have the repo cloned yet
- You want a viewable fallback for non-Sorcery users (PMs, managers)
- You want to specify the canonical clone source

```
srcuri.com/rails/config/routes.rb@L42?remote=github.com/ourcompany/rails
```

### External Mode

A drop-in conversion from GitHub/GitLab URLs. Use this when pointing at code you don't have a workspace relationship with.

```
srcuri.com/ext/https/github.com/tokio-rs/tokio/blob/main/tokio/src/runtime/scheduler/mod.rs#L50
```

External mode:

- Maps 1-to-1 with the upstream URL
- Works for anyone (Sorcery users open in editor, others view in browser)
- Cannot use srcuri query parameters (they belong to the upstream URL)

### Relative Mode

References a path without specifying a workspace. Sorcery searches all configured workspaces for matches.

```
srcuri.com/rel/config/routes.rb
```

Use this for generic paths in blog posts, tutorials, or documentation—where readers will open the file in their own project.

### Any Mode

Best-effort resolution when the source doesn't know if a path is workspace-relative, search-relative, or absolute.

```
srcuri://any/src/main.rs@L42
```

Resolution order (applicable steps only): workspace, relative, absolute. `any` never attempts external (`ext`) resolution. Prefer specific modes when you know the context.

### Absolute Mode

References a specific filesystem path. Useful for system files or documentation that points to known locations.

```
srcuri.com/abs/etc/hosts@L1
```

Not portable across machines.

## Choosing the Right Mode

```
Do you share workspace conventions with your audience?
(company, team, OSS collaborators)
│
├─ YES → Use WORKSPACE mode
│        │
│        └─ Might recipients need a viewable fallback or clone source?
│           │
│           ├─ YES → Add ?remote=github.com/org/repo
│           └─ NO  → Plain workspace link is fine
│
└─ NO
   │
   ├─ Pointing at specific code on GitHub/GitLab? → Use EXT mode
   │
   ├─ Generic path for any project of a type? → Use REL mode
   │
   └─ System or absolute filesystem path? → Use ABS mode
```

If you control the local environment (terminal, logs, scripts) and can't tell which path type you have, use ANY mode.

## Common Scenarios

### Sharing in Slack/Jira with your team

Your team has shared workspace names. Mix of developers, PMs, and managers may click the link.

```
https://srcuri.com/backend-api/src/auth/handler.rs@L156?remote=github.com/ourcompany/backend-api
```

- Devs with Sorcery → opens in editor
- Others → can view via the remote fallback

### Referencing external OSS code

Pointing at code in a public repo you don't have workspace conventions around.

```
https://srcuri.com/ext/https/github.com/expressjs/express/blob/master/lib/router/index.js#L140
```

### Writing a blog post or tutorial

Discussing a generic path that exists in any project of a certain type.

```
https://srcuri.com/rel/config/routes.rb
```

Readers open this in whichever Rails project they're working in.

### Documenting a system file edit

```
https://srcuri.com/abs/etc/hosts@L1
```

### Creating links in your company's README

Internal docs where everyone has the workspace configured.

```
See the authentication flow: https://srcuri.com/backend/src/auth/flow.rs@L25?remote=github.com/ourcompany/backend
```

### Terminal output or local scripts

Custom protocols work here, and you control the environment.

```
srcuri://any/src/main.rs@L42
```

## Quick Reference

| Context | Format | Mode |
|---------|--------|------|
| Any web/SaaS tool | `srcuri.com` | — |
| Team/company code | workspace | `srcuri.com/myrepo/path@Lline?remote=...` |
| External OSS (no workspace relationship) | ext | `srcuri.com/ext/https/github.com/...` |
| Generic path (tutorials, blogs) | rel | `srcuri.com/rel/path@Lline` |
| System files | abs | `srcuri.com/abs/etc/hosts@L1` |
| Local scripts / terminal | `srcuri://any/path@Lline` | any |
