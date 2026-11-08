# Contributing

Contributions are welcome. This page covers how commits and history are kept. For building from source, see [BUILD.md](BUILD.md).

## Workflow

1. Fork the repository and create a feature branch off `main`.
2. Make your change. Before pushing, run:
   ```bash
   cargo fmt
   cargo clippy -- -D warnings
   cargo test
   ```
3. Rebase onto the latest `main` and open a pull request.

## Commit messages: Conventional Commits

Every commit follows [Conventional Commits 1.0](https://www.conventionalcommits.org/en/v1.0.0/):

```
<type>(<optional scope>): <short summary>

<optional body: what changed and why>

<optional footer(s)>
```

- **Summary:** imperative mood, lowercase, no trailing period, about 72 characters max (`fix: handle missing audio streams`, not `Fixed missing audio streams.`).
- **Scope** (optional): the area touched, e.g. `ui`, `encoder`, `vmaf`, `queue`, `packaging`.
- **Breaking changes:** add `!` after the type/scope (`feat(config)!: rename profile keys`) and a `BREAKING CHANGE:` footer describing the migration.
- One logical change per commit. Don't mix a refactor with a behavior change.

| Type | Use for |
|------|---------|
| `feat` | A new user-facing feature |
| `fix` | A bug fix |
| `perf` | A performance improvement |
| `refactor` | Code change that neither fixes a bug nor adds a feature |
| `docs` | Documentation only |
| `test` | Adding or fixing tests |
| `build` | Build system, dependencies, packaging |
| `ci` | CI workflows |
| `chore` | Maintenance that fits none of the above |
| `revert` | Reverting an earlier commit |

## Linear history

`main` has a linear history: no merge commits.

- **Rebase, don't merge.** Update your branch with `git pull --rebase` or `git rebase origin/main`, never `git merge main`.
- **Clean up before review.** Squash fixups (`git rebase -i`, `git commit --fixup` + `--autosquash`) so each remaining commit is a self-contained Conventional Commit that builds and passes tests.
- **Force-push your own branch only**, with `git push --force-with-lease`. Never force-push `main`.
- **PRs land by rebase or squash merge.** Multi-commit PRs whose commits are each meaningful are rebased; otherwise the PR is squashed into one commit whose message follows the format above.

Helpful local config:

```bash
git config pull.rebase true
git config rebase.autoSquash true
```
