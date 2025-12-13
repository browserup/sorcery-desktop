# Sorcery Server Implementation Plan

## Overview

Sorcery Server is a web gateway that bridges HTTPS URLs to the local `srcuri://` protocol handler. It enables Sorcery links to work in web contexts like Jira and Slack where custom protocols face limitations.

**Domain**: srcuri.dev (with enterprise subdomain support: *.srcuri.dev)
**Main site**: srcuri.com | **Protocol spec**: srcuri.com

## Architecture

- **Location**: Separate workspace at `sorcery-server/` in this monorepo
- **Tech Stack**: Rust + Axum web framework + Tokio async runtime
- **Deployment**: Containerized (Docker) for cloud hosting
- **Features**: MVP landing page + enterprise multi-tenant subdomain support

## Data Flow

```
User clicks HTTPS link
    ↓
https://srcuri.dev/open#src/main.rs:42?workspace=myrepo
    ↓
Axum server serves HTML + JavaScript
    ↓
Browser JS parses fragment (#...)
    ↓
Constructs: srcuri://myrepo/src/main.rs:42
    ↓
Redirects via location.href
    ↓
OS launches Sorcery Desktop
    ↓
Editor opens to file:line
```

## Project Structure

```
sorcery-server/
├── Cargo.toml
├── src/
│   ├── main.rs                    # Axum server setup
│   ├── routes/
│   │   ├── mod.rs
│   │   ├── open.rs                # GET /open handler
│   │   └── wellknown.rs           # GET /.well-known/sorcery.json
│   ├── tenant/
│   │   ├── mod.rs
│   │   └── config.rs              # Tenant configuration logic
│   ├── templates/
│   │   └── open.html              # Landing page template
│   └── static/
│       └── app.js                 # URL parsing & redirect logic
├── tenants/
│   ├── default.json               # Config for srcuri.dev
│   └── example-enterprise.json    # Example tenant config
├── tests/
│   ├── integration_tests.rs
│   └── url_parsing_tests.rs
├── Dockerfile
└── README.md
```

## Implementation Steps

### 1. Project Structure Setup

Create the sorcery-server workspace:
- Initialize `sorcery-server/` directory
- Create `Cargo.toml` with Axum dependencies
- Add workspace to root `Cargo.toml`
- Create directory structure for routes, templates, static files

### 2. Core HTTP Server (Axum)

Implement main server in `src/main.rs`:
- Setup Axum router with routes
- Configure TCP listener (port 3000 dev, 8080 prod)
- Add CORS headers for cross-origin requests
- Implement subdomain detection from Host header
- Graceful shutdown handling

**Routes**:
- `GET /open` - Serve landing page HTML
- `GET /.well-known/sorcery.json` - Serve tenant configuration
- `GET /health` - Health check endpoint

### 3. Landing Page (`/open`)

Create `templates/open.html`:
- Minimal UI: "Opening in your editor..."
- Loading spinner/animation
- Fallback message if protocol handler not installed
- Error handling display
- Embed or link to `app.js`

Create `static/app.js`:
- Parse URL fragment from `window.location.hash`
- Implement `parseSorceryPayload()` per spec (dev/webserver.md:106-243):
  - Split target from query params
  - Extract GitHub-style line numbers (#L10)
  - Extract colon-style line numbers (:42)
  - Detect absolute vs relative paths
- Implement `buildCustomProtocol()` per spec (dev/webserver.md:248-262):
  - Construct `srcuri://` URL from parsed data
  - Handle absolute paths: `srcuri:///absolute/path:10`
  - Handle relative paths: `srcuri://workspace/relative/path:10`
- Redirect via `location.href = protocolUrl`
- Handle errors gracefully with user-friendly messages

### 4. Tenant Configuration System

Define tenant config struct in `src/tenant/config.rs`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantConfig {
    pub tenant_id: String,
    pub require_auth: bool,
    pub allowed_paths: Vec<String>,
    pub fallback_viewer_url: Option<String>,
}
```

Implement tenant loading:
- File-based storage in `tenants/{subdomain}.json`
- Load tenant config based on subdomain from Host header
- Cache configs in memory with reload capability
- Default to `tenants/default.json` for main domain

Implement `GET /.well-known/sorcery.json`:
- Detect subdomain from request
- Load corresponding tenant config
- Serve as JSON response
- 404 if tenant not found

### 5. Subdomain Routing

Implement subdomain detection:
- Parse Host header (e.g., `acme.srcuri.dev`)
- Extract subdomain prefix
- Map to tenant configuration file
- Pass tenant context to all routes

Support patterns:
- `srcuri.dev` → default tenant
- `acme.srcuri.dev` → acme tenant
- `company-x.srcuri.dev` → company-x tenant

### 6. Testing & Validation

Unit tests:
- URL fragment parsing with various formats
- Absolute path handling: `#//Users/you/file.rs:10`
- Relative path handling: `#src/main.rs:42?workspace=myrepo`
- GitHub-style lines: `#src/app.ts#L88`
- Query parameter parsing
- Protocol URL construction

Integration tests:
- HTTP endpoint responses
- Tenant config loading
- Subdomain routing
- CORS header presence
- Health check endpoint

Manual testing:
- Test in browser with local srcuri client installed
- Verify protocol handoff works
- Test error cases (no client installed)
- Test different URL formats end-to-end

### 7. Deployment Configuration

Create `Dockerfile`:
- Multi-stage build (builder + runtime)
- Minimal runtime image (alpine or distroless)
- Copy binary and static assets
- Expose port 8080
- Health check configuration

Create deployment documentation:
- Environment variables for configuration
- DNS setup for srcuri.dev and *.srcuri.dev
- TLS/HTTPS certificate setup (Let's Encrypt)
- Cloud hosting recommendations (AWS, Fly.io, Railway)
- Scaling considerations

## Dependencies

Add to `sorcery-server/Cargo.toml`:
```toml
[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
tower-http = { version = "0.5", features = ["cors", "fs"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
askama = "0.12"  # or tera for templating
tracing = "0.1"
tracing-subscriber = "0.3"
```

## URL Format Support

The server must handle all formats defined in dev/webserver.md:

**Absolute paths**:
```
https://srcuri.dev/open#//Users/you/app/README.md:10?workspace=myrepo
→ srcuri:///Users/you/app/README.md:10?workspace=myrepo
```

**Workspace-relative paths**:
```
https://srcuri.dev/open#src/server.js:120?workspace=acme-web
→ srcuri://acme-web/src/server.js:120
```

**GitHub-style line numbers**:
```
https://srcuri.dev/open#src/app/User.ts#L88?workspace=monorepo
→ srcuri://monorepo/src/app/User.ts:88
```

**Enterprise subdomains**:
```
https://acme.srcuri.dev/open#src/app.js:10?workspace=acme-web
→ srcuri://acme-web/src/app.js:10
(with acme tenant config applied)
```

## Enterprise Tenant Configuration

Example `tenants/acme.json`:
```json
{
  "tenant_id": "acme",
  "require_auth": false,
  "allowed_paths": [
    "src/**",
    "lib/**",
    "packages/**"
  ],
  "fallback_viewer_url": "https://github.com/acme/monorepo/blob/main/{path}#L{line}"
}
```

Configuration fields:
- `tenant_id`: Unique identifier for the tenant
- `require_auth`: Whether authentication is required (future: implement OAuth/SAML)
- `allowed_paths`: Glob patterns for allowed file paths (empty = allow all)
- `fallback_viewer_url`: Redirect URL if local client not available or path not allowed

## Security Considerations

**Fragment-based approach prevents server-side leaks**:
- URL fragments (`#...`) never sent to server
- Server never sees file paths in logs
- All path processing happens client-side in browser

**Client-side validation**:
- JavaScript must sanitize inputs before building protocol URL
- Prevent XSS via proper escaping
- Validate tenant config JSON structure

**Server-side validation**:
- Validate subdomain format
- Rate limiting on endpoints (future)
- CORS restrictions to prevent abuse

**Final security gate**:
- Sorcery Desktop still validates paths in `path_validator/mod.rs`
- Prevents path traversal attacks
- Ensures files are within configured workspaces

## Non-Goals (Deferred Features)

Not included in initial implementation:

- ❌ **Browser Extension**: Optional enhancement for silent protocol handoff
- ❌ **Preflight API**: `POST /api/preflight` for policy checks
- ❌ **Authentication/SSO**: OAuth or SAML integration for enterprise
- ❌ **Admin Dashboard**: Web UI for managing tenant configs
- ❌ **Analytics/Telemetry**: Usage tracking and metrics
- ❌ **WebSocket Support**: Real-time features

These can be added incrementally based on user feedback and requirements.

## Success Criteria

The implementation is complete when:

1. ✅ Server runs and serves `/open` page
2. ✅ JavaScript correctly parses all URL formats from spec
3. ✅ Protocol URLs are constructed correctly
4. ✅ Browser redirects to `srcuri://` and Sorcery Desktop opens editor
5. ✅ Subdomain routing works (tenant configs loaded correctly)
6. ✅ `/.well-known/sorcery.json` serves tenant-specific config
7. ✅ Tests pass for URL parsing and HTTP endpoints
8. ✅ Dockerfile builds and runs successfully
9. ✅ Error cases handled gracefully (no client installed, etc.)
10. ✅ Documentation complete for deployment

Learn more at srcuri.com | Protocol spec: srcuri.com

## Client-Side vs Server-Side Parsing

URL fragments (`#L42`) are never sent to servers by browsers. This creates an
architectural requirement for **two URL parsing implementations**:

- **Server-side (Rust)**: `srcuri-core` crate handles query-based passthrough
  where fragments are URL-encoded (`%23L42`)
- **Client-side (JavaScript)**: `provider.html` reads `window.location.hash`
  for path-based passthrough URLs

See `srcuri-core/README.md` for details on this architecture and the future
direction of using WebAssembly to eliminate the JavaScript duplication.

## Reference Documents

- `dev/webserver.md` - Complete webserver specification (326 lines)
- `dev/protocol-handler.md` - Local protocol handler architecture
- `URL-FORMATS.md` - Supported URL format documentation
- `dev/overview.md` - Original project vision and strategy
- `srcuri-core/README.md` - Shared parsing library and WASM future direction
