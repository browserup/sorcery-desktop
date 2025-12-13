# Workspace MRU (Most-Recently Used) Heuristics — Implementation Spec v2 (with non‑Git fallback)

**Goal:** Quickly and robustly select the developer workspace (repo folder) that was most recently *active*, **even if Git is unavailable or the folder isn’t a Git repo**. Must be cross‑platform (macOS, Windows, Linux), low‑overhead, and safe to compute **on demand**.

**Audience:** An implementing LLM (and human reviewers) tasked with producing production-quality Rust code and tests.

---

## What’s new in v2

- **Graceful non‑Git fallback:** When Git isn’t installed, libgit2 fails, or a workspace isn’t a Git repo, we fall back to **cheap filesystem signals** (dir & file mtimes) rather than erroring out.
- **Unified scoring:** Git signals (when available) + process‑presence + filesystem mtimes all feed the same “take the max timestamp” rule.
- **Config knobs** for how aggressively to scan in filesystem fallback (depth/limits/allowlist).

---

## Design Rationale (recap)

We want recency that reflects **developer activity**, not just file ages.

1. **Git `HEAD` reflog time** — Records the time of actions like checkout/rebase/commit, so if a user checks out a **months‑old** commit *now*, reflog still yields a **fresh timestamp**. This avoids being “fooled” by old file mtimes.
2. **Running process CWD inside workspace** — A dev server/test watcher means “active *now*,” independent of file changes.
3. **Uncommitted changes** — Use `git status` to get changed paths; `stat()` only those to find a recent file mtime (cheap, bounded).
4. **Non‑Git fallback (filesystem mtimes)** — If Git isn’t available / not a repo, use directory mtimes (update on create/rename/remove in that directory) and a **capped, shallow** file mtime scan to cheaply detect activity.

All signals are optional; compute a **max timestamp** across whatever you can collect. Keep a **2–5s cache** to debounce repeated queries.

---

## Scope & Non‑Goals

- **In scope:** Local workspaces; Git and non‑Git; low‑overhead; on‑demand evaluation; optional light watchers.
- **Out of scope:** Full-tree hashing; network filesystems peculiarities; expensive OS “recents” databases unless user opts in.

---

## Signal Priority (from strongest to weakest)

1. **Process presence (CWD ∈ workspace)** → treat as `now()`. Captures long‑running servers/tests.
2. **Git `HEAD` reflog last entry time** → fresh *action* timestamp even for old checkouts.
3. **Latest mtime from `git status` paths** → recent uncommitted/staged edits without walking the tree.
4. **Filesystem fallback (non‑Git):**
   - **Directory mtimes** for root and a tiny allowlist of top-level dirs (e.g., `src/`, `app/`, `lib/`, `packages/`, `test/`, `spec/`, `include/`, `bin/`, `scripts/`).
   - **Capped shallow scan** of file mtimes at depth ≤ 2 with a hard cap on examined entries (e.g., 400 total).

**Output:** Per-workspace `last_active: SystemTime` + optional justification for observability.

> **Directory mtime behavior:** It updates when entries are created/removed/renamed (not when a child file’s contents are edited). Hence it’s a *weak but cheap* signal; the shallow file scan complements it.

---

## Algorithm (On‑Demand)

For each workspace root `ws`:

1. **Process presence:** If any running process has `cwd` under `ws`, set `from_process = now()`.
2. Try Git (optional, skip on error):
   - `from_reflog = HEAD reflog last time`.
   - `from_uncommitted = max(mtime) of paths from git status`.
3. **Filesystem fallback:** `from_fs_fallback = max(mtime)` across:
   - Root directory + small allowlist of top-level dirs (just their **own** mtimes).
   - **Capped shallow** list of file entries at depth ≤ 2 (limit total examined entries).
4. **Combine:** `last_active = max(from_process, from_reflog, from_uncommitted, from_fs_fallback)`.
5. Cache `(last_active, cached_at)` for 2–5s; recompute on demand if expired.
6. Select MRU across all workspaces by max `last_active` (with optional tie threshold Δ to trigger a small “burst probe” if needed).

---

## Cargo.toml (unchanged, Git optional at runtime)

```toml
[dependencies]
git2    = "0.18"   # or newer; optional at runtime (we handle errors and fall back)
sysinfo = "0.30"   # or newer
anyhow  = "1.0"    # optional, for convenient error handling
# notify = "6"     # optional, only if you add background watchers
```

> You may add a Cargo feature `pure-fs` to compile without `git2` and stub Git functions as `None`.

---

## Rust Types

```rust
use std::time::SystemTime;

#[derive(Default, Debug, Clone)]
pub struct Probe {
    pub from_process: Option<SystemTime>,
    pub from_reflog: Option<SystemTime>,        // Git only
    pub from_uncommitted: Option<SystemTime>,   // Git only
    pub from_fs_fallback: Option<SystemTime>,   // Non-Git or tie-breaker
    pub last_active: Option<SystemTime>,
}

pub type LastActiveCache = std::collections::HashMap<std::path::PathBuf, (SystemTime, std::time::Instant)>;
```

---

## Reference Implementations (Rust)

### A) Process presence (on demand, cheap)

```rust
use sysinfo::{System, SystemExt, ProcessExt};
use std::path::{Path, PathBuf};

pub fn refresh_process_snapshot(sys: &mut sysinfo::System) {
    sys.refresh_processes();
}

pub fn has_running_process_in(root: &Path, sys: &sysinfo::System) -> bool {
    let canon_root = match root.canonicalize() {
        Ok(p) => p,
        Err(_) => root.to_path_buf(),
    };
    for p in sys.processes().values() {
        let cwd = p.cwd();
        if let Ok(canon_cwd) = cwd.canonicalize() {
            if canon_cwd.starts_with(&canon_root) {
                return true;
            }
        }
    }
    false
}
```

---

### B) Git signals (graceful None on failure)

```rust
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use std::path::Path;
use git2::Repository;

// Returns last HEAD reflog time, or None if repo/HEAD reflog unavailable.
pub fn head_reflog_time(repo_path: &Path) -> Option<SystemTime> {
    let repo = Repository::open(repo_path).ok()?;
    let log = repo.reflog("HEAD").ok()?;
    if log.len() == 0 { return None; }
    let entry = log.get(log.len() - 1)?;
    let when = entry.committer().when();
    Some(UNIX_EPOCH + Duration::from_secs(when.seconds() as u64))
}

use git2::{Status, StatusOptions, StatusShow};
use std::{fs, cmp};

// Returns latest file mtime among paths reported changed by git status (tracked+staged+optional untracked).
pub fn latest_uncommitted_mtime(repo_path: &Path) -> Option<SystemTime> {
    let repo = Repository::open(repo_path).ok()?;
    let workdir = repo.workdir()?; // None for bare repos
    let mut opts = StatusOptions::new();
    opts.show(StatusShow::IndexAndWorkdir)
        .include_untracked(true)
        .recurse_untracked_dirs(false)   // keep it cheap
        .exclude_submodules(true)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true)
        .no_refresh(false);              // ensure fresh comparison

    let statuses = repo.statuses(Some(&mut opts)).ok()?;
    let mut latest: Option<SystemTime> = None;

    for e in statuses.iter() {
        let s = e.status();
        let interesting =
            s.intersects(Status::WT_MODIFIED
                       | Status::WT_NEW
                       | Status::WT_DELETED
                       | Status::WT_TYPECHANGE
                       | Status::WT_RENAMED
                       | Status::INDEX_MODIFIED
                       | Status::INDEX_NEW
                       | Status::INDEX_DELETED
                       | Status::INDEX_TYPECHANGE
                       | Status::INDEX_RENAMED);

        if interesting {
            if let Some(rel) = e.path() {
                let p = workdir.join(rel);
                if let Ok(md) = fs::metadata(&p) {
                    if let Ok(t) = md.modified() {
                        latest = Some(match latest { Some(cur) => cmp::max(cur, t), None => t });
                    }
                }
            }
        }
    }
    latest
}
```

---

### C) Filesystem fallback (non‑Git): cheap, shallow, capped

- **Directory mtimes** (very cheap): root + allowlisted top-level dirs.
- **Shallow file scan** (still cheap): up to `MAX_ENTRIES` across depth ≤ 2; track max file mtime.

```rust
use std::{fs, path::{Path, PathBuf}, time::SystemTime, cmp};

fn mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

/// Cheap, shallow, capped filesystem recency signal.
/// - Checks root & top-level dir mtimes.
/// - Scans up to `max_entries` files over depth ≤ 2 (best effort).
pub fn fs_fallback_recent_mtime(
    root: &Path,
    allow_dirs: &[&str],
    max_entries: usize
) -> Option<SystemTime> {
    let mut best: Option<SystemTime> = None

;   // 1) Root + allowlisted top-level directories (dir mtimes)
    if let Some(t) = mtime(root) {
        best = Some(best.map_or(t, |b| b.max(t)));
    }
    for d in allow_dirs {
        if let Some(t) = mtime(&root.join(d)) {
            best = Some(best.map_or(t, |b| b.max(t)));
        }
    }

    // 2) Shallow scan (depth ≤ 2), capped total entries
    let mut seen = 0usize;

    // Helper to visit immediate children of a directory (non-recursive)
    let mut visit_dir = |dir: &Path| {
        if seen >= max_entries { return; }
        if let Ok(rd) = fs::read_dir(dir) {
            for entry in rd.flatten() {
                if seen >= max_entries { break; }
                seen += 1;
                if let Ok(md) = entry.metadata() {
                    if let Ok(t) = md.modified() {
                        best = Some(best.map_or(t, |b| b.max(t)));
                    }
                }
            }
        }
    };

    // Root immediate children
    visit_dir(root);

    // One more level for allowlisted dirs only
    for d in allow_dirs {
        let p = root.join(d);
        visit_dir(&p);
        if seen >= max_entries { break; }
    }

    best
}

/// Sensible defaults for allowlisted top-level directories
pub fn default_allow_dirs() -> [&'static str; 9] {
    ["src","app","lib","packages","test","spec","include","bin","scripts"]
}
```

---

### D) Combine signals: compute `last_active` for a workspace

```rust
use std::path::Path;

pub fn probe_workspace(ws: &Path, sys: &sysinfo::System) -> Probe {
    let mut p = Probe::default();

    // 1) Process presence => now()
    if has_running_process_in(ws, sys) {
        p.from_process = Some(std::time::SystemTime::now());
    }

    // 2) Git signals (graceful None if not a repo or git fails)
    p.from_reflog = head_reflog_time(ws);
    p.from_uncommitted = latest_uncommitted_mtime(ws);

    // 3) Filesystem fallback (always available)
    let allow = default_allow_dirs();
    p.from_fs_fallback = fs_fallback_recent_mtime(ws, &allow, 400);

    // 4) Max
    p.last_active = [p.from_process, p.from_reflog, p.from_uncommitted, p.from_fs_fallback]
        .into_iter()
        .flatten()
        .max();

    p
}
```

---

### E) Pick MRU across workspaces (unchanged)

```rust
use std::path::PathBuf;

pub fn pick_mru(workspaces: &[PathBuf], sys: &sysinfo::System) -> Option<(PathBuf, Probe)> {
    let mut best: Option<(PathBuf, Probe)> = None;
    for ws in workspaces {
        let probe = probe_workspace(ws, sys);
        if let Some(t) = probe.last_active {
            let replace = match &best {
                None => true,
                Some((_, prev)) => prev.last_active.map_or(true, |pt| t > pt),
            };
            if replace {
                best = Some((ws.clone(), probe));
            }
        }
    }
    best
}
```

---

## Configuration & Defaults

- **Cache TTL:** 3s (default). Recompute if stale.
- **FS fallback allowlist:** `["src","app","lib","packages","test","spec","include","bin","scripts"]`.
- **FS fallback cap:** `max_entries = 400`. Adjust between 100–1,000 based on perf.
- **Tie Δ:** 60s default to optionally trigger a small “burst probe” (e.g., scan a few more entries in the tied workspaces only).

All knobs should be exposed via a config struct and/or environment variables.

---

## Testing Plan (additions for non‑Git)

1. **Non‑Git folder:** Create a temp directory with files; touch a file; assert `fs_fallback_recent_mtime` increases.
2. **Directory rename/create:** Add/remove a file in `src/`; assert directory mtime influences fallback.
3. **Large folder guard:** Create >200 files; ensure the cap limits work; verify performance.
4. **Mixed mode:** Some workspaces Git repos, others not — ensure graceful None for Git functions and valid MRU selection.
5. **Cross‑platform:** Confirm `metadata().modified()` semantics match expectations on macOS/Linux/Windows (unit tests can at least exercise code paths; true FS semantics validated in CI matrix).

---

## Observability

- In `--debug` or `--explain` mode, emit which signals contributed and the chosen `last_active`.
- Log if Git is not available (once per workspace) and that FS fallback is in effect.

---

## Security & Privacy

- Use **metadata** only (mtimes, reflog metadata). No file content reads.
- Canonicalize paths before comparing process CWDs.
- Avoid OS‑level “recents” databases unless opted‑in.

---

## Acceptance Criteria (LLM Checklist)

- [ ] Expose a Rust API returning per‑workspace `Probe` and an MRU selection.
- [ ] **Never** panic if Git is missing/not a repo; return `None` for Git signals and rely on FS fallback.
- [ ] Implement FS fallback: dir mtimes + shallow, capped file scan (depth ≤ 2).
- [ ] Keep a small cache with configurable TTL (default 3s).
- [ ] Provide an example CLI (feature‑gated or in `examples/`) that prints MRU and signal breakdown.
- [ ] Include unit tests for Git paths (reflog, uncommitted) and FS fallback behavior, including caps.
- [ ] Cross‑platform compatibility (macOS, Windows, Linux).

---

**End of Spec v2**
