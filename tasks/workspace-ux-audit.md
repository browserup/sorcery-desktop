# Workspace UX Audit and Terminology Proposal

## Current State: How It Works

### What is a workspace?

A workspace is a local folder (usually a git repo) that Sorcery tracks so it can open `srcuri://` links in the right editor. When someone sends a link like `srcuri://drill/README.md@L75`, Sorcery needs to know where "drill" lives on disk and which editor to open it in.

### The data model behind the scenes

Each workspace mapping stores:

| Internal field | What it actually means |
|---|---|
| `workspace_key` | The **link name** -- the word used in `srcuri://<this>/...` URLs |
| `workspace_path` | The folder on disk |
| `workspace_kind` | Git repo or plain folder |
| `repo_identity` | The normalized git remote (e.g. `github.com/fcsonline/drill`) |
| `workspace_state` | Health status: is the folder still there? Has it changed? |
| `trusted` | Has the user verified this mapping is safe to open? |
| `editor` | Which editor to use (or "default") |

### What the user actually does

1. **Add a folder** -- point Sorcery at a local repo/folder
2. **Choose an editor** -- pick which editor opens files from that folder
3. **Follow links** -- click `srcuri://` links and the right file opens in the right editor

Everything else (key management, identity tracking, health reconciliation) is plumbing that supports those three actions.

---

## Current UI: What's Confusing

### The workspace card (see screenshot)

```
[HEALTHY]  Key: drill  Type: Git  Remote: github.com/fcsonline/drill  Untrusted

WORKSPACE KEY          EDITOR
[drill              ]  [Default (Visual Studio Code)  v]

PATH
[/Users/ebeland/apps/drill        ]  [Rename] [Rebind] [Test] [Forget]
```

### Problems identified

| Element | Problem |
|---|---|
| **"Key: drill"** | Users don't know what a "key" is. It's an internal protocol concept. |
| **"WORKSPACE KEY" label** | Doubles down on the jargon. The user just sees a name. |
| **"Type: Git"** | Low-value metadata cluttering the header. Users know if their project is a git repo. |
| **"Remote: github.com/..."** | Useful for disambiguation, but presented as raw metadata rather than context. |
| **"Untrusted"** | Alarming without explanation. What does it mean? What should I do? |
| **"Rename" button** | Rename what? The key? The folder? Confusing because the name field is right there and editable. |
| **"Rebind" button** | Internal jargon. Means "change which folder this points to." No user thinks in terms of "binding." |
| **"Test" button** | Test what? A user might think it tests the repo, not that it opens a test file in the editor. |
| **"Forget" button** | Reasonable, but the red styling makes it feel dangerous. What happens to my files? |
| **"Resolve" button** | Appears on unhealthy workspaces. Resolve what? The user has no context. |
| **"Reconcile" button** | In the health banner header. Pure engineering terminology. |
| **Status badges** | "Healthy" is fine. "Drifted" means nothing to a user. "Conflict" is vague. |

### Information hierarchy issues

- The metadata bar (`Key: drill  Type: Git  Remote: ...  Untrusted`) mixes status, identity, and type information at the same visual level.
- The "Workspace Key" is both editable inline AND has a separate "Rename" button -- redundant and confusing.
- Actions aren't grouped by intent. "Rename" is about the link name, "Rebind" is about the folder, "Test" is about the editor, "Forget" is about removal -- four unrelated concerns in a flat row.

---

## Proposed Terminology Mapping

### Rename internal concepts for the user

| Internal term | User-facing term | Reason |
|---|---|---|
| `workspace_key` | **Link Name** | It's the name used in shared links. That's what users care about. |
| `workspace_path` | **Folder** | It's a folder on disk. |
| `workspace_kind` | *(hide or show as icon)* | A small git icon is enough. No label needed. |
| `repo_identity` | **Repository** | Show only when relevant (disambiguation). |
| `workspace_state: present` | **OK** or just no badge | Don't show a badge when nothing is wrong. |
| `workspace_state: missing` | **Folder Not Found** | Say what happened. |
| `workspace_state: unavailable` | **Folder Unavailable** | Say what happened. |
| `workspace_state: identity_drift` | **Repository Changed** | Say what happened. |
| `workspace_state: conflict` | **Conflict** | Keep -- but add explanation. |
| `trusted` | *(remove from main view)* | Trust is a safety mechanism, not a user setting. Handle it at link-open time. |
| Rename (button) | *(remove)* | The name field is already editable. Just save on blur/enter. |
| Rebind (button) | **Change Folder...** | Say what it does. |
| Test (button) | **Open Test File** | Say what it does, or better: just show a small "try it" link. |
| Forget (button) | **Remove** | Standard term. Less alarming than "forget." |
| Resolve (button) | **Fix...** | Short, clear, actionable. |
| Reconcile | **Refresh** | Already used on the icon button. Use it consistently. |

---

## Proposed Card Layout

### Healthy workspace (common case)

```
drill                              [Default (Visual Studio Code)  v]
/Users/ebeland/apps/drill          github.com/fcsonline/drill
                                           [Change Folder...]  [Remove]
```

- **Name** is prominent, editable inline (saves on blur).
- **Editor** dropdown stays.
- **Path** is shown but read-only (change via "Change Folder...").
- **Repository** shown in secondary text only for git repos.
- No status badge when healthy -- absence of a badge = everything is fine.
- "Open Test File" can be a small secondary link or move to a `...` overflow menu.

### Unhealthy workspace

```
⚠ Folder Not Found

drill                              [Default (Visual Studio Code)  v]
/Users/ebeland/apps/drill          github.com/fcsonline/drill
                              [Locate Folder...]  [Remove]
```

- Status shown as a colored inline warning **above** the card content.
- "Locate Folder..." replaces both "Rebind" and "Resolve" -- it's the same action (pick a new folder) with a user-understandable name.
- For "Repository Changed" status, show: `"The git repository at this location has changed since it was added."`  with actions: **[Update]** (accept new identity) and **[Locate Folder...]** (point to correct folder).

### Auto-discovered workspace

```
drill  AUTO                        Default editor
/Users/ebeland/apps/drill
                               [Add to My Workspaces]  [Ignore]
```

- "Customize" becomes **"Add to My Workspaces"** -- says what happens.
- "Remove" (x button) becomes **"Ignore"** -- says what happens (it goes to ignored list, not deleted from disk).

---

## Proposed Health Banner

Current:
```
Workspace health issues detected
                                           [Review]  [Reconcile]
```

Proposed:
```
Some workspaces need attention
  1 folder not found, 1 repository changed
                                           [Review]  [Refresh]
```

- "health issues" is clinical. "need attention" is plain language.
- The summary line tells you exactly what's wrong without opening anything.
- "Reconcile" becomes "Refresh" (already used on the refresh icon).

---

## Proposed Page Header

Current:
```
Workspace Mappings                        [⟳]  [+ Add Workspace]
Map workspace directories to specific editors.
```

Proposed:
```
Workspaces                                [⟳]  [+ Add Folder]
Folders where your projects live. Links open files in the editor you choose.
```

- "Mappings" is an internal data concept. Users add workspaces, not mappings.
- "Add Workspace" becomes **"Add Folder"** -- concrete noun.
- Help text explains the *why*, not the *how*.

---

## Summary of Changes

### Labels to change

| Current | Proposed |
|---|---|
| Workspace Mappings | Workspaces |
| Workspace Key | Name |
| Path | Folder |
| + Add Workspace | + Add Folder |
| Key: drill | *(remove from metadata bar)* |
| Type: Git | *(show as small icon if git, otherwise omit)* |
| Remote: ... | *(show as secondary text under folder path)* |
| Untrusted / Trusted | *(remove from card; handle at link-open time)* |

### Buttons to change

| Current | Proposed |
|---|---|
| Rename | *(remove; inline edit is sufficient)* |
| Rebind | Change Folder... |
| Test | Open Test File *(or move to overflow)* |
| Forget | Remove |
| Resolve | Fix... *(or context-specific: "Locate Folder...")* |
| Reconcile | Refresh |
| Customize (discovered) | Add to My Workspaces |

### Status badges to change

| Current | Proposed |
|---|---|
| Healthy | *(no badge -- absence = healthy)* |
| Missing | Folder Not Found |
| Unavailable | Folder Unavailable |
| Drifted | Repository Changed |
| Conflict | Conflict *(keep, add explanation tooltip)* |
