# Workspace Identity Implementation Plan

## Goal

Implement `dev/workspace-identity-resolution-spec.md` in small, testable phases, without breaking current link behavior.

## Current status (2026-02-06)

- Completed:
  - `WI-001` model fields and persistence defaults.
  - `WI-002` path and remote identity normalization helpers.
  - `WI-003` save-time uniqueness validation with typed errors.
  - `WI-004` startup reconciliation.
  - `WI-005` filesystem watch + debounce reconcile + health events.
  - `WI-006` periodic safety reconcile.
  - `WI-007` key/remote-first resolver behavior.
  - `WI-008` unhealthy mapping opens are blocked before dispatch.
  - `WI-009` clone preflight guardrails (same-key/same-remote and target collision checks).
  - `WI-010` workspace table status/key/type/remote surfaced in settings.
  - `WI-011` workspace health banner and live update wiring.
  - `WI-012` clone dialog conflict-aware validation messaging.
  - `WI-013` dedicated repair/conflict dialogs and actions.
  - `WI-014` persisted worktree metadata (`git_common_dir`, `current_branch`, `head_commit`) and repo-group association helpers.
  - `WI-015` branch-aware worktree selection for revision links.
  - `WI-016` optional enterprise policy schema, loading, and validation.
  - `WI-017` enforced policy checks in resolver/clone paths with explicit UI reasons.

## Delivery strategy

1. Land backend model and reconciliation first.
2. Add resolver logic behind deterministic rules.
3. Add UI repair/conflict flows.
4. Add worktree-specific selection.
5. Add enterprise policy hooks last.

Each phase ships with tests and can be rolled back independently.

## Phase breakdown

### Phase 1: Model and persistence foundation

#### WI-001: Extend workspace model with identity and health state
- Type: Rust backend
- Files:
  - `src-tauri/src/settings/models.rs`
  - `src-tauri/src/settings/mod.rs`
- Changes:
  - Add `workspace_key`, `workspace_kind`, `workspace_state`, `repo_identity`, `last_verified_at`.
  - Use `workspace_key` as the single persisted identity field.
  - Keep serde defaults for optional health/identity fields.
- Tests:
  - Deserialize old YAML without new fields.
  - Serialize new fields and preserve old required fields.

#### WI-002: Add canonicalization + remote identity utilities
- Type: Rust backend
- Files:
  - `src-tauri/src/settings/manager.rs`
  - `src-tauri/src/protocol_handler/git.rs`
  - new: `src-tauri/src/settings/identity.rs`
- Changes:
  - Move path normalization to shared helper with macOS `/private` handling.
  - Add `normalize_remote_identity()` (SSH/HTTPS equivalence).
  - Add `workspace_key` normalization helper (case and unicode handling).
- Tests:
  - Remote normalization table tests.
  - Path canonicalization and symlink dedupe tests.

#### WI-003: Enforce uniqueness invariants on save
- Type: Rust backend
- Files:
  - `src-tauri/src/settings/manager.rs`
- Changes:
  - Reject empty `workspace_key`.
  - Reject duplicate `workspace_key`.
  - Reject duplicate `canonical_path` for present mappings.
  - Return structured validation errors (not generic strings).
- Tests:
  - Empty key rejected on load/save.
  - Duplicate key rejected.
  - Duplicate canonical path rejected.
  - Missing path can persist with unique key.

### Phase 2: Reconciliation and drift detection

#### WI-004: Startup reconciliation pass
- Type: Rust backend
- Files:
  - `src-tauri/src/main.rs`
  - `src-tauri/src/settings/manager.rs`
  - new: `src-tauri/src/settings/reconcile.rs`
- Changes:
  - On startup, verify each mapping path and recompute state.
  - Detect `missing`, `unavailable`, and `identity_drift`.
  - Persist state updates only when changed.
- Tests:
  - Deleted path -> `missing`.
  - Permission/mount error -> `unavailable`.
  - Remote changed -> `identity_drift`.

#### WI-005: Event-driven refresh with debounce
- Type: Rust backend
- Files:
  - new: `src-tauri/src/settings/watch.rs`
  - `src-tauri/src/main.rs`
  - `src-tauri/src/commands/mod.rs`
- Changes:
  - Watch configured workspace paths and default workspace root.
  - Debounce bursts and trigger reconcile for affected mappings.
  - Emit UI event (`workspace-health-updated`) when states change.
- Tests:
  - Rename/delete events transition states correctly.
  - Debounce avoids repeated writes.

#### WI-006: Periodic safety reconcile
- Type: Rust backend
- Files:
  - `src-tauri/src/main.rs`
  - `src-tauri/src/settings/reconcile.rs`
- Changes:
  - Add low-frequency sweep to recover from dropped watcher events.
  - Skip active writes while save lock is held.
- Tests:
  - Missed event recovered by periodic sweep.

### Phase 3: Resolver and clone behavior

#### WI-007: Resolver candidate ranking by key, remote, ref, MRU
- Type: Rust backend
- Files:
  - `src-tauri/src/protocol_handler/matcher.rs`
  - `src-tauri/src/protocol_handler/mod.rs`
  - `src-tauri/src/workspace_mru/mod.rs`
- Changes:
  - Resolve exact `workspace_key` first.
  - Apply remote filter when `?remote` exists.
  - Prefer ref/worktree fit when revision context exists.
  - Keep chooser fallback when ambiguous.
- Tests:
  - Same-name fork/upstream disambiguated by remote.
  - Ambiguous without remote falls to chooser.

#### WI-008: Block unhealthy mapping opens before dispatch
- Type: Rust backend
- Files:
  - `src-tauri/src/protocol_handler/mod.rs`
  - `src-tauri/src/dispatcher/mod.rs`
  - `src-tauri/src/dialog_state.rs`
- Changes:
  - If mapping state is `missing`, `unavailable`, `identity_drift`, or `conflict`, return repair/conflict dialog result.
  - Keep `allow_non_workspace_files=false` strict behavior.
- Tests:
  - Unhealthy mapping cannot open directly.
  - Non-workspace path still blocked when disabled.

#### WI-009: Clone collision guardrails
- Type: Rust backend
- Files:
  - `src-tauri/src/dialog_state.rs`
  - `src-tauri/src/protocol_handler/mod.rs`
  - `src-tauri/src/protocol_handler/git.rs`
- Changes:
  - Preflight clone target:
    - empty dir => allowed
    - same identity => reuse confirmation
    - different identity/non-empty unrelated => require explicit destination
  - If key exists with same identity, open existing mapping.
- Tests:
  - Collision matrix for empty/non-empty/same-identity/different-identity.

### Phase 4: Settings and dialog UI

#### WI-010: Workspace table health columns and actions
- Type: UI + commands
- Files:
  - `public/settings.html`
  - `src-tauri/src/commands/mod.rs`
- Changes:
  - Add fields: key, type, remote, status.
  - Add actions: rename key, rebind path, resolve conflict, forget mapping.
  - Add inline status badges (`Healthy`, `Missing`, `Drifted`, `Conflict`, `Unavailable`).
- Tests:
  - Manual UI verification checklist + command unit tests.

#### WI-011: Workspace health banner + refresh actions
- Type: UI + commands
- Files:
  - `public/settings.html`
  - `src-tauri/src/commands/mod.rs`
- Changes:
  - Add global warning banner when unhealthy mappings exist.
  - Add `Retry reconcile` and `Review workspaces` actions.
  - Subscribe to `workspace-health-updated` events.
- Tests:
  - Banner appears and clears after repair.

#### WI-012: Clone dialog conflict-aware UX
- Type: UI + commands
- Files:
  - `public/clone-dialog.html`
  - `src-tauri/src/dialog_state.rs`
  - `src-tauri/src/commands/mod.rs`
- Changes:
  - Show normalized remote identity.
  - Show preflight result for selected target.
  - Suggest deterministic alternate folder names on collision.
- Tests:
  - Clone dialog shows block state for invalid target.

#### WI-013: New repair/conflict dialogs
- Type: UI + protocol handler
- Files:
  - new: `public/workspace-repair.html`
  - new: `public/workspace-conflict.html`
  - `src-tauri/src/dialog_state.rs`
  - `src-tauri/src/main.rs`
  - `src-tauri/src/protocol_handler/mod.rs`
- Changes:
  - Add dialog payloads and routes.
  - Actions: locate folder, rebind, open existing mapping, clone as new key, forget mapping.
- Tests:
  - Dialog payload round-trip and action command tests.

### Phase 5: Worktree-aware behavior

#### WI-014: Persist git common dir and worktree metadata
- Type: Rust backend
- Files:
  - `src-tauri/src/settings/models.rs`
  - `src-tauri/src/protocol_handler/git.rs`
  - `src-tauri/src/settings/reconcile.rs`
- Changes:
  - Store `git_common_dir` and branch hints.
  - Associate multiple paths to one repo identity.
- Tests:
  - Worktree metadata extracted from standard git worktree layout.

#### WI-015: Branch-aware worktree selection
- Type: Rust backend
- Files:
  - `src-tauri/src/protocol_handler/mod.rs`
  - `src-tauri/src/protocol_handler/matcher.rs`
- Changes:
  - When link includes branch/tag, prefer matching worktree.
  - If no exact branch match, use MRU in same repo identity group.
- Tests:
  - Two-worktree branch selection scenarios.

### Phase 6: Enterprise policy hooks

#### WI-016: Add optional policy schema and validator
- Type: Rust backend
- Files:
  - new: `src-tauri/src/settings/policy.rs`
  - `src-tauri/src/settings/models.rs`
  - `src-tauri/src/settings/manager.rs`
- Changes:
  - Parse optional policy file with advisory/enforced mode.
  - Validate key and remote identity against policy.
- Tests:
  - Advisory warns, enforced blocks.

#### WI-017: Enforce policy in resolver and clone paths
- Type: Rust backend + UI
- Files:
  - `src-tauri/src/protocol_handler/mod.rs`
  - `src-tauri/src/protocol_handler/git.rs`
  - `public/clone-dialog.html`
  - `public/workspace-conflict.html`
- Changes:
  - Block non-compliant mapping/clone/open in enforced mode.
  - Show explicit policy reason in UI.
- Tests:
  - Resolver and clone policy denial coverage.

## Cross-cutting test plan

1. Unit tests for canonicalization and identity normalization.
2. Unit tests for key/path uniqueness and state transitions.
3. Table-driven resolver tests for A-M + extended cases.
4. Integration tests for dialog command payloads.
5. Manual UI checklist for settings/clone/repair flows.

## Rollout

No feature flag is used. Changes ship directly by phase with test coverage.

## Risks and mitigations

1. Risk: false identity drift due to remote parsing quirks.
- Mitigation: broad normalization tests and fallback to user confirmation.

2. Risk: filesystem watcher differences across OSes.
- Mitigation: periodic reconcile safety net and startup reconcile.

3. Risk: UI complexity in settings page.
- Mitigation: ship status badges first, advanced actions behind overflow menu.

4. Risk: trust confusion after rebind.
- Mitigation: always clear trust on identity change and show explicit message.

## Recommended execution order

1. WI-001, WI-002, WI-003
2. WI-004, WI-005, WI-006
3. WI-007, WI-008, WI-009
4. WI-010, WI-011, WI-012, WI-013
5. WI-014, WI-015
6. WI-016, WI-017
