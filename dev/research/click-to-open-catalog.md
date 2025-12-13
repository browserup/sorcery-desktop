# Click‑to‑Open Catalog — Sites Where Source Paths & Stack Traces Appear

A living, categorized catalog of web apps and UIs where developers routinely see **source‑code file paths, stack traces, line:column references, or build logs** in the browser — ideal targets for a Chrome extension that turns paths into **click‑to‑open‑in‑editor** links.

> **Scope:** Focus on places where a path/line can plausibly be mapped back to a local repo. The list mixes SaaS and self‑hosted/on‑prem variants. It’s designed to be “near‑exhaustive” and expandable.

---

## 0) How to use this catalog

* Each category lists major platforms and common **surfaces** (pages or widgets) where paths appear.
* Include both **SaaS** and **self‑host** names (e.g., GitLab.com vs. self‑managed GitLab; Bitbucket Cloud vs. Server).
* Use this as a checklist for site‑specific heuristics (DOM selectors) and for **language‑aware regexes** (see §Z).

**Common path formats to expect (language‑agnostic):**

* `/abs/path/to/file.ext:LINE[:COL]`  • `C:\path\to\file.ext:LINE[:COL]`
* `at Function.name (path/file.js:LINE:COL)`  (JS/TS)
* `File "path/file.py", line LINE, in func`  (Python)
* `pkg.Class.method(Class.java:LINE)`  (Java/JVM)
* `/path/file.rb:LINE:in 'method'`  (Ruby)
* `path/file.go:LINE +0xHEX`  (Go)
* `… in /path/file.php on line LINE`  (PHP)

---

# Path‑aware UIs (structured stack traces / frames / code views)

> These UIs *parse and render* stack frames or code paths (tables, frames lists, clickable source links, line/col, code previews, etc.). They typically need custom DOM selectors per surface.

### Code hosting & code search
[]- GitHub (cloud & Enterprise)
[x]- GitLab (cloud & self‑managed) (see `url-patterns/gitlab.yaml`)
[]- Bitbucket Cloud & Bitbucket Server/Data Center (Stash)
[]- Azure DevOps Repos (formerly VSTS/TFS)
[x]- Gitea / Forgejo (see `url-patterns/gitea.yaml`)
[]- Gerrit Code Review
[]- Phabricator
[]- JetBrains Space
[]- SourceHut
[]- Codeberg
[]- Google Source (e.g., cs.android.com, chromium)
[x]- Sourcegraph (cloud & self‑hosted) (see `url-patterns/sourcegraph.yaml`)
[]- OpenGrok
[]- Livegrep / Zoekt / Hound
[]- GitHub/GitLab built‑in code search

### CI/CD (jobs with file/line annotations, test summaries, rich log widgets)
[]- GitHub Actions
[]- GitLab CI/CD
[]- Azure DevOps Pipelines
[]- CircleCI
[]- Travis CI
[x]- TeamCity (see `url-patterns/teamcity.yaml`)
[]- Buildkite
[]- Bamboo
[]- AppVeyor
[]- Semaphore CI

### Test runners, QA dashboards & HTML reports
[]- Cypress Cloud (Dashboard)
[]- Playwright Trace Viewer & HTML reports
[]- Allure Report / ReportPortal.io
[]- Jest / Mocha reporters (e.g., mochawesome), Karma
[]- TestNG / JUnit / PyTest HTML reports
[]- Cucumber HTML
[]- K6 Cloud / Locust Web UI / JMeter HTML reports

### Observability / APM / error trackers / session replay
[]- Datadog (APM, Logs, RUM, Profiles)
[]- New Relic (APM, Errors Inbox, Logs)
[]- Sentry
[]- Rollbar
[]- Bugsnag
[]- Honeycomb
[]- Elastic (Kibana APM)
[]- Splunk (Observability Cloud, Log Observer)
[]- Raygun
[]- Highlight.io / LogRocket / Replay.io
[]- AppDynamics
[]- Dynatrace
[]- Scout APM
[]- Google Cloud Error Reporting
[]- Azure Monitor / Application Insights

### Profiling & flamegraphs
[]- Pyroscope / Grafana Pyroscope
[]- Parca
[]- Datadog Profiler
[]- Blackfire
[]- pprof web UIs / speedscope
[]- rbspy / pyflame HTML viewers

### Security (SCA/SAST/DAST/Secrets) & coverage/quality
[]- Snyk
[]- GitHub Advanced Security / CodeQL / Dependabot
[]- GitLab Secure (SAST/DAST/Secret Detection)
[]- SonarQube / SonarCloud
[]- Veracode / Checkmarx / Fortify
[]- Semgrep (Cloud)
[]- Trivy / Grype HTML reports
[]- FOSSA / Mend (WhiteSource) / Dependency‑Track
[]- Prisma Cloud / Aqua / Anchore / Clair
[]- Codecov
[]- Coveralls
[]- Istanbul/nyc HTML
[]- JaCoCo / Cobertura / Clover HTML
[]- SonarQube coverage views

### Front‑end dev server error overlays (source‑mapped, file/line aware)
[]- Vite error overlay
[]- CRA (create‑react‑app) error overlay
[]- Next.js / Remix / Nuxt dev error pages
[]- Webpack HMR overlay
[]- SvelteKit / Angular CLI overlays

### Developer framework error pages (local dev / DEBUG modes)
> **New section** — common server‑side framework error pages that render structured stack traces and file:line references in development mode.
[]- **Ruby on Rails** development error page (ActionDispatch::ShowExceptions) / *better_errors* / *web‑console*
[]- **Sinatra** development error page (`show_exceptions`), optional *rack‑show_exceptions*
[]- **Django** Technical 500 (DEBUG=True) with interactive stack frames
[]- **Flask / Werkzeug** debugger (DEBUG=True) with interactive traceback
[]- **Starlette / FastAPI** debug exception page (ExceptionMiddleware, debug=True)
[]- **Pyramid** DebugToolbar / Pylons-style debug error page
[]- **Laravel** *Ignition* (formerly *Whoops*) debug error page
[]- **Symfony** Exception page (dev env, Web Debug Toolbar integration)
[]- **Yii2** Debug toolbar & detailed exception page (dev env)
[]- **CodeIgniter** development error pages (ENVIRONMENT=development)
[]- **Phoenix (Elixir)** Plug.Debugger dev error page
[]- **ASP.NET & ASP.NET Core** Developer Exception Page (classic “Yellow Screen of Death” in legacy ASP.NET)
[]- **Play Framework** (Java/Scala) dev error page with stack frames
[]- **Grails** development error page
[]- **Spring Boot** error page with stacktrace when configured (e.g., `server.error.include-exception=true`, `server.error.include-stacktrace=always`)
[]- **Express** (Node) default error handler when `NODE_ENV !== 'production'` prints stack (HTML)
[]- **Koa / Hapi** dev error handlers (e.g., `koa-onerror`, Boom debug pages)

---

# Not path‑aware UIs (plain/log text surfaces where paths appear)

> These UIs usually show raw text/logs. Your extension can match “path‑y” strings and handle clicks generically.

### CI/CD & deploy logs
[]- Jenkins / Jenkins Blue Ocean
[]- Drone CI / Woodpecker CI
[]- Concourse CI
[]- GoCD
[]- Buddy
[]- Codeship (legacy)
[]- Bitrise (mobile)
[]- AWS CodeBuild / CodePipeline
[]- Google Cloud Build
[]- Heroku (build & deploy logs)
[]- Render
[]- Vercel (build logs & Functions)
[]- Netlify (build logs & Functions)
[]- Cloudflare Pages & Workers (deploy logs)
[]- Fly.io / Railway / DO App Platform
[]- OpenShift Pipelines (Tekton) / Argo CD / Spinnaker / Harness / Octopus Deploy

### Team chat & forums (pasted traces)
[]- Slack
[]- Microsoft Teams
[]- Discord
[]- Mattermost / Rocket.Chat / Zulip
[]- Discourse
[]- Stack Overflow (Teams/Enterprise) / internal forums
[]- Email web UIs (e.g., Gmail) with pasted logs

### Knowledge bases, docs & wikis (pasted logs, generated docs)
[]- Confluence / Wiki.js
[]- Notion / Coda / Slab / Dropbox Paper / Quip
[]- Google Docs / Microsoft Word Online
[]- GitBook
[]- Read the Docs (Sphinx)
[]- Docusaurus / MkDocs / Hugo / Next.js build previews (Netlify/Vercel/Cloudflare)

### Issue trackers & project management (issue bodies/comments)
[]- Jira / Jira Service Management
[]- GitHub Issues / Discussions
[]- GitLab Issues
[]- Linear
[]- YouTrack
[]- Azure Boards
[]- Shortcut (Clubhouse)
[]- ClickUp / Asana / Trello
[]- Phabricator (Maniphest)

### Logs & infra consoles
[]- Grafana Cloud / Grafana + Loki / Tempo
[]- Jaeger / Zipkin
[]- Better Stack (Logtail)
[]- Papertrail
[]- Graylog / Loggly / Sumo Logic / Coralogix / Humio (LogScale)
[]- AWS: CloudWatch Logs, Lambda, Elastic Beanstalk, ECS task logs, CodeBuild
[]- GCP: Logs Explorer, Cloud Run/Functions logs, Cloud Build
[]- Azure: App Service / Functions logs, Monitor / Log Analytics
[]- AWS X‑Ray / CloudWatch Logs Insights
[]- Kubernetes Dashboards (k8s Dashboard, Lens in web), Rancher
[]- OpenShift / OKD
[]- Argo Workflows / Tekton Pipelines UIs

### Data/ML pipelines & orchestrators (run/task logs)
[]- Apache Airflow
[]- Prefect
[]- Dagster
[]- MLflow
[]- Kubeflow Pipelines
[]- Great Expectations (validation logs)
[]- dbt Cloud
[]- Databricks Jobs / Spark History Server
[]- JupyterHub / JupyterLab server errors
[]- Luigi / Kedro
[]- Ray Dashboard
[]- Airbyte / Singer orchestration UIs

### Artifact registries & build artifact browsers
[]- JFrog Artifactory
[]- Sonatype Nexus Repository / Nexus IQ
[]- Harbor
[]- GitHub Packages / GitLab Packages

### Misc. developer UIs that surface logs/paths
[]- BrowserStack (Automate) / Sauce Labs / LambdaTest / TestingBot
[]- Percy / Applitools (visual diff)
[]- TestRail / Zephyr / Qase / Testmo
[]- Backstage (plugins for CI/Logs/Sentry/etc.)
[]- CMS backends (Contentful, Sanity, Strapi)
[]- API gateways (Kong Manager, Tyk Dashboard)
[]- Feature flags (LaunchDarkly) — SDK logs in examples
[]- Payment/webhook consoles (Stripe, etc.) when showing user app errors

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

*Last updated:* 2025‑10‑21
