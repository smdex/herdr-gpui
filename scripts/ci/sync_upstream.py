#!/usr/bin/env python3
"""Fail-closed upstream rebase + GitHub mirror for smdex/herdr-gpui.

Run modes (argv[1]):
  sync     Default. Rebase fork main onto upstream and mirror to GitHub.
  status   Read-only: print observed tips and exit (diagnostics).

sync chain (schedule/web/api trigger on the protected default branch
only; the whole job is serialized by a GitLab resource group):

  1. Read-only reconnaissance of upstream, GitLab, and GitHub tips plus
     the `maintenance/github-main` checkpoint via ls-remote. Those SHAs
     are the anchors for every later fetch and push; fetched refs are
     verified to match before any local operation.
  2. Fail closed unless the observed GitHub tip equals the checkpoint or
     equals the current GitLab tip (crash-recovery case). A foreign tip
     is never used as a force-with-lease anchor; GitHub-only commits are
     preserved by refusing to mirror over them.
  3. Rebase the fork tip onto the upstream tip (skipped when the fork
     already contains upstream). Any conflict aborts and fails the job:
     no automatic conflict resolution. The number of fork-only commits
     before and after the rebase must be identical, so the fork stack
     can never be silently dropped.
  4. Push GitLab main, then GitHub main, then advance the checkpoint on
     GitLab - each a single SHA-anchored force-with-lease push anchored
     on the originally observed ref. If the GitLab push succeeded but
     GitHub failed, the next run re-mirrors (GitHub tip still equals the
     checkpoint) and repairs the state, even when upstream is unchanged.

No repository code is ever executed: objects are fetched into a
throwaway bare repo with hooks disabled, system/global config ignored,
ext:// protocol forbidden, and tracing disabled so credentials cannot
leak into logs. Git error output is summarized, never dumped raw.

Credentials: GL_TOKEN travels only as an http.extraHeader set via git
environment config (never in a URL, argv, or logs). GitHub uses a
dedicated write deploy SSH key (chmod 600 temp copy) with strict host
checking (BatchMode, IdentitiesOnly) against a pinned known_hosts file.

Only the Python standard library is used; runs under plain python3.

Environment:
  GL_TOKEN             GitLab project access token (write_repository)
  GL_SERVER_URL        e.g. https://git.fila.lab (default CI_SERVER_URL)
  GL_PROJECT_PATH      e.g. smaximov/herdr-gpui (default CI_PROJECT_PATH)
  DEFAULT_BRANCH       default main
  UPSTREAM_URL         default https://github.com/sheprd-labs/sheprd.git
  UPSTREAM_REF         default main
  GITHUB_MIRROR_URL    default git@github.com:smdex/herdr-gpui.git
  GITHUB_SSH_KEY       path to (GitLab FILE variable) private deploy key
  GITHUB_KNOWN_HOSTS   path to (GitLab FILE variable) pinned host keys
  CI_BOT_NAME          default herdr-sync-bot
  CI_BOT_EMAIL         default herdr-sync-bot@users.noreply.gitlab.local
"""

from __future__ import annotations

import base64
import os
import shlex
import shutil
import stat
import subprocess
import sys
import tempfile
from urllib.parse import urlsplit

CHECKPOINT_REF = "refs/heads/maintenance/github-main"


class SyncError(RuntimeError):
    """Deliberate, fail-closed abort with a sanitized message."""


def summarize_error(args: tuple[str, ...], returncode: int, kind: str) -> str:
    """Sanitized failure line; NO raw git stderr (may embed secrets/URLs)."""
    cmd = " ".join(args[:2])
    return f"git {cmd} failed ({kind}, exit {returncode})"


def base_git_env() -> dict[str, str]:
    """Hostile-repo-hardened git environment: no hooks, no config, no trace."""
    env = dict(os.environ)
    env["GIT_CONFIG_NOSYSTEM"] = "1"
    env["GIT_CONFIG_GLOBAL"] = os.devnull
    # /dev/null as hooks path on Linux disables every hook category.
    env["GIT_CONFIG_COUNT"] = "3"
    env["GIT_CONFIG_KEY_0"] = "core.hooksPath"
    env["GIT_CONFIG_VALUE_0"] = os.devnull
    env["GIT_CONFIG_KEY_1"] = "protocol.ext.allow"
    env["GIT_CONFIG_VALUE_1"] = "never"
    env["GIT_CONFIG_KEY_2"] = "protocol.file.allow"
    env["GIT_CONFIG_VALUE_2"] = "always"  # local worktree ops only
    # Strip every trace/verbosity var: any GIT_TRACE* prefix plus curl.
    for var in [k for k in env if k.startswith("GIT_TRACE")]:
        env.pop(var, None)
    env.pop("GIT_CURL_VERBOSE", None)
    env.pop("GIT_CONFIG_PARAMETERS", None)
    for var in ("GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_OBJECT_DIRECTORY"):
        env.pop(var, None)
    # Never use an agent-forwarded identity inherited from the runner.
    env.pop("SSH_AUTH_SOCK", None)
    return env


def git(env: dict[str, str], *args: str, check: bool = True) -> subprocess.CompletedProcess:
    try:
        proc = subprocess.run(
            ["git", "--no-pager", *args],
            env=env,
            capture_output=True,
            text=True,
            timeout=600,
        )
    except subprocess.TimeoutExpired:
        raise SyncError(
            summarize_error(args, -1, "timeout after 600s")
        ) from None
    if check and proc.returncode != 0:
        raise SyncError(summarize_error(args, proc.returncode, "nonzero exit"))
    return proc


def gitlab_env(token: str, server: str) -> dict[str, str]:
    """GitLab auth: token only via http.extraHeader env config."""
    auth = base64.b64encode(f"oauth2:{token}".encode()).decode()
    env = base_git_env()
    env["GIT_CONFIG_COUNT"] = "4"
    env["GIT_CONFIG_KEY_3"] = f"http.{server}.extraHeader"
    env["GIT_CONFIG_VALUE_3"] = f"Authorization: Basic {auth}"
    return env


def ssh_env(key_path: str, known_hosts: str) -> dict[str, str]:
    env = base_git_env()
    env["GIT_SSH_COMMAND"] = (
        "ssh -o BatchMode=yes -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes"
        " -o IdentityAgent=none"
        f" -o UserKnownHostsFile={shlex.quote(known_hosts)}"
        f" -i {shlex.quote(key_path)}"
    )
    return env


def require_env(name: str) -> str:
    value = os.environ.get(name, "").strip()
    if not value:
        raise SyncError(f"{name} is not set; refusing to run")
    return value


def redact(url: str) -> str:
    parts = urlsplit(url)
    if parts.username or parts.password:
        return parts._replace(netloc=parts.hostname or "").geturl()
    return url


def remote_ref_sha(env: dict[str, str], url: str, ref: str) -> str | None:
    """Read one remote ref via ls-remote; no clone, no code execution."""
    proc = git(env, "ls-remote", url, ref, check=False)
    if proc.returncode != 0:
        raise SyncError(f"ls-remote failed for {redact(url)} {ref}")
    for line in proc.stdout.splitlines():
        sha, _, r = line.partition("\t")
        if r.strip() == ref:
            return sha.strip()
    return None


def fetch_anchor(env: dict[str, str], repo_dir: str, url: str,
                 remote: str, ref: str, anchor: str) -> str:
    """Fetch exactly one ref and verify it landed on the ls-remote anchor.

    Only the specific refspec is fetched (never the branch universe), and
    the fetched local ref must equal the anchor SHA or we fail closed:
    the remote moved between recon and fetch, so anchors are stale.
    """
    local = f"refs/sync/{remote}/{ref[len('refs/heads/'):] if ref.startswith('refs/heads/') else ref}"
    git(env, "--git-dir", repo_dir, "fetch", "--quiet", "--no-tags", "--force",
        url, f"+{ref}:{local}")
    got = git(env, "--git-dir", repo_dir, "rev-parse", "--verify",
              f"{local}^{{commit}}").stdout.strip()
    if got != anchor:
        raise SyncError(
            f"{remote} {ref} moved during fetch ({anchor[:12]} -> {got[:12]}); "
            "anchors stale, aborting"
        )
    return local


def is_ancestor(env: dict[str, str], repo_dir: str, anc: str, desc: str) -> bool:
    proc = git(env, "--git-dir", repo_dir, "merge-base", "--is-ancestor",
               anc, desc, check=False)
    return proc.returncode == 0


def commit_count(env: dict[str, str], repo_dir: str, rev: str, *exclude: str) -> int:
    args = ["--git-dir", repo_dir, "rev-list", "--count", rev]
    args += [f"^{e}" for e in exclude]
    out = git(env, *args).stdout.strip()
    return int(out)


def recheck_ref(env: dict[str, str], url: str, ref: str, expected: str,
                label: str) -> None:
    """Re-observe a remote ref mid-run; abort if it left the anchor.

    Closes the no-op race: a remote that moved between the original
    reconnaissance and the mirror/marker pushes is detected explicitly
    before any further push, instead of relying only on the lease.
    """
    got = remote_ref_sha(env, url, ref)
    if got != expected:
        raise SyncError(
            f"{label} {ref} changed mid-run "
            f"({expected[:12]} -> {got and got[:12]}); safe to rerun"
        )


def push_leased(env: dict[str, str], repo_dir: str, url: str,
                ref: str, anchor: str, new: str, label: str,
                probe_env: dict[str, str]) -> None:
    """SHA-anchored force-with-lease push with race classification.

    A rejected lease is expected when the remote moved between
    reconnaissance and push; re-probe the ref and report it as a
    retryable race instead of a generic git failure.
    """
    try:
        git(env, "--git-dir", repo_dir, "push", url,
            f"--force-with-lease={ref}:{anchor}", f"{new}:{ref}")
    except SyncError:
        moved = remote_ref_sha(probe_env, url, ref)
        if moved is not None and moved != anchor:
            raise SyncError(
                f"race on {label} {ref}: remote moved {anchor[:12]} -> "
                f"{moved[:12]} during this run; safe to rerun the pipeline"
            ) from None
        raise


def do_sync() -> int:
    token = require_env("GL_TOKEN")
    server = require_env("GL_SERVER_URL").rstrip("/")
    project = require_env("GL_PROJECT_PATH")
    branch = os.environ.get("DEFAULT_BRANCH", "main")
    upstream_url = os.environ.get("UPSTREAM_URL", "https://github.com/sheprd-labs/sheprd.git")
    upstream_ref = os.environ.get("UPSTREAM_REF", "main")
    github_url = os.environ.get("GITHUB_MIRROR_URL", "git@github.com:smdex/herdr-gpui.git")
    key_path = require_env("GITHUB_SSH_KEY")
    known_hosts = require_env("GITHUB_KNOWN_HOSTS")
    os.chmod(key_path, stat.S_IRUSR | stat.S_IWUSR)

    gl_url = f"{server}/{project}.git"
    branch_ref = f"refs/heads/{branch}"
    up_branch_ref = f"refs/heads/{upstream_ref}"

    gl_env = gitlab_env(token, server)   # authenticated: private project
    gh_env = ssh_env(key_path, known_hosts)
    plain = base_git_env()

    # Reconnaissance: all anchors in one pass.
    gl_tip = remote_ref_sha(gl_env, gl_url, branch_ref)
    up_tip = remote_ref_sha(plain, upstream_url, up_branch_ref)
    gh_tip = remote_ref_sha(gh_env, github_url, branch_ref)
    checkpoint = remote_ref_sha(gl_env, gl_url, CHECKPOINT_REF)

    print(f"observed: gitlab={gl_tip and gl_tip[:12]} upstream={up_tip and up_tip[:12]}"
          f" github={gh_tip and gh_tip[:12]} checkpoint={checkpoint and checkpoint[:12]}",
          flush=True)

    if gl_tip is None:
        raise SyncError(f"GitLab {branch} not found")
    if up_tip is None:
        raise SyncError(f"upstream {upstream_ref} not found")
    if gh_tip is None:
        raise SyncError(f"GitHub {branch} not found")
    if checkpoint is None:
        raise SyncError(
            f"{CHECKPOINT_REF} checkpoint missing; bootstrap it to the verified "
            "current GitHub main tip before enabling the schedule"
        )
    # Never trust a freshly observed, unexplained GitHub tip as a lease
    # anchor; GitHub-only commits are preserved by refusing to proceed.
    if gh_tip != checkpoint and gh_tip != gl_tip:
        raise SyncError(
            "GitHub main diverged from checkpoint and GitLab main "
            "(GitHub-only commits?); refusing to mirror. Reconcile manually "
            "and update the checkpoint deliberately."
        )

    repo_dir = work_dir = ""
    try:
        repo_dir = tempfile.mkdtemp(prefix="forksync-repo-")
        work_dir = tempfile.mkdtemp(prefix="forksync-work-")
        git(plain, "init", "--quiet", "--bare", repo_dir)
        git(plain, "--git-dir", repo_dir, "config", "transfer.fsckObjects", "true")

        # Fetch exactly the needed refs, verify each against its anchor.
        gl_main = fetch_anchor(gl_env, repo_dir, gl_url, "gitlab", branch_ref, gl_tip)
        up_main = fetch_anchor(plain, repo_dir, upstream_url, "upstream", up_branch_ref, up_tip)
        fetch_anchor(gl_env, repo_dir, gl_url, "gitlab", CHECKPOINT_REF, checkpoint)

        # Detached work tree on the fork tip (never a checkout of upstream
        # and never an existing clone).
        git(plain, "--git-dir", repo_dir, "worktree", "add", "--quiet",
            "--detach", work_dir, gl_main)
        git(plain, "-C", work_dir, "config", "user.name",
            os.environ.get("CI_BOT_NAME", "herdr-sync-bot"))
        git(plain, "-C", work_dir, "config", "user.email",
            os.environ.get("CI_BOT_EMAIL", "herdr-sync-bot@users.noreply.gitlab.local"))

        if is_ancestor(plain, repo_dir, up_main, gl_main):
            new_tip = gl_tip
            print("fork main already contains upstream; no rebase needed", flush=True)
        else:
            # Fork-only commits must survive the rebase one-for-one.
            before = commit_count(plain, repo_dir, gl_main, up_main)
            proc = git(plain, "-C", work_dir, "rebase", up_main, check=False)
            if proc.returncode != 0:
                git(plain, "-C", work_dir, "rebase", "--abort", check=False)
                raise SyncError(
                    "rebase conflict or failure; refusing to auto-resolve. "
                    "Resolve manually and rerun."
                )
            new_tip = git(plain, "-C", work_dir, "rev-parse", "HEAD").stdout.strip()
            if not is_ancestor(plain, repo_dir, up_tip, new_tip):
                raise SyncError("rebase result does not contain upstream tip; aborting")
            after = commit_count(plain, repo_dir, new_tip, up_tip)
            if after != before:
                raise SyncError(
                    f"rebase dropped or duplicated fork commits "
                    f"({before} -> {after}); aborting"
                )
            print(f"rebased {before} fork commits onto {up_tip[:12]} "
                  f"-> {new_tip[:12]}", flush=True)

        # 1) GitLab main (SHA-anchored force-with-lease on recon tip).
        if new_tip != gl_tip:
            push_leased(gl_env, repo_dir, gl_url, branch_ref, gl_tip,
                        new_tip, "GitLab", gl_env)
            print(f"gitlab {branch}: {gl_tip[:12]} -> {new_tip[:12]}", flush=True)

        # 2) GitHub main (SHA-anchored force-with-lease on verified anchor).
        # Unconditional pre-mirror rechecks: GitLab must hold new_tip and
        # GitHub must still hold its observed anchor, even on a fully-noop
        # run where no push would otherwise occur.
        recheck_ref(gl_env, gl_url, branch_ref, new_tip, "GitLab")
        recheck_ref(gh_env, github_url, branch_ref, gh_tip, "GitHub")
        if gh_tip != new_tip:
            push_leased(gh_env, repo_dir, github_url, branch_ref, gh_tip,
                        new_tip, "GitHub", gh_env)
            print(f"github {branch}: {gh_tip[:12]} -> {new_tip[:12]}", flush=True)

        # 3) Advance checkpoint to the mirrored tip (SHA-anchored lease).
        # Recheck that the GitHub push landed exactly on the intended tip
        # before recording it as the checkpoint.
        if checkpoint != new_tip:
            recheck_ref(gh_env, github_url, branch_ref, new_tip, "GitHub")
            recheck_ref(gl_env, gl_url, branch_ref, new_tip, "GitLab")
            push_leased(gl_env, repo_dir, gl_url, CHECKPOINT_REF, checkpoint,
                        new_tip, "checkpoint", gl_env)
            print(f"checkpoint: {checkpoint[:12]} -> {new_tip[:12]}", flush=True)

        # Final invariant rechecks before declaring success: all three
        # refs must hold exactly new_tip (unconditional, covers no-op runs
        # and any remote movement after the last push/recheck).
        recheck_ref(gl_env, gl_url, branch_ref, new_tip, "GitLab")
        recheck_ref(gh_env, github_url, branch_ref, new_tip, "GitHub")
        recheck_ref(gl_env, gl_url, CHECKPOINT_REF, new_tip, "checkpoint")

        print("sync complete", flush=True)
        return 0
    finally:
        if repo_dir:
            shutil.rmtree(repo_dir, ignore_errors=True)
        if work_dir:
            shutil.rmtree(work_dir, ignore_errors=True)


def do_status() -> int:
    server = require_env("GL_SERVER_URL").rstrip("/")
    project = require_env("GL_PROJECT_PATH")
    branch = os.environ.get("DEFAULT_BRANCH", "main")
    upstream_url = os.environ.get("UPSTREAM_URL", "https://github.com/sheprd-labs/sheprd.git")
    upstream_ref = os.environ.get("UPSTREAM_REF", "main")
    github_url = os.environ.get("GITHUB_MIRROR_URL", "git@github.com:smdex/herdr-gpui.git")
    gl_url = f"{server}/{project}.git"
    branch_ref = f"refs/heads/{branch}"
    gl_env = gitlab_env(require_env("GL_TOKEN"), server)
    gh_env = ssh_env(require_env("GITHUB_SSH_KEY"), require_env("GITHUB_KNOWN_HOSTS"))
    print("gitlab:", remote_ref_sha(gl_env, gl_url, branch_ref))
    print("checkpoint:", remote_ref_sha(gl_env, gl_url, CHECKPOINT_REF))
    print("upstream:", remote_ref_sha(base_git_env(), upstream_url, f"refs/heads/{upstream_ref}"))
    print("github:", remote_ref_sha(gh_env, github_url, branch_ref))
    return 0


def main() -> int:
    mode = sys.argv[1] if len(sys.argv) > 1 else "sync"
    try:
        if mode == "sync":
            return do_sync()
        if mode == "status":
            return do_status()
        print(f"unknown mode {mode!r}; use sync|status", file=sys.stderr)
        return 2
    except SyncError as error:
        print(f"SYNC ABORTED: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
