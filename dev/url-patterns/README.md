# URL Pattern Configurations

This directory contains YAML configuration files that define how to convert URLs from various web platforms into `srcuri://` protocol links.

## Purpose

These configuration files serve as:
1. **Specifications** for future Chrome extension development
2. **Documentation** of URL patterns and DOM structures for each platform
3. **Reference** for implementing URL-to-srcuri conversion logic

## Structure

Each YAML file corresponds to a specific platform and includes:

### Core Sections

- **`platform`**: Platform metadata (name, type, self-hosted status)
- **`known_instances`**: List of known domains for this platform
- **`url_patterns`**: Regular expressions to match and parse URLs
- **`dom_selectors`**: CSS selectors for Chrome extension DOM manipulation
- **`conversion`**: Logic for converting URLs to srcuri:// format
- **`examples`**: Real-world examples of URL transformations
- **`implementation_notes`**: Platform-specific quirks and gotchas
- **`test_urls`**: URLs for testing the conversion logic

### URL Pattern Format

Each pattern includes:
```yaml
- name: pattern_name
  pattern: '^regex_pattern$'
  description: "What this pattern matches"
  groups:
    domain: 1      # Capture group index
    owner: 2
    repo: 3
    path: 5
    line_start: 6
  example: "https://example.com/owner/repo/file.c#L10"
```

### Conversion Strategy

The conversion process typically follows:

1. **Match URL** against patterns
2. **Extract components** (domain, owner, repo, path, line, etc.)
3. **Map to workspace** using one of:
   - Git remote matching
   - Repository name matching
   - User-configured mapping
4. **Construct srcuri URL**: `srcuri://{workspace}/{path}:{line}[?commit={sha}]`

## Current Platforms

- [x] **Gitea** (`gitea.yaml`) - Lightweight self-hosted Git service
  - Includes Forgejo (Gitea fork)
  - Covers file browsing, commits, diffs, comparisons, and pull requests
  - Tested with https://demo.gitea.com

- [x] **TeamCity** (`teamcity.yaml`) - JetBrains CI/CD server
  - Change/commit views with file diffs
  - Build logs with compilation errors
  - Test results with stack traces
  - Tested with https://teamcity.jetbrains.com

- [x] **Sourcegraph** (`sourcegraph.yaml`) - Universal code search platform
  - Commit diffs with file changes
  - File blob view with line numbers
  - Multi-host support (GitHub, GitLab, etc.)
  - Code search results
  - Tested with https://sourcegraph.com

## Future Platforms (Priority Order)

Based on `../click-to-open-catalog.md`:

1. **GitHub** - Most popular code hosting
   - File views, PRs, commits, Actions logs
   - GitHub Enterprise support

2. **GitLab** - Second most popular
   - Self-managed and cloud
   - CI/CD pipeline logs

3. **Bitbucket** - Atlassian ecosystem
   - Cloud and Server/Data Center
   - Pipelines integration

4. **Azure DevOps** - Microsoft ecosystem
   - Repos, Pipelines, Boards

5. **Error Tracking Platforms**
   - Sentry, Rollbar, Bugsnag
   - Stack trace linking

6. **Observability Platforms**
   - Datadog, New Relic
   - APM traces with file references

## Usage in Chrome Extension

When the Chrome extension is implemented, it will:

1. Load these YAML files as configuration
2. Register URL patterns for each platform
3. Inject content scripts based on `content_script.matches`
4. Use `dom_selectors` to find and enhance links
5. Convert URLs using the `conversion` logic
6. Handle workspace mapping per `workspace_mapping.strategies`

## Example: Converting a Gitea URL

```yaml
Input URL:
  https://demo.gitea.com/wediaklup/calc/src/branch/master/add.c#L10

Parsed Components:
  domain: demo.gitea.com
  owner: wediaklup
  repo: calc
  ref: master
  path: add.c
  line: 10

Workspace Resolution:
  1. Check git remotes for: git@demo.gitea.com:wediaklup/calc.git
  2. If not found, check for workspace named "calc"
  3. If not found, prompt user to configure workspace

Output srcuri URL:
  srcuri://calc/add.c:10
```

## Adding a New Platform

To add a new platform:

1. Create `{platform}.yaml` in this directory
2. Define all sections (use `gitea.yaml` as template)
3. Add test URLs with expected outputs
4. Update `../click-to-open-catalog.md` to mark as [x] complete
5. Document any platform-specific quirks in `implementation_notes`

### Template Structure

```yaml
platform:
  name: PlatformName
  type: code-hosting|ci-cd|error-tracking|observability
  self_hosted: true|false
  description: "Brief description"

known_instances:
  - domain: example.com
    name: "Instance Name"

url_patterns:
  - name: file_view
    pattern: '^https?://...'
    groups: { ... }
    example: "..."

dom_selectors:
  file_list: { ... }

conversion:
  workspace_mapping: { ... }
  line_number: { ... }

examples:
  - gitea_url: "..."
    srcuri_url: "..."

test_urls:
  - url: "..."
    expected_workspace: "..."
```

## Testing

When implementing conversions:

1. Use `test_urls` section for unit tests
2. Verify each `url_patterns` regex matches expected URLs
3. Test workspace mapping with various git remote formats
4. Ensure line numbers are correctly extracted
5. Test with self-hosted instances if applicable

## Notes

- **Workspace mapping** is the most complex part - users may have:
  - Multiple clones of the same repo
  - Renamed local directories
  - Monorepos with different structures
- **Git reference handling** varies by platform:
  - Some use branches (GitHub/GitLab) - use `?branch=`
  - Some show specific commits (stack traces) - use `?commit=` or `?sha=`
  - Some show tags or releases - use `?tag=`
- **Self-hosted instances** may have:
  - Custom domains
  - Custom URL schemes
  - Different feature sets

## Related Documentation

- [Click-to-Open Catalog](../research/click-to-open-catalog.md) - Comprehensive list of platforms
- [URL Formats](../../URL-FORMATS.md) - srcuri URL specification
- [Protocol Handler](../../src-tauri/src/protocol_handler/parser.rs) - Current parser implementation

---

*Last updated:* 2025-10-25
