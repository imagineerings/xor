---
title: Contributing
description: Prepare Sim contributions with clear issues, scoped pull requests, tests, and release notes.
---

# Contributing

Contributions work best when the problem, scope, and validation are clear. Start small when you are new to the codebase.

## Before You Start

- Search existing issues and pull requests.
- Reproduce bugs locally when possible.
- Identify the crate or docs area involved.
- Ask for design feedback before large rewrites.

## Pull Requests

Good pull requests are focused and easy to evaluate:

- Keep one behavior change per PR.
- Include tests or explain why tests are not practical.
- Avoid drive-by formatting or unrelated cleanup.
- Add release notes for user-facing changes.

## Code Style

For Rust changes:

- Propagate errors instead of panicking.
- Avoid silently discarding fallible results.
- Use descriptive variable names.
- Prefer existing module patterns.

For docs changes:

- Keep headings direct.
- Prefer links to canonical pages over duplicating long reference material.
- Validate changed sidebar entries and relative links.

## Reporting Issues

Include the Sim version, platform, reproduction steps, expected behavior, actual behavior, and relevant logs or screenshots.
