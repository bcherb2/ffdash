## Subagent models

When spawning subagents, set the `model` explicitly:

- **Implementation** (writing or editing code, tests, docs): use Sonnet 5.5 (`model: "sonnet"`, `claude-sonnet-5-5`).
- **Exploration and summarizing** (searching the codebase, reading files, gathering context, condensing logs or docs): use Haiku 5.5 (`model: "haiku"`, `claude-haiku-5-5`).

## Commits and history

Follow `CONTRIBUTING.md`: Conventional Commits for every commit message, and a linear history (rebase onto `main`, never create merge commits).

## Agent skills

### Issue tracker

Issues live in GitHub Issues on `draoncc/ffdash`, managed with the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Default vocabulary: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `GLOSSARY.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.
