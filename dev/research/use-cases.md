# Sorcery Desktop Use Cases

> **Maintenance**: When adding a feature that represents a new use case, add it here. Use the "As a..." format. This document serves two purposes: (1) a testing checklist and (2) source material for marketing. Only add major use cases here, not minor behavioral improvements.

## Primary Use Cases

**As a developer**, I want to click a code link and have it open in my editor at the exact line, so I can immediately start working.

**As a team member**, I want to share code references that work regardless of which editor recipients use, so we avoid editor lock-in.

**As a code reviewer**, I want to share specific line references in PR comments, Slack, or docs that open in the recipient's editor.

## Collaboration

**As a documentation author**, I want to embed clickable code references in wikis or READMEs that open the source in the reader's editor.

**As a mentor**, I want to share exact code locations with teammates so they can see the code in their own environment.

## Git Workflows

**As a developer debugging a regression**, I want to open a file at a specific commit to see what the code looked like when the bug was introduced.

**As a new team member**, I want to click a link to a repo I don't have and be prompted to clone it, so I can get started without manual setup.

## Multi-Project Development

**As a developer with multiple projects**, I want partial-path links to find the right workspace automatically, so I don't need to specify full paths.

**As a developer working across languages**, I want different editors per workspace (PyCharm for Python, WebStorm for JS), so links open in the appropriate IDE.

## Link Creation & Web Integration

**As a developer in my editor**, I want to generate a universal link for the current line of code to share with colleagues.

**As a developer browsing a remote repository**, I want to open the current file in my local editor to transition from reading to editing.

**As a developer**, I want to convert a web-based repository link (GitHub/GitLab) into a universal link to share with others.

**As an observability tools vendor**, I want to create a link-to-edit that I can embed in my web-based observability tool that opens the user's editor to the right file
and line, and if they don't have sorcery installed, I want to have it fall back to a view-only link to the repo.

