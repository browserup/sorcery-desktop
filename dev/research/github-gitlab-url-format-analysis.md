# GitHub and GitLab URL Format Analysis

**Research Date:** 2025-11-09  
**Purpose:** Document exact URL syntax used by GitHub and GitLab for linking to files with git references, and compare with srcuri's current format.

---

## Executive Summary

Both GitHub and GitLab use **path-based git references** in their URLs (not query parameters). The git reference (branch/tag/commit) is embedded directly in the URL path between the repository name and the file path.

**Current srcuri format uses query parameters:**
```
srcuri://workspace/path/file.rs:42?commit=abc123
srcuri://workspace/path/file.rs:42?branch=main
```

**GitHub and GitLab use path-based refs:**
```
https://github.com/owner/repo/blob/main/path/file.rs#L42
https://gitlab.com/owner/repo/-/blob/main/path/file.rs#L42
```

---

## 1. GitHub URL Format

### Structure

```
https://github.com/{owner}/{repo}/blob/{ref}/{path}#{line}
```

### Components

| Component | Description | Examples |
|-----------|-------------|----------|
| `owner` | Repository owner (user or org) | `microsoft`, `torvalds` |
| `repo` | Repository name | `vscode`, `linux` |
| `blob` | Literal path segment for file viewing | Always `blob` |
| `ref` | Git reference (branch/tag/commit) | `main`, `v1.0.0`, `b212af08a6c` |
| `path` | File path from repository root | `src/main.rs`, `README.md` |
| `line` | Line number or range (fragment) | `#L42`, `#L10-L20` |

### Real Examples

#### Branch Reference (Mutable)
```
https://github.com/microsoft/vscode/blob/main/src/vs/base/common/uri.ts#L42
```
- **Branch:** `main`
- **File:** `src/vs/base/common/uri.ts`
- **Line:** 42
- **Note:** Changes as the branch moves forward

#### Tag Reference (Immutable)
```
https://github.com/rust-lang/rust/blob/1.75.0/library/std/src/lib.rs#L100
```
- **Tag:** `1.75.0`
- **File:** `library/std/src/lib.rs`
- **Line:** 100
- **Note:** Stable, tagged version

#### Commit Reference (Immutable, Recommended)
```
https://github.com/torvalds/linux/blob/b212af08a6cffbb434f3c8a2795a579e092792fd/kernel/sched/core.c#L5000
```
- **Commit:** `b212af08a6cffbb434f3c8a2795a579e092792fd` (full SHA)
- **File:** `kernel/sched/core.c`
- **Line:** 5000
- **Note:** Permanent link, never changes
- **Shortcut:** Press `y` key on GitHub to convert branch URL to commit URL

#### Short Commit SHA
```
https://github.com/torvalds/linux/blob/b212af08/kernel/sched/core.c
```
- **Commit:** `b212af08` (7-character short SHA)
- **Note:** GitHub accepts short SHAs (typically 7+ characters)

### Line Number Syntax

| Format | Example | Description |
|--------|---------|-------------|
| Single line | `#L42` | Highlight line 42 |
| Line range | `#L10-L20` | Highlight lines 10-20 (inclusive) |
| No line | (no fragment) | Just open the file |

**Important:** GitHub uses `#L10-L20` format (with `L` prefix on both numbers).

### Query Parameters

GitHub does use some query parameters, but NOT for git references:

| Parameter | Purpose | Example |
|-----------|---------|---------|
| `?plain=1` | View Markdown without rendering | `README.md?plain=1` |
| `?raw=true` | Redirect to raw file content | `script.sh?raw=true` |
| `?w=1` | Ignore whitespace in diffs | `/commit/abc123?w=1` |

**Git references are NEVER in query parameters** - they are always in the path.

---

## 2. GitLab URL Format

### Structure

```
https://gitlab.com/{owner}/{repo}/-/blob/{ref}/{path}#{line}
```

### Components

| Component | Description | Examples |
|-----------|-------------|----------|
| `owner` | User or group (can be nested) | `gitlab-org`, `group/subgroup` |
| `repo` | Project name | `gitlab-foss`, `my-project` |
| `/-/` | Literal separator (GitLab-specific) | Always `/-/` |
| `blob` | Literal path segment for file viewing | Always `blob` |
| `ref` | Git reference (branch/tag/commit) | `master`, `v1.0.0`, `751547b2ad` |
| `path` | File path from repository root | `app/models/user.rb` |
| `line` | Line number or range (fragment) | `#L42`, `#L10-20` |

### Real Examples

#### Branch Reference
```
https://gitlab.com/gitlab-org/gitlab-foss/-/blob/master/README.md#L10
```
- **Owner:** `gitlab-org`
- **Repo:** `gitlab-foss`
- **Branch:** `master`
- **File:** `README.md`
- **Line:** 10

#### Commit Reference
```
https://gitlab.com/gitlab-org/gitlab-foss/-/blob/751547b2ad6ff6a6c8761ada3fcb14f7f9f9d293/LICENSE#L5
```
- **Commit:** `751547b2ad6ff6a6c8761ada3fcb14f7f9f9d293` (full SHA)
- **File:** `LICENSE`
- **Line:** 5

#### Subdirectory File
```
https://gitlab.com/gitlab-org/gitlab-foss/-/blob/master/app/models/user.rb#L125
```
- **Branch:** `master`
- **File:** `app/models/user.rb`
- **Line:** 125

#### Tree (Directory) View
```
https://gitlab.com/gitlab-org/gitlab-foss/-/tree/master/app
```
- **Type:** `tree` (directory, not `blob`)
- **Branch:** `master`
- **Directory:** `app/`

### Line Number Syntax

| Format | Example | Description |
|--------|---------|-------------|
| Single line | `#L42` | Highlight line 42 |
| Line range | `#L10-20` | Highlight lines 10-20 (NO `L` on end!) |
| No line | (no fragment) | Just open the file |

**Important:** GitLab uses `#L10-20` format (NO `L` prefix on the end number). This differs from GitHub's `#L10-L20`.

### Key Difference: `/-/` Separator

GitLab uses a unique `/-/` separator before action paths:
- Files: `/-/blob/`
- Directories: `/-/tree/`
- Commits: `/-/commit/`
- Merge Requests: `/-/merge_requests/`

This is a GitLab-specific convention not used by GitHub.

---

## 3. Side-by-Side Comparison

### URL Structure Comparison

| Platform | URL Pattern | Example |
|----------|-------------|---------|
| **GitHub** | `github.com/{owner}/{repo}/blob/{ref}/{path}#L{line}` | `github.com/rust-lang/rust/blob/master/src/main.rs#L100` |
| **GitLab** | `gitlab.com/{owner}/{repo}/-/blob/{ref}/{path}#L{line}` | `gitlab.com/gitlab-org/gitlab/-/blob/master/app/main.rb#L100` |
| **srcuri** | `srcuri://{workspace}/{path}:{line}?commit={ref}` | `srcuri://rust/src/main.rs:100?commit=abc123` |

### Git Reference Placement

| Platform | Ref Location | Branch Example | Commit Example |
|----------|--------------|----------------|----------------|
| **GitHub** | In path | `.../blob/main/...` | `.../blob/abc123/...` |
| **GitLab** | In path | `.../-/blob/master/...` | `.../-/blob/abc123/...` |
| **srcuri** | Query param | `...?branch=main` | `...?commit=abc123` |

### Line Number Syntax

| Platform | Single Line | Line Range | Notes |
|----------|-------------|------------|-------|
| **GitHub** | `#L42` | `#L10-L20` | Both numbers have `L` prefix |
| **GitLab** | `#L42` | `#L10-20` | Only start has `L` prefix |
| **srcuri** | `:42` | `:10` | Colon-based, no range support currently |

---

## 4. Query Parameters Analysis

### GitHub Query Parameters

GitHub does NOT use query parameters for git references. Query parameters serve other purposes:

| Parameter | Purpose | Example URL |
|-----------|---------|-------------|
| `?plain=1` | View Markdown as plain text with line numbers | `README.md?plain=1#L14` |
| `?raw=true` | Redirect to raw file content | `script.sh?raw=true` |
| `?w=1` | Ignore whitespace in diffs | `/commit/abc?w=1` |
| `?ts=4` | Set tab stop width in diffs | `/commit/abc?ts=4` |

**For file references, the git ref is ALWAYS in the path.**

### GitLab Query Parameters

GitLab also does NOT use query parameters for git references. The API uses query params:

| Parameter | Purpose | Context |
|-----------|---------|---------|
| `?ref=master` | Specify branch in API | API calls only, not web URLs |
| `?committed_after` | Filter commits by date | Git history view |
| `?committed_before` | Filter commits by date | Git history view |

**For web URLs, the git ref is ALWAYS in the path.**

### srcuri Current Approach

srcuri currently uses query parameters for git references:

```
srcuri://workspace/file.rs:42?commit=abc123
srcuri://workspace/file.rs:42?branch=main
srcuri://workspace/file.rs:42?tag=v1.0.0
```

**This differs from both GitHub and GitLab.**

---

## 5. Comparison with srcuri Format

### Current srcuri URL Format

From `URL-FORMATS.md`:

```
srcuri://<workspace>/<path>:<line>:<column>
```

With git references:
```
srcuri://srcuri/README.md:1?commit=abc123def
srcuri://srcuri/src/main.rs:50?sha=abc123def
srcuri://myproject/src/app.js:10?branch=main
srcuri://myproject/file.txt:10?tag=v1.0.0
```

### Key Differences

| Aspect | GitHub/GitLab | srcuri |
|--------|---------------|-------|
| **Git ref location** | Path-based (`/blob/{ref}/`) | Query parameter (`?commit=`, `?branch=`) |
| **Line syntax** | Fragment (`#L42`) | Colon (`:42`) |
| **Column support** | None | Yes (`:42:10`) |
| **Workspace concept** | None (always owner/repo) | Yes (maps to local paths) |
| **Absolute paths** | Not supported | Yes (`srcuri:///abs/path:42`) |

### Advantages of srcuri's Query Parameter Approach

1. **Separation of Concerns**
   - File path is independent of git ref
   - `srcuri://workspace/file.rs:42` works without git context
   - Adding `?commit=abc` is optional, not structural

2. **Backward Compatibility**
   - Links without git refs still work
   - Can add git ref parameter later without breaking existing links

3. **Flexibility**
   - Clear distinction between `?commit=`, `?branch=`, `?tag=`
   - GitHub/GitLab conflate all refs into the same path position
   - With srcuri you can explicitly specify the ref type

4. **Column Support**
   - `:line:column` syntax is unambiguous
   - GitHub/GitLab don't support column numbers at all

5. **Workspace Abstraction**
   - `srcuri://myproject/file.rs` is portable across machines
   - GitHub URLs are tied to specific hosting platforms
   - Different developers can have different local paths

6. **Absolute Path Support**
   - `srcuri:///Users/me/file.txt:42` for local-only files
   - GitHub/GitLab can't represent files outside repositories

### Disadvantages of srcuri's Query Parameter Approach

1. **Unfamiliar Convention**
   - Developers expect git refs in paths (GitHub/GitLab pattern)
   - Query params feel less "standard" for version control

2. **Visual Noise**
   - `?commit=abc123` is longer than `/abc123/`
   - Breaks the visual flow of the path

3. **URL Complexity**
   - Query strings are often associated with temporary/transient state
   - Path-based refs feel more "permanent"

4. **Conversion Complexity**
   - Converting GitHub/GitLab URLs to srcuri requires restructuring
   - Can't do simple string replacement

### Advantages of GitHub/GitLab's Path-Based Approach

1. **Industry Standard**
   - Every major code host uses this pattern
   - Familiar to all developers

2. **Visual Clarity**
   - The ref is clearly part of the "address" of the file
   - `/blob/main/file.rs` reads naturally

3. **Brevity**
   - Shorter URLs: `/main/` vs `?branch=main`
   - Easier to type and share

4. **Semantic Meaning**
   - The ref is structurally part of the resource identifier
   - "The file at this commit" vs "the file, with this commit parameter"

### Disadvantages of GitHub/GitLab's Path-Based Approach

1. **No Optional Refs**
   - Must always include a ref in the path
   - Can't have "workspace-aware" URLs without a ref

2. **No Ref Type Distinction**
   - `/blob/abc123/` - is this a branch named "abc123" or a commit?
   - Must guess based on format (hex = commit, etc.)

3. **No Column Support**
   - Fragment syntax only supports lines
   - No standard way to specify column

4. **Platform-Specific**
   - Always tied to owner/repo structure
   - No concept of local workspace mapping

---

## 6. Real-World URL Examples

### GitHub Examples

```bash
# Python project, main branch, specific line
https://github.com/python/cpython/blob/main/Lib/os.py#L100

# Rust project, tagged release, line range
https://github.com/rust-lang/rust/blob/1.75.0/library/std/src/lib.rs#L50-L75

# Linux kernel, specific commit, deep path
https://github.com/torvalds/linux/blob/b212af08a6c/kernel/sched/core.c#L5000

# React project, feature branch
https://github.com/facebook/react/blob/feature-hooks/packages/react/src/React.js#L20

# Short commit SHA
https://github.com/nodejs/node/blob/a1b2c3d/lib/internal/process/promises.js#L10
```

### GitLab Examples

```bash
# GitLab project, master branch
https://gitlab.com/gitlab-org/gitlab-foss/-/blob/master/README.md#L10-20

# Nested group, specific commit
https://gitlab.com/gitlab-org/security/gitlab-foss/-/blob/751547b2ad/LICENSE#L5

# Self-hosted GitLab (hypothetical)
https://gitlab.mycompany.com/backend/api-server/-/blob/develop/src/main.rs#L100

# Directory view (tree, not blob)
https://gitlab.com/gitlab-org/gitlab-foss/-/tree/master/app/models
```

### srcuri Examples (Current)

```bash
# Simple workspace path
srcuri://myproject/src/main.rs:100

# With commit reference
srcuri://myproject/src/main.rs:100?commit=abc123

# With branch reference
srcuri://myproject/src/main.rs:100?branch=feature-auth

# With tag reference
srcuri://myproject/CHANGELOG.md:1?tag=v1.0.0

# Absolute path (no git context)
srcuri:///Users/me/temp/script.py:42

# With column
srcuri://myproject/src/main.rs:100:15
```

---

## 7. Converting Between Formats

### GitHub → srcuri Conversion

**Input:**
```
https://github.com/rust-lang/rust/blob/b212af08/library/std/src/lib.rs#L100
```

**Extraction:**
- Owner: `rust-lang`
- Repo: `rust`
- Ref: `b212af08`
- Path: `library/std/src/lib.rs`
- Line: `100`

**Mapping Strategy:**
1. Match git remote: `git@github.com:rust-lang/rust.git`
2. Find local workspace: `rust` (configured in srcuri settings)

**Output:**
```
srcuri://rust/library/std/src/lib.rs:100?commit=b212af08
```

### GitLab → srcuri Conversion

**Input:**
```
https://gitlab.com/gitlab-org/gitlab-foss/-/blob/master/README.md#L10-20
```

**Extraction:**
- Owner: `gitlab-org`
- Repo: `gitlab-foss`
- Ref: `master`
- Path: `README.md`
- Line: `10` (start of range)

**Mapping Strategy:**
1. Match git remote: `git@gitlab.com:gitlab-org/gitlab-foss.git`
2. Find local workspace: `gitlab-foss`

**Output:**
```
srcuri://gitlab-foss/README.md:10?branch=master
```

### srcuri → GitHub Conversion (Hypothetical)

**Input:**
```
srcuri://rust/library/std/src/lib.rs:100?commit=b212af08
```

**Workspace Mapping:**
- Workspace `rust` → git remote `git@github.com:rust-lang/rust.git`
- Extract owner: `rust-lang`
- Extract repo: `rust`

**Output:**
```
https://github.com/rust-lang/rust/blob/b212af08/library/std/src/lib.rs#L100
```

---

## 8. Analysis & Recommendations

### Should srcuri Align with GitHub/GitLab?

#### Arguments FOR Alignment (Path-Based Refs)

1. **Developer Familiarity**
   - Everyone knows GitHub/GitLab URLs
   - Mental model already established

2. **Easier Conversion**
   - Converting GitHub URLs to srcuri would be more intuitive
   - Less restructuring needed

3. **Industry Standard**
   - Matches every major code hosting platform
   - Git refs "feel like" part of the resource path

4. **Potential Format:**
   ```
   srcuri://workspace@ref/path/file.rs:42
   srcuri://rust@main/src/lib.rs:100
   srcuri://rust@b212af08/src/lib.rs:100
   ```
   - Uses `@` to separate workspace from ref (npm package style)
   - Path-based like GitHub/GitLab
   - Still supports workspace concept

#### Arguments AGAINST Alignment (Keep Query Params)

1. **Workspace Abstraction is Different**
   - srcuri isn't a web hosting platform
   - Workspaces are local concepts, not URLs
   - Git refs are "overlay" information, not core identity

2. **Optional Refs are Valuable**
   - `srcuri://myproject/file.rs:42` should work without git context
   - Adding `?commit=abc` when needed is cleaner
   - GitHub/GitLab always require a ref in the path

3. **Column Support Requires Different Syntax**
   - `:line:column` is already different from `#L` syntax
   - Query params feel consistent with optional features

4. **Backward Compatibility**
   - Existing srcuri URLs work fine
   - No breaking changes needed
   - Query params can be added/removed without restructuring

5. **Clear Semantics**
   - `?commit=abc` explicitly says "this is a commit"
   - `?branch=main` explicitly says "this is a branch"
   - Path-based refs conflate all ref types

### Hybrid Approach Option

Support BOTH formats:

```bash
# Path-based (GitHub-style, for familiarity)
srcuri://workspace@main/file.rs:42
srcuri://workspace@abc123/file.rs:42

# Query-based (current, for flexibility)
srcuri://workspace/file.rs:42?branch=main
srcuri://workspace/file.rs:42?commit=abc123
```

**Pros:**
- Best of both worlds
- Easier onboarding (path-based familiar)
- Power users get query param flexibility

**Cons:**
- Two ways to do the same thing
- More complex parser
- Potential confusion

### Recommendation: **Keep Query Parameters**

**Rationale:**

1. **srcuri is fundamentally different from GitHub/GitLab**
   - It's a local file launcher, not a web hosting platform
   - Workspace abstraction is the core value, not git hosting
   - Git refs are optional metadata, not required structure

2. **Query params match srcuri's design**
   - `srcuri://workspace/file.rs:42` is the "base" link
   - `?commit=abc` is "open this file, but at this commit"
   - Semantic: the file is the resource, commit is a parameter

3. **Flexibility for future features**
   - `?commit=abc&view=blame` 
   - `?branch=main&diff=feature`
   - Query params naturally extend

4. **Workspace portability**
   - `srcuri://myproject/file.rs:42` works on any machine with `myproject` workspace
   - GitHub URLs are tied to `github.com/owner/repo` structure
   - Local workspace mapping is srcuri's killer feature

5. **Column support already diverges**
   - `:line:column` vs `#L` means syntax is already different
   - No point aligning on git refs when line syntax already differs

### Alternative: Document Conversion Clearly

Instead of changing srcuri format, provide excellent conversion tools:

1. **Browser extension:**
   - Adds "Copy srcuri link" button to GitHub/GitLab
   - Converts GitHub URL → srcuri URL automatically

2. **CLI tool:**
   ```bash
   srcuri convert "https://github.com/owner/repo/blob/main/file.rs#L42"
   # Output: srcuri://repo/file.rs:42?branch=main
   ```

3. **Documentation:**
   - Clear examples showing GitHub → srcuri conversion
   - Explain the semantic difference
   - Show benefits of query param approach

---

## 9. Line Range Support

### GitHub Line Ranges

```
https://github.com/owner/repo/blob/main/file.rs#L10-L20
```
- Start: Line 10
- End: Line 20
- Format: Both numbers have `L` prefix

### GitLab Line Ranges

```
https://gitlab.com/owner/repo/-/blob/master/file.rb#L10-20
```
- Start: Line 10
- End: Line 20
- Format: Only start has `L` prefix

### srcuri Line Ranges

Currently NOT supported. Potential formats:

**Option 1: Query parameter**
```
srcuri://workspace/file.rs:10?end=20
```

**Option 2: Colon-based range**
```
srcuri://workspace/file.rs:10-20
```
- Ambiguity: Is `:10-20` a line range, or line 10 column 20 with typo?

**Option 3: Separate syntax**
```
srcuri://workspace/file.rs:10:20       # line 10, column 20
srcuri://workspace/file.rs:10-20       # lines 10-20
```
- Parser must distinguish `-` (range) from `:` (column)

**Recommendation:**
```
srcuri://workspace/file.rs:10?end=20&highlight=true
```
- Clear and unambiguous
- Doesn't conflict with `:line:column` syntax
- Extensible: `?end=20&highlight=false` to just scroll, not highlight

---

## 10. Summary Table

| Feature | GitHub | GitLab | srcuri (Current) | Recommendation |
|---------|--------|--------|-----------------|----------------|
| **Git ref location** | Path: `/blob/{ref}/` | Path: `/-/blob/{ref}/` | Query: `?commit=` | Keep query params |
| **Ref required?** | Yes | Yes | No | Keep optional |
| **Line syntax** | `#L42` | `#L42` | `:42` | Keep colon syntax |
| **Line range** | `#L10-L20` | `#L10-20` | Not supported | Add `?end=20` |
| **Column support** | No | No | `:42:10` | Keep as-is |
| **Ref type clarity** | Ambiguous | Ambiguous | Explicit (`?branch=`) | Keep explicit |
| **Workspace concept** | No (owner/repo) | No (owner/repo) | Yes | Keep as-is |
| **Absolute paths** | No | No | Yes (`///path`) | Keep as-is |

---

## 11. Conclusion

GitHub and GitLab use **path-based git references** embedded in URLs between the repository and file path. They use **fragment-based line numbers** (`#L42`) and do NOT use query parameters for git references.

srcuri's current approach with **query parameters** is deliberately different because:

1. **Workspaces are not web hosts** - they're local directory mappings
2. **Git refs are optional metadata** - not required for every link
3. **Query params provide semantic clarity** - explicit `?commit=` vs `?branch=`
4. **Flexibility for future features** - easy to extend with more params
5. **Portability across machines** - workspace abstraction is the core value

**Recommendation:** Keep srcuri's query parameter approach. Document conversion from GitHub/GitLab URLs clearly. Provide browser extensions and CLI tools to make conversion seamless.

The difference in syntax reflects a fundamental difference in purpose: GitHub/GitLab are web hosts with version control, while srcuri is a local file launcher with workspace awareness.
