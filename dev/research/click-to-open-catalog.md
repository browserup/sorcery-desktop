Splunk (Cisco) / Splunk Observability — huge installed base (Splunk reports 15,000+ customers) and large ARR scale (FY2024 $4.2B Total ARR).

Dynatrace — very strong enterprise penetration; ~4,100 customers (as of Mar 31, 2025) and very high verified review volume.

New Relic — longstanding APM/observability player with high verified review volume.

AppDynamics (Cisco) — large long-lived enterprise deployment footprint, still heavily used.

Grafana Labs (Grafana Cloud / Enterprise Stack) — big commercial growth ($400M+ ARR and 7,000 customers) plus enormous OSS ecosystem halo.

Elastic Observability — common choice especially where Elastic/ELK is already present.

IBM Instana — meaningful enterprise presence, especially in IBM-heavy shops.

Honeycomb — smaller by volume, influential (especially tracing-first / “observability 2.0” teams).

Grafana (dashboards)

Prometheus (metrics) — still the de facto standard; majority production usage in surveys

AWS CloudWatch (cloud default) — consistently top-3 in usage reports

OpenTelemetry (instrumentation + collection) — rapidly growing production usage and momentum

Loki (logs) — very common in “Grafana stack” deployments

Jaeger / Tempo (tracing backends) — common pairings with OTel + Grafana stacks


---



# Click‑to‑Open Catalog — Sites Where Source Paths & Stack Traces Appear

A living, categorized catalog of web apps and UIs where developers routinely see **source‑code file paths, stack traces, line:column references, or build logs** in the browser — ideal targets for a Chrome extension that turns paths into **click‑to‑open‑in‑editor** links.

> **Scope:** Focus on places where a path/line can plausibly be mapped back to a local repo. The list mixes SaaS and self‑hosted/on‑prem variants.

---

## How to use this catalog

The catalog is organized by **how the UI relates to source code**, which determines our implementation strategy:

| Category | UI Behavior | Implementation | Testing |
|----------|-------------|----------------|---------|
| **Source-Code Aware** | Navigates repo content (files, branches, commits) | Custom extractors for workspace/branch/SHA/path | Need fixtures |
| **Backtrace-Aware** | Renders structured stack traces (tables, expandable frames) | Custom DOM selectors per platform | Need fixtures |
| **Backtrace-Unaware** | Raw text/logs where traces may appear | Generic regex heuristics | Works automatically |

**Legend:** `[x]` = implemented, `[ ]` = not yet implemented

**Common path formats to expect (language‑agnostic):**

* `/abs/path/to/file.ext:LINE[:COL]`  • `C:\path\to\file.ext:LINE[:COL]`
* `at Function.name (path/file.js:LINE:COL)`  (JS/TS)
* `File "path/file.py", line LINE, in func`  (Python)
* `pkg.Class.method(Class.java:LINE)`  (Java/JVM)
* `/path/file.rb:LINE:in 'method'`  (Ruby)
* `path/file.go:LINE +0xHEX`  (Go)
* `… in /path/file.php on line LINE`  (PHP)

---

# 1) Source-Code Aware UIs

> The UI **directly navigates repository content** — files, directories, branches, commits. There is a 1:1 mapping between UI elements and repo structure. We extract workspace, branch, SHA, and path from URL and DOM to build precise `srcuri://` links.
>
> **Implementation:** Custom extractors needed per platform. **Testing:** Need MHTML fixtures.

### Code Hosting Platforms
[x]- GitHub (cloud & Enterprise)
[x]- GitLab (cloud & self‑managed)
[ ]- Bitbucket Cloud & Bitbucket Server/Data Center
[ ]- Azure DevOps Repos (formerly VSTS/TFS)
[x]- Gitea / Forgejo
[x]- Codeberg
[ ]- Gerrit Code Review
[ ]- Phabricator (Diffusion/Differential)
[ ]- JetBrains Space
[ ]- SourceHut
[ ]- AWS CodeCommit
[ ]- RhodeCode
[ ]- Perforce Helix Swarm
[ ]- Google Source (cs.android.com, chromium)
[ ]- cgit / gitweb (self-hosted)
[ ]- ViewVC (SVN/CVS web viewer)

### Code Search & Intelligence
[x]- Sourcegraph (cloud & self‑hosted)
[ ]- OpenGrok
[ ]- Livegrep
[ ]- Zoekt-based UIs
[ ]- Hound
[ ]- GitHub code search (advanced)
[ ]- GitLab code search

### Web IDEs & Sandboxes
> Source files + runtime stack traces in-console
[ ]- GitHub Codespaces
[ ]- Gitpod
[ ]- Replit
[ ]- StackBlitz
[ ]- CodeSandbox
[ ]- AWS Cloud9
[ ]- Eclipse Theia-based web IDEs

### Code Quality & Coverage (file/line views)
> These render source files with inline annotations
[ ]- SonarQube / SonarCloud (code views)
[ ]- Codecov (line-level coverage)
[ ]- Coveralls (line-level coverage)
[ ]- Istanbul/nyc HTML reports
[ ]- JaCoCo / Cobertura / Clover HTML reports

---

# 2) Backtrace-Aware UIs

> The UI **renders stack traces with structure** — frames in tables, expandable rows, file:line as distinct DOM elements. We need site-specific DOM selectors to extract path, line, column from the structured UI and make frames clickable.
>
> **Implementation:** Custom DOM selectors needed per platform. **Testing:** Need MHTML fixtures.

### Error / Crash / Exception Trackers
[x]- Sentry
[ ]- Bugsnag
[ ]- Rollbar
[ ]- Raygun
[ ]- Airbrake
[ ]- Honeybadger
[ ]- Exceptionless
[ ]- GlitchTip (Sentry-compatible)
[ ]- Appsignal
[ ]- Backtrace (crash reporting / debugging)

### Mobile Crash Reporting
[ ]- Firebase Crashlytics
[ ]- Instabug
[ ]- Embrace

### Session Replay with Error Views
> These capture JS errors with structured stack traces
[ ]- LogRocket
[ ]- FullStory
[ ]- Highlight.io
[ ]- OpenReplay

### APM Error/Exception Views
> The structured error/exception UIs within APM platforms
[x]- Datadog Error Tracking
[ ]- New Relic Errors Inbox / Error Analytics
[ ]- Dynatrace (error views)
[ ]- AppDynamics (error views)
[ ]- Elastic APM (error views)
[ ]- Splunk APM (error views)
[ ]- Scout APM
[ ]- Lightstep / ServiceNow Cloud Observability
[ ]- Instana
[ ]- SigNoz (open-source APM)
[ ]- Uptrace (OpenTelemetry-native)
[ ]- Honeycomb (error views)

### Cloud Provider Error UIs
[ ]- Google Cloud Error Reporting
[ ]- Azure Application Insights (Failures view)
[ ]- AWS X-Ray (error traces)

### CI/CD Test Results (structured)
> Test failure UIs with structured stack traces per test
[x]- Jenkins Test Results
[x]- TeamCity (test failures)
[ ]- GitHub Actions (test summary annotations)
[ ]- GitLab CI (test reports)
[ ]- Azure Pipelines (test results)
[ ]- CircleCI (test insights)
[ ]- Buildkite (test analytics)

### Test Reporting Dashboards
[ ]- Allure TestOps / Allure Report
[ ]- ReportPortal
[ ]- Cypress Cloud
[ ]- Playwright HTML reports
[ ]- TestRail
[ ]- qTest
[ ]- Zephyr / Xray (Jira apps)
[ ]- PractiTest
[ ]- Launchable

### Security Findings (file:line in findings)
> SAST/DAST tools that show findings with source locations
[ ]- Snyk Code
[ ]- Semgrep App
[ ]- GitHub CodeQL / Advanced Security
[ ]- GitLab Secure (SAST/DAST)
[ ]- SonarQube (issues view)
[ ]- Checkmarx
[ ]- Veracode
[ ]- Fortify
[ ]- Coverity
[ ]- DeepSource
[ ]- Codacy
[ ]- StackHawk
[ ]- OWASP ZAP (findings UI)
[ ]- Burp Suite Enterprise

### Framework Dev Error Pages
> Server-side framework error pages with structured stack traces
[x]- Ruby on Rails (ActionDispatch / better_errors / web-console)
[ ]- Sinatra (show_exceptions)
[ ]- Django (DEBUG=True technical 500)
[ ]- Flask / Werkzeug debugger
[x]- FastAPI / Starlette debug page
[ ]- Pyramid DebugToolbar
[x]- Laravel Ignition
[ ]- Symfony Exception page
[ ]- Phoenix (Elixir) Plug.Debugger
[ ]- ASP.NET Core Developer Exception Page
[ ]- Spring Boot error page (with stacktrace)
[ ]- Play Framework dev error page
[x]- Express/Koa/Hapi dev error handlers

### Frontend Dev Server Overlays
> Source-mapped, structured error overlays
[ ]- Vite error overlay
[x]- Create React App error overlay
[x]- Next.js / Remix / Nuxt error pages
[ ]- Webpack HMR overlay
[ ]- SvelteKit / Angular CLI overlays

### Profiling & Flamegraphs
> Stack frames in profiling UIs
[ ]- Pyroscope / Grafana Pyroscope
[ ]- Parca
[ ]- Datadog Continuous Profiler
[ ]- Blackfire
[ ]- pprof web UIs / speedscope

---

# 3) Backtrace-Unaware UIs (Free-form Text)

> The UI displays **raw text or logs** where stack traces *may* appear but aren't parsed or structured. We use language-aware regex heuristics to detect file:line patterns and make them clickable.
>
> **Implementation:** Generic regex — no site-specific work needed. **Testing:** Should work automatically.

### CI/CD Console Logs
[ ]- Jenkins console output
[ ]- GitHub Actions job logs
[ ]- GitLab CI job logs
[ ]- Azure Pipelines logs
[ ]- CircleCI job output
[ ]- Travis CI logs
[ ]- Buildkite logs
[ ]- Bamboo logs
[ ]- Drone CI / Woodpecker CI
[ ]- Concourse CI
[ ]- GoCD
[ ]- Semaphore CI
[ ]- Buddy
[ ]- Codefresh
[ ]- Harness CI
[ ]- Google Cloud Build
[ ]- AWS CodeBuild / CodePipeline

### Logging & Log Search Platforms
[ ]- Splunk Enterprise / Cloud (Search app)
[ ]- Elastic Kibana (Discover)
[ ]- OpenSearch Dashboards
[ ]- Datadog Logs
[ ]- New Relic Logs
[ ]- Grafana + Loki (Explore)
[ ]- Sumo Logic
[ ]- Graylog
[ ]- Humio / CrowdStrike Falcon LogScale
[ ]- Mezmo / LogDNA
[ ]- Logz.io
[ ]- Coralogix
[ ]- Sematext Logs
[ ]- Better Stack (Logtail)
[ ]- Papertrail
[ ]- Loggly
[ ]- Axiom
[ ]- Chronosphere

### Cloud Provider Log Consoles
[ ]- AWS CloudWatch Logs / Logs Insights
[ ]- AWS Lambda console (invocation errors)
[ ]- AWS Elastic Beanstalk logs
[ ]- AWS ECS / EKS logs
[ ]- GCP Cloud Logging (Logs Explorer)
[ ]- GCP Cloud Run / Functions logs
[ ]- Azure Monitor Logs / Log Analytics
[ ]- Azure App Service / Functions logs

### Deployment & Hosting Logs
[ ]- Vercel (build logs, function errors)
[ ]- Netlify (build logs, function logs)
[ ]- Heroku (build & runtime logs)
[ ]- Render
[ ]- Fly.io
[ ]- Railway
[ ]- DigitalOcean App Platform
[ ]- Cloudflare Pages / Workers
[ ]- Firebase Hosting / Functions logs
[ ]- Fastly Compute logs
[ ]- Supabase (edge function logs)
[ ]- SST Console
[ ]- Serverless Framework dashboard

### Kubernetes & Container Platforms
[ ]- Kubernetes Dashboard
[ ]- Rancher
[ ]- OpenShift Console
[ ]- Lens (web variants)
[ ]- Argo CD UI
[ ]- Flux UIs
[ ]- Portainer
[ ]- Docker Hub build logs

### Distributed Tracing (log-like views)
[ ]- Jaeger
[ ]- Zipkin
[ ]- Grafana Tempo
[ ]- AWS X-Ray (trace logs)

### Issue Trackers & Project Management
> Stack traces pasted in issue bodies/comments
[ ]- Jira / Jira Service Management
[ ]- GitHub Issues / Discussions
[ ]- GitLab Issues
[ ]- Linear
[ ]- YouTrack
[ ]- Azure Boards
[ ]- Shortcut (Clubhouse)
[ ]- ClickUp / Asana / Trello
[ ]- Phabricator (Maniphest)
[ ]- ServiceNow

### Incident Management & On-Call
> Stack traces in alerts, runbooks, incident notes
[ ]- PagerDuty
[ ]- Opsgenie
[ ]- Splunk On-Call (VictorOps)
[ ]- incident.io
[ ]- FireHydrant
[ ]- Rootly
[ ]- Blameless

### Team Chat & Forums
> Stack traces pasted in messages
[ ]- Slack
[ ]- Microsoft Teams
[ ]- Discord
[ ]- Mattermost / Rocket.Chat / Zulip
[ ]- Discourse
[ ]- Stack Overflow (Teams/Enterprise)

### Knowledge Bases & Wikis
> Stack traces in documentation
[ ]- Confluence / Wiki.js
[ ]- Notion / Coda / Slab
[ ]- Google Docs / Microsoft Word Online
[ ]- GitBook
[ ]- Read the Docs

### Support & Ticketing
[ ]- Zendesk
[ ]- Freshdesk
[ ]- Intercom

### Data/ML Pipeline Logs
[ ]- Apache Airflow
[ ]- Prefect
[ ]- Dagster
[ ]- MLflow
[ ]- Kubeflow Pipelines
[ ]- dbt Cloud
[ ]- Databricks Jobs
[ ]- Spark History Server
[ ]- Ray Dashboard

### Developer Portals
> Often embed logs/traces from other sources
[ ]- Backstage (Spotify)
[ ]- Port
[ ]- Cortex
[ ]- OpsLevel
[ ]- Atlassian Compass
[ ]- Datadog Service Catalog

### Browser & Device Testing
[ ]- BrowserStack
[ ]- Sauce Labs
[ ]- LambdaTest
[ ]- HeadSpin

### Artifact Registries
[ ]- JFrog Artifactory
[ ]- Sonatype Nexus
[ ]- Harbor
[ ]- GitHub Packages / GitLab Packages

### Misc Developer UIs
[ ]- Percy / Applitools / Chromatic (visual diff logs)
[ ]- CMS backends (Contentful, Sanity, Strapi)
[ ]- API gateways (Kong Manager, Tyk Dashboard)
[ ]- Feature flags (LaunchDarkly)
[ ]- Payment consoles (Stripe webhook logs)

---

## X) Priority Platform Details

Detailed implementation specifications for high-priority platforms, organized by how the UI relates to source code.

---

## X.1) Source-Code Aware UIs

> The UI **directly navigates repository content** — files, directories, branches, commits. There is a 1:1 mapping between UI elements and repo structure. We extract workspace, branch, SHA, and path from URL and DOM to build precise `srcuri://` links.

### [x] GitHub (cloud & Enterprise)

**Screens to handle:**
- [x] Repo home + file browser (paths in READMEs, code snippets, links)
- [x] File view ("blob") — line numbers, permalinks
- [x] Pull Request: Files changed (diff view links / line anchors)
- [ ] Pull Request: Conversation (stack traces in comments — *backtrace-unaware*)
- [ ] Actions: Workflow run summary
- [ ] Actions: Job logs (stack traces in step output — *backtrace-unaware*)

> GitHub Actions run/job/log structure is documented: https://docs.github.com/en/actions

**URL patterns:**
- `https://github.com/{owner}/{repo}`
- `https://github.com/{owner}/{repo}/tree/{ref}/{path?}`
- `https://github.com/{owner}/{repo}/blob/{ref}/{path}`
- `https://github.com/{owner}/{repo}/pull/{number}`
- `https://github.com/{owner}/{repo}/pull/{number}/files`
- `https://github.com/{owner}/{repo}/actions/runs/{run_id}`
- `https://github.com/{owner}/{repo}/actions/runs/{run_id}/job/{job_id}` (often has `#step:{n}:{line}` anchors)

**Public example URLs:**
- https://github.com/rails/rails
- https://github.com/rails/rails/blob/main/README.md
- https://github.com/rails/rails/pulls
- https://github.com/rust-lang/rust/actions
- https://github.com/elastio/bon/actions/runs/14971709746/job/42053977014

---

### [x] GitLab (cloud & self-managed)

**Screens to handle:**
- [x] Project home
- [x] Repository tree + file blob pages
- [x] Merge Request: Changes (diff view)
- [ ] Merge Request: Overview / discussion (stack traces in comments — *backtrace-unaware*)
- [ ] Pipelines list
- [ ] Pipeline detail
- [ ] Job detail + job log (stack traces in output — *backtrace-unaware*)

> GitLab merge requests and CI/CD pipeline concepts: https://docs.gitlab.com/ee/ci/

**URL patterns:**
- `https://gitlab.com/{group}/{project}`
- `https://gitlab.com/{group}/{project}/-/tree/{ref}/{path?}`
- `https://gitlab.com/{group}/{project}/-/blob/{ref}/{path}`
- `https://gitlab.com/{group}/{project}/-/merge_requests/{iid}`
- `https://gitlab.com/{group}/{project}/-/merge_requests/{iid}/diffs`
- `https://gitlab.com/{group}/{project}/-/pipelines`
- `https://gitlab.com/{group}/{project}/-/pipelines/{pipeline_id}`
- `https://gitlab.com/{group}/{project}/-/jobs/{job_id}`

**Public example URLs:**
- https://gitlab.com/gitlab-org/gitlab
- https://gitlab.com/gitlab-org/gitlab/-/merge_requests
- https://gitlab.com/gitlab-org/gitlab/-/merge_requests/32065
- https://gitlab.com/gitlab-org/gitlab/-/pipelines

---

## X.2) Backtrace-Aware UIs

> The UI **renders stack traces with structure** — frames in tables, expandable rows, file:line as distinct elements. We need site-specific DOM selectors to extract path, line, column from the structured UI and make frames clickable.

### [x] Sentry

**Screens to handle:**
- [x] Issue details page (stack trace frame list is the main target)
- [ ] Stack frame expanders (source context / surrounding lines when available)
- [ ] Discover / Events views when rendering exception payloads

> Sentry source context documentation: https://docs.sentry.io/product/issues/issue-details/

**URL patterns:**
- `https://{org}.sentry.io/issues/{issue_id}/`
- `https://{org}.sentry.io/organizations/{org}/issues/{issue_id}/`
- Event details: `.../events/{event_id}/`

**Public example URL:**
- Requires login; treat as pattern-only unless demo org available

---

### [x] Datadog (Error Tracking + APM)

**Screens to handle:**
- [x] Error Tracking views (stack trace frames — structured UI)
- [x] APM Trace view (error spans with stack traces)

> Datadog Error Tracking: https://docs.datadoghq.com/tracing/error_tracking/
> Source Code Integration: https://docs.datadoghq.com/integrations/guide/source-code-integration/

**URL patterns:**
- `https://app.datadoghq.com/apm/traces` (trace list)
- `https://app.datadoghq.com/apm/trace/{trace_id}` (trace detail)
- Error Tracking: varies by product area / org settings; match stack frame widgets

**Public example URL:**
- Requires login (pattern-only)

---

### [ ] New Relic (Errors + APM)

**Screens to handle:**
- [ ] APM Errors / Error Analytics pages (stack traces + exception trace panels)
- [ ] Errors Inbox (grouped error detail pages)
- [ ] Distributed tracing views with exception attributes

> New Relic Errors UI: https://docs.newrelic.com/docs/errors-inbox/errors-inbox/

**URL patterns:**
- `https://one.newrelic.com/...` (entity-centered routes)
- Errors inbox: `https://one.newrelic.com/errors-inbox/...`
- APM errors: under entity pages; treat as stack trace widgets anywhere in APM

**Public example URL:**
- Requires login (pattern-only)

---

### [ ] Elastic / Kibana (APM Errors)

**Screens to handle:**
- [ ] Observability / APM Errors view — grouped exceptions + stack trace panels

> Elastic APM: https://www.elastic.co/guide/en/apm/guide/current/apm-ui.html

**URL patterns:**
- `https://{kibana-host}/app/apm` (and subroutes for services/errors)
- With Spaces: `.../s/{space}/app/apm`

**Public example URL:**
- Most are private (pattern-only)

---

### [ ] Google Cloud Error Reporting

**Screens to handle:**
- [ ] Error group detail pages (stack trace is central, structured UI)

> Error Reporting: https://cloud.google.com/error-reporting/docs

**URL patterns:**
- `https://console.cloud.google.com/errors/`

**Public example URL:**
- Usually requires login

---

### [ ] Azure Application Insights (Failures)

**Screens to handle:**
- [ ] Application Insights Failures view (exceptions tab — structured stack traces)
- [ ] Exception detail pane / drilldowns
- [ ] Transaction diagnostics views with exception stacks

> Microsoft Failures/Exceptions documentation: https://learn.microsoft.com/en-us/azure/azure-monitor/app/asp-net-exceptions

**URL patterns:**
- Azure portal hash-routes; match by host + page semantics:
- `https://portal.azure.com/` (then App Insights blades)

**Public example URL:**
- Portal requires login; docs are the public reference

---

### [x] Jenkins (Test Results)

**Screens to handle:**
- [x] Test results pages (JUnit, etc.) with stack traces per test failure — structured tables

> Jenkins URL shapes: https://www.jenkins.io/doc/book/using/

**URL patterns:**
- `https://{jenkins-host}/job/{job}/{build}/testReport/`

**Public example URLs:**
- https://ci.jenkins.io/job/Websites/job/jenkins.io/job/master/lastSuccessfulBuild/testReport/
- https://ci.jenkins.io/job/Plugins/job/workflow-cps-plugin/job/master/749/testReport/org.jenkinsci.plugins.workflow.cps/PersistenceProblemsTest/windows_21___Build__windows_21____inProgressNormal/

---

## X.3) Backtrace-Unaware UIs (Free-form Text)

> The UI displays **raw text or logs** where stack traces *may* appear but aren't parsed or structured. We use language-aware regex heuristics to detect file:line patterns and make them clickable. No site-specific selectors needed — this is the extension's generic capability.

### [ ] Jira (Cloud + Server/DC)

**Screens to handle:**
- [ ] Issue view (description + comments: stack traces pasted as text/code blocks)
- [ ] Issue create/edit (rich text editor)
- [ ] Issue navigator / search results (snippets)

**URL patterns:**
- Cloud: `https://{site}.atlassian.net/browse/{KEY-123}`
- Server/DC: `https://{jira-host}/browse/{KEY-123}`
- Project list: `.../projects`
- Search/navigator: `.../issues/?jql=...`

**Public-ish example URLs:**
- https://issues.apache.org/jira/projects/
- https://issues.apache.org/jira/browse/ARIA

---

### [ ] Datadog (Logs Explorer)

**Screens to handle:**
- [ ] Logs Explorer (stack traces in log event detail panel — raw text)

**URL patterns:**
- `https://app.datadoghq.com/logs` (logs explorer)

**Public example URL:**
- Requires login (pattern-only)

---

### [ ] Elastic / Kibana (Discover)

**Screens to handle:**
- [ ] Discover (`/app/discover`) — stack traces in log fields + document detail flyout (raw text)

> Elastic Discover: https://www.elastic.co/guide/en/kibana/current/discover.html

**URL patterns:**
- `https://{kibana-host}/app/discover`
- With Spaces: `.../s/{space}/app/discover`

**Public example URL:**
- Most are private (pattern-only)

---

### [ ] Splunk (Search & Reporting)

**Screens to handle:**
- [ ] Search page + results timeline (stack traces in events — raw text)
- [ ] Event detail panel (multi-line stack traces)
- [ ] Saved searches / dashboards with event panels

> Splunk Search app: https://docs.splunk.com/Documentation/Splunk/latest/Search/GetstartedwithSearch

**URL patterns:**
- `https://{splunk-host}/app/search/search`
- `https://{splunk-host}/app/{app}/{view}` (dashboards/views)

**Public example URL:**
- Usually private (pattern-only)

---

### [ ] AWS CloudWatch Logs (and Logs Insights)

**Screens to handle:**
- [ ] Log groups list → log streams → log event view (stack traces — raw text)
- [ ] Logs Insights query results (stack traces in result rows / event detail)

> CloudWatch Logs: https://docs.aws.amazon.com/AmazonCloudWatch/latest/logs/
> Logs Insights: https://docs.aws.amazon.com/AmazonCloudWatch/latest/logs/AnalyzingLogData.html

**URL patterns:**
- Console hash-routes; match by host + "cloudwatch" + page content:
- `https://console.aws.amazon.com/cloudwatch/`
- Inside: "Log groups", "Log streams", "Logs Insights"

**Public example URL:**
- Console requires login; use docs as reference

---

### [ ] Google Cloud Logging (Logs Explorer)

**Screens to handle:**
- [ ] Cloud Logging "Logs Explorer" (stack traces in log entries — raw text)

> Cloud Logging: https://cloud.google.com/logging/docs

**URL patterns:**
- `https://console.cloud.google.com/logs/` (Logs Explorer)

**Public example URL:**
- Usually requires login

---

### [ ] Jenkins (Console Output)

**Screens to handle:**
- [ ] Job page
- [ ] Build page
- [ ] Console Output (primary target; stack traces everywhere — raw text)

**URL patterns:**
- `https://{jenkins-host}/job/{job}/`
- `https://{jenkins-host}/job/{job}/{build}/`
- `https://{jenkins-host}/job/{job}/{build}/console` (or `/consoleFull`)

**Public example URLs:**
- https://ci.jenkins.io/job/Websites/job/jenkins.io/job/master/lastSuccessfulBuild/
- https://ci.jenkins.io/job/Websites/job/jenkins.io/job/master/lastSuccessfulBuild/console

---

### [ ] Grafana (Explore Logs / Loki)

**Screens to handle:**
- [ ] Explore (logs queries; log line detail — raw text)
- [ ] Logs Drilldown app (where present) showing log events with multi-line payloads

> Grafana Logs in Explore: https://grafana.com/docs/grafana/latest/explore/logs-integration/

**URL patterns:**
- `https://{grafana-host}/explore`
- Drilldown (varies): `.../a/{plugin-id}/explore/...`

**Public example URLs:**
- https://play.grafana.org/explore?orgId=1
- https://play.grafana.org/a/grafana-lokiexplore-app/explore/service/Grafana%20Community%20Forums/fields?from=now-15m&to=now

---

## Y) MVP Priorities (high signal, broad reach)

1. GitHub, GitLab, Bitbucket, Azure DevOps (PRs, diffs, Actions/CI logs)
2. Jenkins, CircleCI, Buildkite, TeamCity (job logs)
3. Datadog, New Relic, Sentry, Rollbar, Bugsnag (error/trace views)
4. Elastic/Kibana, Grafana + Loki (log explorers)
5. Sourcegraph (search results)
6. Jira/Linear/YouTrack + Slack/Discord (pasted traces)
7. Vercel/Netlify/Cloudflare build logs
8. Developer framework error pages (Rails/Django/Flask/etc.)

---

## Z) Language‑Aware Regex Hints (starter set)

> Use **negative look‑behinds** to avoid matching URLs, and prefer groups for `path`, `line`, `col`.

**Posix/Windows generic:**

```
(?P<path>(?:[A-Za-z]:\|/)[^\s:()]+?\.[A-Za-z0-9_]+):(?P<line>\d+)(?::(?P<col>\d+))?
```

**JavaScript/TypeScript:**

```
at\s+[^()]+\((?P<path>[^:()]+):(?P<line>\d+):(?P<col>\d+)\)
```

**Python:**

```
File\s+"(?P<path>.+?)",\s+line\s+(?P<line>\d+)
```

**Java/JVM:**

```
\((?P<file>[^:()]+\.java):(?P<line>\d+)\)
```

**Ruby:**

```
(?P<path>/.+?\.rb):(?P<line>\d+)(?::in\s+`[^`]+`)?
```

**Go:**

```
(?P<path>[^\s:]+\.go):(?P<line>\d+)\b
```

**PHP:**

```
 in\s+(?P<path>/.+?\.php)\s+on\s+line\s+(?P<line>\d+)
```

> **Windows drive + UNC:** add `(?:\\\\[A-Za-z0-9_.-]+\\[^\s:]+|[A-Za-z]:\\[^\s:]+)` to `path`.

---

## Implementation Notes

* Treat this list as **append‑only**; add per‑site notes: known selectors, scroll/virtualized lists, auth walls.
* Long‑tail: default to language regexes everywhere; then layer site‑specific tweaks.
* Guardrails: don’t match URLs, routes, or secrets; prefer file extensions & plausible dirs (`src/`, `lib/`, `app/`, `pkg/`).
* Repo resolution: map `<repo, path>` via heuristics (workspace roots, monorepo mapping, source maps, VCS remotes).

---

*Last updated:* 2026‑01‑11
