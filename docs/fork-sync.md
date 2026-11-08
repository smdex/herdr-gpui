# Scheduled upstream sync and GitHub mirror

GitLab project `smaximov/herdr-gpui` on `git.fila.lab` is the source of
truth for fork `main`. Its pipeline rebases onto
`https://github.com/sheprd-labs/sheprd.git` (formerly `penso/herdr-gpui`)
and publishes the result to `smdex/herdr-gpui` on GitHub. Other branches
and tags are not mirrored.

## Installed configuration

- Project: **318**, protected default branch `main`.
- Schedule: **8**, `17 0,12 * * *`, `Etc/UTC`, targeting `main`.
  Runs at **00:17 and 12:17 UTC**. Enable it only after acceptance passes.
- Runner: **98**, existing shared Incus runner, tag `usa-incus-autoscaler`.
  Jobs need Python 3, Git, and OpenSSH, not a Docker executor or Nix.
- Both `main` and `maintenance/github-main` are protected on GitLab,
  with Maintainer push access and force pushes enabled for rebasing.
- GitLab token `upstream-sync`: project-scoped Maintainer bot,
  `write_repository` only, expires **November 1, 2027**. Rotate it before
  that date and replace `GL_TOKEN` without exposing its value in logs.
- GitHub authentication uses a dedicated repository write deploy key,
  not the operator's account-wide token. Host keys are pinned from
  GitHub's authenticated `/meta` API response.

All three CI variables are **protected**, with environment scope
`fork-maintenance`:

| Variable | Type | Purpose |
| --- | --- | --- |
| `GL_TOKEN` | Masked environment variable | GitLab reads and leased pushes |
| `GITHUB_SSH_KEY` | File variable | Dedicated GitHub private deploy key |
| `GITHUB_KNOWN_HOSTS` | File variable | Pinned GitHub SSH host keys |

SSH-agent use is explicitly disabled: jobs unset `SSH_AUTH_SOCK`, and
Git subprocesses remove it from their environment. GitHub SSH uses
`IdentityAgent=none`, `IdentitiesOnly=yes`, `BatchMode=yes`, a dedicated
key file, and strict host-key checking. No agent forwarding is needed.
GitLab credentials are supplied through an environment-only HTTP header,
never a URL or command argument. Git tracing and raw Git error output
are suppressed to avoid credential disclosure.

## Safety and recovery

The pipeline accepts scheduled, manual web, and API runs only on the
protected default branch. An isolated local-Git regression suite runs
before the serialized `fork-sync` job (`resource_group: fork-maintenance`).
Both jobs have a 15-minute timeout.

The sync reads GitLab, upstream, GitHub, and checkpoint tips, fetches only
the required refs into a temporary repository, and verifies the fetched
SHAs against those observations. Hooks and external Git transports are
disabled. No code from the fetched checkout is executed.

`maintenance/github-main` records the **last verified GitHub mirror**,
not a guarantee that the two hosts always remain equal between runs.
An unexpected GitHub tip is refused unless it equals GitLab `main`,
which also permits recovery after a successful mirror whose checkpoint
update failed. Never reset this checkpoint just to silence a divergence.

A conflict-free rebase must retain the fork-only commit count and contain
the fetched upstream tip. Conflicts, dropped commits, and unexpected
remote changes abort rather than receive an automatic resolution.
Resolve these manually in an isolated checkout, validate, publish to
GitLab, and rerun. When upstream independently incorporates a fork patch,
`--reapply-cherry-picks --empty=keep` retains that fork commit explicitly
as empty. This preserves history without weakening the commit-count check.

Push order is GitLab `main`, GitHub `main`, then the checkpoint, each
using an explicit expected-SHA `--force-with-lease`. Remote tips are
rechecked around mirroring and before success, including no-op runs.
**Cross-host updates are not atomic.** A detected race or failed push
fails the job. Rerunning repairs a partial mirror when its checkpoint
still explains GitHub's tip, even if upstream has not advanced.

## Bootstrap and operation

The checkpoint was initially created at the verified shared tip
`cf76dc2fcc5f584ecf14f66c3d386e58528891fa` on November 8, 2026.
For a new installation, verify the current tips first, substitute the
verified full GitHub SHA below, and select an explicit GitLab key:

```sh
env -u SSH_AUTH_SOCK git \
  -c core.sshCommand='ssh -o IdentityAgent=none -o IdentitiesOnly=yes -o BatchMode=yes -i /path/to/gitlab-key' \
  push ssh://git@git.fila.lab/smaximov/herdr-gpui.git \
  cf76dc2fcc5f584ecf14f66c3d386e58528891fa:refs/heads/maintenance/github-main
```

Do not run this bootstrap again against an existing checkpoint. Protected
branch configuration and the three scoped variables must be in place
before enabling the schedule. Trigger an acceptance pipeline manually:

```sh
env -u SSH_AUTH_SOCK glab api --hostname git.fila.lab \
  projects/318/pipeline --method POST --field ref=main
```

With the same CI variables available, the script's `status` mode prints
remote tips without pushing. To test locally without credentials or
network access:

```sh
env -u SSH_AUTH_SOCK PYTHONDONTWRITEBYTECODE=1 \
  python3 scripts/ci/test_sync_upstream.py -v
```

The suite uses temporary bare remotes to exercise real rebases, fork
preservation, conflicts, partial-push recovery, unexpected GitHub commits,
stale anchors, full-orchestration no-op races, SSH-agent isolation, and
credential-safe diagnostics. Native UI checks are separate from this
maintenance pipeline.
