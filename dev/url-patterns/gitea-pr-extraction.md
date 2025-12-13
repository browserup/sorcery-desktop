# Gitea Pull Request Diff - File Path Extraction Guide

This document explains how to extract file paths and line numbers from Gitea pull request diff URLs.

## URL Format

```
http://demo.gitea.com/adamm2/foo/pulls/2/files#diff-{hash}[L|R]{line}
```

Components:
- **owner**: `adamm2`
- **repo**: `foo`
- **pr_number**: `2`
- **file_hash**: `8ec9a00bfd09b3190ac6b22251dbb1aa95a0579d`
- **line_prefix**: `L` (old line) or `R` (new line)
- **line_number**: The line number in the diff

## HTML Structure

### File Diff Box

Each file in the diff has a container with a unique ID:

```html
<div class="diff-file-box file-content tab-size-4"
     id="diff-8ec9a00bfd09b3190ac6b22251dbb1aa95a0579d"
     data-old-filename="README.md"
     data-new-filename="README.md">
```

**Key attributes:**
- `id`: `diff-{hash}` - matches the hash in the URL fragment
- `data-old-filename`: Original filename (before changes)
- `data-new-filename`: New filename (after changes, may be different if renamed)

### File Header

Inside the diff box, the header contains the file name:

```html
<h4 class="diff-file-header">
  <span class="file">
    <a class="muted file-link" title="README.md" href="#diff-8ec9a00bfd09b3190ac6b22251dbb1aa95a0579d">
      README.md
    </a>
  </span>
</h4>
```

**Selector:** `.diff-file-header .file-link`

### Table with Line Numbers

The diff content is in a table:

```html
<table class="chroma"
       data-new-comment-url="/adamm2/foo/pulls/2/files/reviews/new_comment"
       data-path="README.md">
```

**Key attributes:**
- `data-path`: The file path (most reliable source!)
- `data-new-comment-url`: Contains owner, repo, and PR number

### Line Number Cells

Each row has line numbers for old and new versions:

```html
<!-- Deleted line (old version) -->
<tr class="del-code nl-1 ol-1" data-line-type="del">
  <td class="lines-num lines-num-old" data-line-num="1">
    <span rel="diff-8ec9a00bfd09b3190ac6b22251dbb1aa95a0579dL1"></span>
  </td>
  <td class="lines-num lines-num-new" data-line-num="">
    <span rel=""></span>
  </td>
  ...
</tr>

<!-- Added line (new version) -->
<tr class="add-code nl-2 ol-2" data-line-type="add">
  <td class="lines-num lines-num-old" data-line-num="">
    <span rel=""></span>
  </td>
  <td class="lines-num lines-num-new" data-line-num="1">
    <span rel="diff-8ec9a00bfd09b3190ac6b22251dbb1aa95a0579dR1"></span>
  </td>
  ...
</tr>

<!-- Unchanged line (both versions) -->
<tr class="same-code nl-3 ol-3" data-line-type="same">
  <td class="lines-num lines-num-old" data-line-num="2">
    <span rel="diff-8ec9a00bfd09b3190ac6b22251dbb1aa95a0579dL2"></span>
  </td>
  <td class="lines-num lines-num-new" data-line-num="2">
    <span rel="diff-8ec9a00bfd09b3190ac6b22251dbb1aa95a0579dR2"></span>
  </td>
  ...
</tr>
```

**Key attributes:**
- `data-line-type`: `del`, `add`, `same`, or `tag` (section header)
- `data-line-num`: The actual line number (empty for deleted/added lines on one side)
- `rel`: Format is `diff-{hash}[L|R]{line}` - exactly matches URL fragment format

**Line prefixes:**
- `L` = Left side (old version, "before" changes)
- `R` = Right side (new version, "after" changes)

## Extraction Algorithm

### Step 1: Parse the URL Fragment

```javascript
// Example URL: http://demo.gitea.com/adamm2/foo/pulls/2/files#diff-8ec9a00bR3
const hash = window.location.hash; // "#diff-8ec9a00bR3"
const match = hash.match(/#diff-([a-f0-9]+)([LR])?(\d+)?$/);

if (match) {
  const fileHash = match[1];  // "8ec9a00b..."
  const side = match[2];      // "R" (or "L")
  const lineNum = match[3];   // "3"
}
```

### Step 2: Find the Diff Box

```javascript
const diffBox = document.getElementById(`diff-${fileHash}`);
```

### Step 3: Extract File Path

**Method 1: From table data-path (most reliable)**
```javascript
const table = diffBox.querySelector('table.chroma');
const filePath = table.getAttribute('data-path'); // "README.md"
```

**Method 2: From data attributes on diff-file-box**
```javascript
const filePath = diffBox.getAttribute('data-new-filename'); // "README.md"
// Or for deleted files:
const filePath = diffBox.getAttribute('data-old-filename');
```

**Method 3: From file-link text**
```javascript
const fileLink = diffBox.querySelector('.diff-file-header .file-link');
const filePath = fileLink.textContent.trim(); // "README.md"
```

### Step 4: Determine Line Number

If the URL has a line number suffix (`L3` or `R3`):

```javascript
if (lineNum) {
  if (side === 'R') {
    // Right side (new version) - use this for opening in editor
    const lineNumber = parseInt(lineNum, 10); // 3
  } else if (side === 'L') {
    // Left side (old version) - deleted or unchanged line
    // May want to show the NEW version's line number instead
    const lineNumber = parseInt(lineNum, 10);
  }
}
```

**Recommendation:** Always prefer the `R` (right/new) side line numbers for opening in the editor, as that represents the current state of the file.

### Step 5: Build srcuri URL

```javascript
// Example: http://demo.gitea.com/adamm2/foo/pulls/2/files#diff-8ec9a00bR3
// Results in:
const workspace = 'foo';         // From repo name
const filePath = 'README.md';    // From DOM extraction
const lineNumber = 3;            // From URL fragment

const srcuriUrl = `srcuri://${workspace}/${filePath}:${lineNumber}`;
// => "srcuri://foo/README.md:3"
```

## Complete Example

```javascript
function extractGiteaPRFileInfo(url) {
  const urlObj = new URL(url);
  const pathParts = urlObj.pathname.split('/');

  // Extract from URL path
  const owner = pathParts[1];      // "adamm2"
  const repo = pathParts[2];       // "foo"
  const prNumber = pathParts[4];   // "2"

  // Extract from hash
  const hashMatch = urlObj.hash.match(/#diff-([a-f0-9]+)([LR])?(\d+)?$/);
  if (!hashMatch) return null;

  const fileHash = hashMatch[1];
  const side = hashMatch[2] || null;
  const lineNum = hashMatch[3] ? parseInt(hashMatch[3], 10) : null;

  // Find the diff box in DOM
  const diffBox = document.getElementById(`diff-${fileHash}`);
  if (!diffBox) return null;

  // Extract file path (try multiple methods)
  const table = diffBox.querySelector('table.chroma');
  let filePath = table?.getAttribute('data-path');

  if (!filePath) {
    filePath = diffBox.getAttribute('data-new-filename') ||
               diffBox.getAttribute('data-old-filename');
  }

  if (!filePath) {
    const fileLink = diffBox.querySelector('.diff-file-header .file-link');
    filePath = fileLink?.textContent.trim();
  }

  return {
    owner,
    repo,
    prNumber,
    filePath,
    lineNumber: lineNum,
    side,
    workspace: repo, // Simplified - would need workspace mapping
    srcuriUrl: lineNum
      ? `srcuri://${repo}/${filePath}:${lineNum}`
      : `srcuri://${repo}/${filePath}`
  };
}

// Usage:
const info = extractGiteaPRFileInfo('http://demo.gitea.com/adamm2/foo/pulls/2/files#diff-8ec9a00bR3');
console.log(info);
// {
//   owner: "adamm2",
//   repo: "foo",
//   prNumber: "2",
//   filePath: "README.md",
//   lineNumber: 3,
//   side: "R",
//   workspace: "foo",
//   srcuriUrl: "srcuri://foo/README.md:3"
// }
```

## Edge Cases

### Renamed Files

When a file is renamed in a PR:
- `data-old-filename`: Original name
- `data-new-filename`: New name

**Recommendation:** Use `data-new-filename` for opening in editor.

### Deleted Files

- `data-new-filename` will be empty
- Use `data-old-filename` with a warning that file may not exist locally

### Binary Files

- No line numbers
- May have different diff display
- Just open the file without line number

### Multiple Files in One PR

- Each file has its own `diff-{hash}` ID
- User can have multiple hash fragments in browser history
- Extension should handle all visible diff boxes

## Testing Checklist

- [ ] Extract file path from simple PR
- [ ] Handle line numbers (R prefix)
- [ ] Handle line numbers (L prefix)
- [ ] Handle renamed files
- [ ] Handle deleted files
- [ ] Handle PRs with multiple files
- [ ] Handle PRs in subdirectories (e.g., `src/handlers/user.go`)
- [ ] Handle special characters in filenames
- [ ] Handle files with no line number in URL

## Related Files

- `gitea.yaml` - Full Gitea configuration
- `README.md` - URL patterns directory documentation
- `../click-to-open-catalog.md` - Platform catalog

---

*Last updated:* 2025-10-25
