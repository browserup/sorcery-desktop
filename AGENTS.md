## Overview

Sorcery Desktop is a cross-platform hyperlinker for editors/IDEs. The srcuri:// protocol lets developers share editor-independent code references (file + line) that open in each recipient's preferred editor.
Example: `srcuri://myproject/README.md@L75`

The app maintains a long-lived local background service to track active editors and their activity

## Key Directories

- `src-tauri/src/` - Rust backend (Tauri app)
    - `editors/` - Editor manager implementations (one per editor)
    - `protocol_handler/` - srcuri:// URL parsing and handling
    - `settings/` - Configuration management
    - `workspace_mru/` - Workspace tracking
- `public/` - HTML UI components
- `dev/` - Development documentation

## Git

- Commits: imperative, concise (e.g., "Fix path detection")
- Never commit secrets or local paths

## Communication Style

IMPORTANT: Docs, comments, markdown must be concise
- Write in the style of Strunk & White's *The Elements of Style*
- Don't use decorative emojis; meaningful visual info is ok
- Temporary debug emojis are acceptable

## Documentation Maintenance

When adding a feature:
1. Update `dev/features.md` under appropriate section
2. Update `dev/use-cases.md` if new use case (use "As a..." format)

# Code Standards

## Baseline expectations

- Code must **compile**, be `rustfmt`-clean, and **clippy-clean** (`cargo clippy -- -D warnings`)
- Prefer **small modules**, minimal `pub`, and explicit invariants in docs/comments
- Include **unit tests** for non-trivial logic and edge cases
- Keep diffs minimal: **don’t refactor unrelated code**

## Correctness & clarity first

- **Make invalid states unrepresentable.** Prefer enums/newtypes over loosely related booleans or “stringly typed” values
- Keep types small + explicit. Derive common traits when appropriate
    - `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`
- **Document invariants** where correctness depends on them (indexing, concurrency, perf assumptions)

## Ownership, borrowing, and allocations

- **Default to borrowing** in inputs
    - `&T`, `&str`, `&[T]`, iterators, `impl AsRef<Path>`
- **Only allocate when necessary**, and be explicit about it
- Avoid “clone-to-fix-borrow-checker.” Only clone with a clear reason
- Use `Cow<'a, str>` only when you truly need *conditional* ownership; otherwise use `&str` or `String`
- Prefer returning
    - `impl Iterator<Item = T>` when callers can stream results
    - `Vec<T>` when callers need ownership or random access
    - `String` only when you must own; otherwise `&str` (or spans/indices)

## API design defaults

- **Use `Result<T, E>` everywhere errors can occur.**
    - No `unwrap()` / `expect()` outside tests/examples or “impossible by invariant” code (and if so, comment why)
- Prefer **newtypes** for identifiers and units
    - `UserId(u64)`, `Bytes(u64)`, `Millis(u64)`
- Avoid “boolean soup” parameters like `fn foo(a: bool, b: bool, c: bool)`
    - Use enums/options/builders instead
- If more than ~3 parameters or many optional params
    - introduce an `Options` struct or builder

## Error handling (crate-specific rules)

### Domain/library code

- Define domain errors with **thiserror**
    - `#[derive(thiserror::Error, Debug)]`
    - Use structured variants, not raw strings
    - Preserve sources with `#[source]` or `#[from]`
- Keep mapping intentional
    - `NotFound`, `Conflict`, `Validation`, `Unauthorized`, etc

### Application boundaries (handlers/CLI/tasks)

- Use **anyhow** at the boundary
    - `anyhow::Result<T>`
    - Add context with `.context("...")` / `.with_context(|| ...)`
- Don’t drop sources; don’t lose context
- Convert to user-facing/API errors in one place

## Logging & observability

- Use **tracing**, not `println!` or `log`
- Prefer `#[tracing::instrument(skip(...), fields(...))]` on
    - public async entrypoints
    - key operations where you need structured context
- Log policy
    - `debug` for internal state
    - `info` for major lifecycle
    - `warn` for recoverable anomalies
    - `error` only when failing a request/task
- Don’t log secrets

## Traits, generics, and lifetimes (keep it readable)

- Avoid lifetime gymnastics unless needed. Start with straightforward owned/borrowed APIs
- Prefer `impl Trait` returns for simple iterators and closures
- Avoid overly generic signatures; use generics when they reduce duplication *without* harming readability

## Performance rules of thumb

- Prefer slices/iterators over copying collections
- Use `Vec::with_capacity()` when size is known or bounded
- Choose `HashMap` vs `BTreeMap` intentionally (ordering vs speed)
- Don’t introduce `SmallVec`, interning, `Cow`, or unsafe code unless there’s a clear reason (or profiling)
- If performance-sensitive, comment **what** is optimized and **why**

## Concurrency & async (tokio)

- Never hold a `MutexGuard`/`RwLockGuard` across `.await`
- Prefer message passing (channels) over shared mutable state
- Use `tokio::sync` primitives in async code; use `std::sync` only in purely sync code
- Spawned tasks must
    - propagate errors or log with context (don’t silently drop)
    - be cancellable where appropriate
    - be structured/supervised if long-lived
- Avoid I/O while holding locks

## Serde: DTO vs domain model

- Treat the boundary as hostile/messy
- Prefer a boundary “wire” type (DTO) that matches JSON, then convert to domain

### Pattern

- DTO structs: `#[derive(Serialize, Deserialize)]`
- Domain types enforce invariants (newtypes/enums)
- Convert DTO → domain via `TryFrom` / `TryInto` (or `FromStr`) to enforce invariants once
- Avoids pushing complex parsing/validation into custom `Deserialize` implementations

## Code style & structure

- Act as principal architect level developer
- Reuse existing implementations before creating new ones
- No fallback mechanisms that mask errors
- No comments restating what code says
- Remove dead code; don't comment out
- Don't memorialize code removals/changes in comments
- Write intent-revealing code
- Avoid abbreviations in variable names/methods, except where aligning with existing interfaces
- Keep functions short and single-purpose; extract helpers
- Prefer early returns for validation and error paths
- Use exhaustive `match`; avoid `_ =>` when it hides future variants
- Prefer minimal visibility
    - `pub(crate)` by default, `pub` only when necessary
- Add targeted `#[allow(...)]` only with a comment explaining why

## Testing

- Never "fix" tests by changing expectations without valid reason
- New features need tests
- Explain why expectation changes are valid if uncertain
- Unit tests for tricky logic and edge cases
- Use table-driven tests for parsers and boundary checks
- Use `#[tokio::test]` for async code
- Avoid mocking SQLx internals; mock service boundaries instead

# Web Server & SQL (Axum, SQLx)

## Axum

- **Prefer typed state + typed extractors**
    - Use `State<AppState>` for shared dependencies
    - Use `Path<T>`, `Query<T>`, `Json<T>`, `Form<T>` with `#[derive(Deserialize)]`
- **Validate inputs at the boundary.**
    - Convert DTO → domain early (`TryFrom`), return structured errors
    - Keep handlers thin: parse/validate → call service → map to response
- **Define one error type that implements `IntoResponse`.**
    - Map domain errors + external errors into a single `ApiError`
    - Return appropriate HTTP status codes; avoid leaking internal details
    - Add per-request context with `tracing` (request id, user id)
- **Use middleware intentionally.**
    - `TraceLayer` (tower-http) for request spans
    - Timeouts and body limits (`RequestBodyLimitLayer`, `TimeoutLayer`) where relevant
- **Don’t block the runtime.**
    - No CPU-heavy work in handlers; use `spawn_blocking` if needed
- **Route organization**
    - Group routes by feature; keep router construction separate from handlers
    - Prefer `Router::nest("/v1", ...)` and clear modules

## SQLx

- **Use compile-time checked queries.**
    - Prefer `query!` / `query_as!` (or `#[sqlx::query_file]`) for correctness
    - If you must use dynamic SQL, confine it to small, well-reviewed helpers
- **Parameterize everything.**
    - Never string-concatenate user input into SQL
- **Use a connection pool via app state.**
    - `PgPool` / `MySqlPool` / `SqlitePool`. Don’t create pools per request
- **Transactions are explicit and short-lived.**
    - Use `pool.begin().await?`
    - Pass `&mut Transaction<'_, DB>` through repo/service functions
    - Don’t hold a transaction across network calls or long `.await` chains
- **Model DB rows separately from domain types.**
    - Use `#[derive(sqlx::FromRow)]` for row structs, then convert to domain
    - Keep DB nullability explicit (`Option<T>`)
- **Avoid N+1 queries.**
    - Use joins, batching, or `IN (...)` with arrays
- **Use structured errors.**
    - Map `sqlx::Error` to domain/API errors at the boundary; preserve sources
    - Handle “not found” intentionally (`fetch_optional` → 404 mapping)
- **Migrations**
    - Use `sqlx migrate run` / embedded migrations consistently; keep changes reviewed and reproducible

## Axum + SQLx integration patterns

- App `State` holds `Pool` + config + clients; handlers never create pools
- Handlers call services; services call repositories; SQLx stays in the repository layer
