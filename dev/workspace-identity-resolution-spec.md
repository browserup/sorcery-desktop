# Workspace Identity and Resolution Spec (Draft)

## Purpose

Define a deterministic workspace model that stays simple for common setups (`~/apps/<repo>`) and handles collisions, forks, worktrees, and enterprise policy controls.

## Scope

This spec covers:
- workspace identity and mapping invariants
- link resolution and clone decision rules
- UI behavior for conflicts and repair
- refresh and reconciliation after filesystem drift

This spec does not change srcuri URL syntax in this phase.

Implementation sequencing is tracked in `dev/workspace-identity-implementation-plan.md`.

## Design goals

1. Make the common case frictionless: one top-level directory with many repo folders.
2. Keep names deterministic: one workspace key maps to one active local workspace.
3. Support non-git folders as first-class workspaces.
4. Disambiguate git clones with remote identity when available.
5. Preserve security defaults: no silent trust, no silent remap on identity drift.
6. Keep enterprise control optional but enforceable when configured.

## Definitions

- `workspace_key`: The URL-facing name (authority for `srcuri://<workspace_key>/...`).
- `workspace_path`: User-facing configured absolute path.
- `canonical_path`: Normalized filesystem path when path exists.
- `workspace_kind`: `git` or `non_git`.
- `repo_identity`: Normalized git identity, if `workspace_kind = git`.
- `workspace_state`:
  - `present`
  - `missing` (path does not exist)
  - `unavailable` (permission/mount error)
  - `identity_drift` (repo identity changed)
  - `conflict` (key/path uniqueness violation)

## Data model

`WorkspaceMapping`:
- `workspace_key: String`
- `workspace_path: PathBuf`
- `canonical_path: Option<PathBuf>`
- `workspace_kind: WorkspaceKind`
- `repo_identity: Option<RepoIdentity>`
- `trusted: bool`
- `state: WorkspaceState`
- `last_seen_at: Option<SystemTime>`
- `last_verified_at: Option<SystemTime>`

`RepoIdentity`:
- `primary_remote: Option<String>` (normalized `host/org/repo`)
- `all_remotes: Vec<String>` (normalized)
- `git_common_dir: Option<PathBuf>` (for worktree association)

## Invariants

1. `workspace_key` is globally unique (case-insensitive on case-insensitive filesystems).
2. `workspace_key` is required and non-empty.
3. `canonical_path` is unique among `present` mappings.
4. `missing` mappings persist until explicit user action.
5. Non-workspace opens are blocked when `allow_non_workspace_files = false`.
6. Identity drift never auto-trusts and never silently rewires mappings.

## Canonicalization rules

Path:
1. Expand leading `~`.
2. Require absolute path.
3. If path exists, canonicalize.
4. If missing, store absolute path as configured.
5. On macOS, normalize `/private/...` to `/...` for matching.

Git remote identity:
1. Lowercase host.
2. Strip scheme and auth (`https://`, `ssh://`, `git@`).
3. Convert `:` SSH delimiter to `/`.
4. Strip trailing `.git`.
5. Result: `host/org/repo`.

## Resolution algorithm

### Priority order

1. Exact `workspace_key` match.
2. Remote/upstream match when link provides `remote`.
3. Git ref/worktree fit when link provides branch/tag/commit context.
4. MRU score.
5. User disambiguation dialog.

### Workspace-mode links (`srcuri://<workspace_key>/...`)

1. Resolve by `workspace_key`.
2. If not found: offer map/clone flow.
3. If found but state is `missing`, `unavailable`, or `identity_drift`: show repair dialog before open.
4. If link includes `remote` and mapping is git:
   - if identity matches: continue
   - if mismatch: show conflict dialog; do not silently open
5. Open only after path validation and trust checks.

### Relative/any links

1. Build candidate set from `present` workspaces only.
2. Apply existing path matching.
3. If link includes `workspaceHint`, rank that workspace first.
4. If link includes `remote`, filter to matching git identities.
5. If multiple remain, rank by MRU and show chooser.

### Absolute links

1. Apply path validation.
2. If `allow_non_workspace_files = false`, require containment in configured workspace.
3. Use workspace detection for trust and MRU bookkeeping.

## Clone and map rules

1. Default clone base: configured top-level workspace directory (for example `~/apps`).
2. Default target folder: `<base>/<workspace_key>`.
3. If target exists:
   - empty dir: allow clone
   - same repo identity: reuse with confirmation
   - different identity or non-empty unrelated dir: block auto-clone, require explicit target
4. If `workspace_key` already mapped:
   - same identity: open existing mapping
   - different identity: require new `workspace_key` or explicit rebind
5. For non-git mapping flow, create mapping without `repo_identity`.

## Worktrees

1. Worktrees share one `repo_identity` and `git_common_dir`.
2. Multiple workspace mappings may point to different worktree paths.
3. Ref-aware opens should prefer matching worktree branch when available.
4. If no ref match exists, fall back to MRU within same repo identity.

## Refresh and reconciliation

Use three mechanisms:

1. Startup reconciliation:
   - verify each mapping path and identity
   - recompute `state`, `canonical_path`, `repo_identity`
2. Event-driven refresh:
   - watch mapped paths and top-level workspace roots
   - debounce and re-verify changed mappings
3. Periodic background reconcile:
   - low-frequency sweep for missed events (mount changes, watcher drops)

### State transitions

- `present -> missing`: path removed.
- `present -> unavailable`: permission denied or mount unavailable.
- `present -> identity_drift`: git identity changed.
- `missing -> present`: path reappears; verify identity before clearing warning.
- `missing -> conflict`: replacement path collides with existing key/identity.

### Drift handling rules

Delete:
- keep mapping, mark `missing`, show fix actions.

Rename/move:
- if old mapping missing and a new path appears with same repo identity, suggest rebind.
- do not auto-rebind across identity mismatch.

Recreate with different repo:
- mark `identity_drift`; require explicit user confirmation to rebind.
- clear `trusted` on confirmed rebind.

Non-git becomes git:
- upgrade mapping to `workspace_kind = git` after verification.
- if remote collides with another mapping identity, mark `conflict`.

Git becomes non-git:
- mark `identity_drift` and require confirmation.

## UI changes

### Settings: Workspaces table

Add columns:
- `Key`
- `Path`
- `Type` (`Git` / `Folder`)
- `Remote` (normalized primary remote when git)
- `Status` (`Healthy`, `Missing`, `Drifted`, `Conflict`, `Unavailable`)

Add row actions:
- `Open`
- `Rename key`
- `Rebind path`
- `Mark trusted` / `Untrust`
- `Forget mapping`
- `Resolve conflict`

### Workspace health banner

Show a global warning when any workspace is `missing`, `identity_drift`, `conflict`, or `unavailable`.
Include a `Review workspaces` action.

### Link-time dialogs

1. `Conflict dialog`:
   - explains mismatch (`workspace_key`, `remote`, or path collision)
   - actions: open existing, clone as new key, rebind, cancel

2. `Repair dialog` for `missing`/`unavailable`:
   - actions: locate folder, retry, forget mapping

3. `Ambiguous workspace chooser`:
   - include `workspace_key`, path, remote, status, last seen

### Clone dialog updates

- show target folder and collision check result
- show remote identity used for match
- if key collision exists, suggest deterministic alternatives (`<key>-<owner>`, `<key>-fork`)

## Enterprise policy integration

Policy modes:
- `advisory`: show warnings, allow user override
- `enforced`: block non-compliant mapping/clone/open

Policy payload (minimum):
- allowed `workspace_key -> repo_identity` pairs
- optional allowed clone roots
- optional allowed remote hosts/orgs

Future link field (optional):
- `wid` (stable workspace id) for enterprise canonical mapping.

Security constraints in enterprise mode:
- never auto-map a link to a different policy identity
- never preserve trust through identity drift
- log policy denials with structured reason

## Case matrix (A-M + additional)

### Requested cases

- A: Duplicate folder name outside top-level root is rejected when key collides.
- B: Fork and upstream clones are allowed only with distinct keys; remote disambiguates.
- C: Repo in custom folder (`nexty`) maps to key `nexty` unless user sets explicit key.
- D: Non-git folder under root is valid and resolvable by key/path.
- E: Recreated folder does not steal existing key; stays conflict until user resolves.
- F: Clone link with same key and same remote opens existing mapping.
- G: Clone link with same key and different remote requires explicit disambiguation.
- H: Deleted manual mapping persists as `missing`.
- I: Deleted root-child mapping persists as `missing`.
- K: Worktrees are separate paths under one repo identity; ref/MRU pick target.
- L: Transition to worktrees preserves existing mapping and adds new candidates.
- M: Recreated repo with different remote becomes `identity_drift`, requires confirmation.

### Additional coverage

- Case-only key/path collisions on case-insensitive filesystems.
- Symlink and `..` path dedupe.
- SSH/HTTPS remote equivalence.
- Multiple remotes ambiguity (`origin` vs `upstream`).
- Nested repo/submodule resolution.
- Unicode normalization collisions.
- Mount/unmount or permission loss transitions.

## Acceptance tests

1. Key uniqueness enforcement with and without case differences.
2. Path dedupe via canonicalization and symlink normalization.
3. Remote normalization parity across SSH/HTTPS forms.
4. Missing path persistence across restart.
5. Identity drift detection on remote change.
6. Rebind flow clears `missing` and preserves key uniqueness.
7. Rebind flow resets trust after identity change.
8. Worktree branch-aware selection when multiple worktrees exist.
9. Relative mode disambiguation with remote filter.
10. Non-workspace blocking when setting is disabled.
11. Clone collision handling for existing non-empty target.
12. Enterprise enforced mode blocks policy violations.
