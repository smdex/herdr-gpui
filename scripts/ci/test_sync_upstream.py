#!/usr/bin/env python3
"""Isolated regression tests for scripts/ci/sync_upstream.py.

Uses only temp local bare "remotes" and real git plumbing; no network,
no real credentials, no checkout of the host project. Run:

    python3 scripts/ci/test_sync_upstream.py -v

Covers: happy path (rebase + three ordered pushes), no-op run, conflict
fail-closed (nothing pushed), crash-after-GitLab-push recovery, crash-
after-GitHub-push checkpoint repair, foreign GitHub tip refusal, fork
commit survival, secret non-leakage (even with GIT_TRACE=1), missing
checkpoint bootstrap failure, and stale-anchor fetch verification.
"""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
import sync_upstream as su  # noqa: E402

SCRIPT = str(Path(__file__).resolve().parent / "sync_upstream.py")
SECRET = "glpat-not-a-real-token-0123456789"


def run_git(*args: str, cwd: str | None = None, check: bool = True) -> str:
    env = dict(os.environ)
    env["GIT_CONFIG_NOSYSTEM"] = "1"
    env["GIT_CONFIG_GLOBAL"] = os.devnull
    proc = subprocess.run(["git", *args], cwd=cwd, env=env,
                          capture_output=True, text=True)
    if check and proc.returncode != 0:
        raise AssertionError(f"git {args} failed: {proc.stderr}")
    return proc.stdout.strip()


class World:
    """Three local bare remotes: upstream, gitlab (with checkpoint), github."""

    def __init__(self, root: Path):
        self.root = root
        self.upstream = self._bare("upstream.git")
        self.gitlab = self._bare("gitlab.git")
        self.github = self._bare("github.git")

    def _bare(self, name: str) -> Path:
        path = self.root / name
        path.mkdir()
        run_git("init", "--quiet", "--bare", str(path))
        run_git("--git-dir", str(path), "symbolic-ref", "HEAD", "refs/heads/main")
        return path

    @property
    def upstream_url(self) -> str:
        return f"file://{self.upstream}"

    @property
    def gitlab_url(self) -> str:
        return f"file://{self.gitlab}"

    @property
    def github_url(self) -> str:
        return f"file://{self.github}"

    def push_new(self, repo: Path, files: dict[str, str], message: str,
                 ref: str = "refs/heads/main") -> str:
        """Clone `repo`, commit `files`, push to it, return the new SHA."""
        work = tempfile.mkdtemp(dir=self.root)
        run_git("clone", "--quiet", str(repo), ".", cwd=work)
        run_git("config", "user.name", "t", cwd=work)
        run_git("config", "user.email", "t@t", cwd=work)
        for rel, content in files.items():
            target = Path(work) / rel
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(content)
            run_git("add", rel, cwd=work)
        run_git("commit", "--quiet", "-m", message, cwd=work)
        sha = run_git("rev-parse", "HEAD", cwd=work)
        run_git("push", "--quiet", "origin", f"HEAD:{ref}", cwd=work)
        return sha

    def ref(self, repo: Path, ref: str = "refs/heads/main") -> str | None:
        out = run_git("--git-dir", str(repo), "rev-parse", "--verify",
                      "--quiet", ref, check=False)
        return out or None

    def set_ref(self, repo: Path, ref: str, sha: str) -> None:
        run_git("--git-dir", str(repo), "update-ref", ref, sha)


class SyncTestCase(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory(prefix="forksync-test-")
        self.world = World(Path(self._tmp.name))
        # upstream: base commit
        self.world.push_new(self.world.upstream, {"f.txt": "base"}, "base")
        # fork commit built on upstream main, pushed to gitlab + github
        work = tempfile.mkdtemp(dir=self.world.root)
        run_git("clone", "--quiet", self.world.upstream_url, ".", cwd=work)
        run_git("config", "user.name", "t", cwd=work)
        run_git("config", "user.email", "t@t", cwd=work)
        (Path(work) / "fork.txt").write_text("fork")
        run_git("add", "fork.txt", cwd=work)
        run_git("commit", "--quiet", "-m", "fork commit", cwd=work)
        self.fork_tip = run_git("rev-parse", "HEAD", cwd=work)
        for url in (self.world.gitlab_url, self.world.github_url):
            run_git("push", "--quiet", url, f"{self.fork_tip}:refs/heads/main", cwd=work)
        self.world.set_ref(self.world.gitlab, su.CHECKPOINT_REF, self.fork_tip)

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def env(self, **over: str) -> dict[str, str]:
        root = Path(self._tmp.name)
        key = root / "dummy_key"
        key.write_text("not a real key\n")
        key.chmod(0o600)
        env = dict(os.environ)
        env.update({
            "GL_TOKEN": SECRET,
            "GL_SERVER_URL": str(root),
            "GL_PROJECT_PATH": "gitlab",
            "DEFAULT_BRANCH": "main",
            "UPSTREAM_URL": self.world.upstream_url,
            "GITHUB_MIRROR_URL": self.world.github_url,
            "GITHUB_SSH_KEY": str(key),
            "GITHUB_KNOWN_HOSTS": os.devnull,
        })
        env.update(over)
        return env

    def run_sync(self, env: dict[str, str] | None = None) -> subprocess.CompletedProcess:
        return subprocess.run(
            [sys.executable, SCRIPT],
            env=env or self.env(),
            capture_output=True, text=True,
        )

    # -- tests ----------------------------------------------------------
    def test_happy_path_rebase_and_ordered_pushes(self) -> None:
        self.world.push_new(self.world.upstream, {"g.txt": "u"}, "up commit")
        proc = self.run_sync()
        self.assertEqual(proc.returncode, 0, proc.stderr)
        gl = self.world.ref(self.world.gitlab)
        self.assertEqual(self.world.ref(self.world.github), gl)
        self.assertEqual(self.world.ref(self.world.gitlab, su.CHECKPOINT_REF), gl)
        # fork content survived the rebase
        out = run_git("--git-dir", str(self.world.gitlab), "show", f"{gl}:fork.txt")
        self.assertEqual(out.strip(), "fork")
        self.assertIn("sync complete", proc.stdout)

    def test_noop_run_pushes_nothing(self) -> None:
        proc = self.run_sync()
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertIn("no rebase needed", proc.stdout)
        self.assertEqual(self.world.ref(self.world.gitlab), self.fork_tip)
        self.assertEqual(self.world.ref(self.world.github), self.fork_tip)

    def test_conflict_fails_closed_nothing_pushed(self) -> None:
        self.world.push_new(self.world.upstream, {"f.txt": "upstream"}, "u2")
        self.world.push_new(self.world.gitlab, {"f.txt": "fork"}, "f2")
        gl_before = self.world.ref(self.world.gitlab)
        proc = self.run_sync()
        self.assertEqual(proc.returncode, 1)
        self.assertIn("refusing to auto-resolve", proc.stderr)
        self.assertEqual(self.world.ref(self.world.gitlab), gl_before)  # untouched
        self.assertEqual(self.world.ref(self.world.github), self.fork_tip)
        self.assertEqual(self.world.ref(self.world.gitlab, su.CHECKPOINT_REF), self.fork_tip)

    def test_crash_after_gitlab_push_recovers(self) -> None:
        # upstream advanced; sync pushes all three; then simulate the
        # crash: GitHub and checkpoint were not updated.
        self.world.push_new(self.world.upstream, {"g.txt": "u"}, "up commit")
        self.assertEqual(self.run_sync().returncode, 0)
        gl = self.world.ref(self.world.gitlab)
        self.world.set_ref(self.world.github, "refs/heads/main", self.fork_tip)
        self.world.set_ref(self.world.gitlab, su.CHECKPOINT_REF, self.fork_tip)
        # rerun: upstream unchanged, but GitHub must be re-mirrored and
        # the checkpoint repaired - even though upstream did not move.
        proc2 = self.run_sync()
        self.assertEqual(proc2.returncode, 0, proc2.stderr)
        self.assertEqual(self.world.ref(self.world.github), gl)
        self.assertEqual(self.world.ref(self.world.gitlab, su.CHECKPOINT_REF), gl)

    def test_crash_after_github_push_checkpoint_repairs(self) -> None:
        self.world.push_new(self.world.upstream, {"g.txt": "u"}, "up commit")
        self.assertEqual(self.run_sync().returncode, 0)
        gl = self.world.ref(self.world.gitlab)
        # only the checkpoint push failed
        self.world.set_ref(self.world.gitlab, su.CHECKPOINT_REF, self.fork_tip)
        proc2 = self.run_sync()
        # github == gitlab main -> guard passes, checkpoint repaired
        self.assertEqual(proc2.returncode, 0, proc2.stderr)
        self.assertEqual(self.world.ref(self.world.gitlab, su.CHECKPOINT_REF), gl)

    def test_foreign_github_tip_refused(self) -> None:
        # GitHub-only commit: gh != checkpoint and gh != gitlab main
        self.world.push_new(self.world.github, {"x.txt": "foreign"}, "foreign")
        proc = self.run_sync()
        self.assertEqual(proc.returncode, 1)
        self.assertIn("diverged", proc.stderr)
        # nothing was overwritten
        self.assertNotEqual(self.world.ref(self.world.github), self.fork_tip)
        self.assertEqual(self.world.ref(self.world.gitlab), self.fork_tip)

    def test_missing_checkpoint_aborts(self) -> None:
        run_git("--git-dir", str(self.world.gitlab), "update-ref", "-d",
                su.CHECKPOINT_REF)
        proc = self.run_sync()
        self.assertEqual(proc.returncode, 1)
        self.assertIn("checkpoint missing", proc.stderr)
        self.assertEqual(self.world.ref(self.world.github), self.fork_tip)

    def test_fork_commit_count_survives_rebase(self) -> None:
        for i in range(3):
            self.world.push_new(self.world.upstream, {f"u{i}.txt": "x"}, f"u{i}")
        proc = self.run_sync()
        self.assertEqual(proc.returncode, 0, proc.stderr)
        gl = self.world.ref(self.world.gitlab)
        up = self.world.ref(self.world.upstream)
        count = run_git("--git-dir", str(self.world.gitlab), "rev-list",
                        "--count", gl, f"^{up}")
        self.assertEqual(count, "1")  # exactly the one fork commit remains

    def test_secret_never_in_output_even_with_git_trace(self) -> None:
        self.world.push_new(self.world.upstream, {"g.txt": "u"}, "up commit")
        proc = self.run_sync(self.env(GIT_TRACE="1"))
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertNotIn(SECRET, proc.stdout)
        self.assertNotIn(SECRET, proc.stderr)

    def test_stale_anchor_fetch_fails_closed(self) -> None:
        # upstream moved after ls-remote: fetch_anchor must detect the
        # fetched ref differs from the stale anchor and abort.
        repo = tempfile.mkdtemp(dir=self._tmp.name)
        run_git("init", "--quiet", "--bare", repo)
        stale = self.world.ref(self.world.upstream)
        self.world.push_new(self.world.upstream, {"later.txt": "y"}, "later")
        with self.assertRaises(su.SyncError) as ctx:
            su.fetch_anchor(su.base_git_env(), repo, self.world.upstream_url,
                            "upstream", "refs/heads/main", stale)
        self.assertIn("moved during fetch", str(ctx.exception))

    def test_ssh_agent_bypassed(self) -> None:
        # Agent forwarding must never be used: base_git_env strips
        # SSH_AUTH_SOCK and GIT_SSH_COMMAND pins IdentityAgent=none,
        # BatchMode, IdentitiesOnly, and the dedicated key.
        env_in = dict(os.environ)
        env_in["SSH_AUTH_SOCK"] = "/tmp/malicious-agent.sock"
        with mock.patch.dict(os.environ, env_in):
            base = su.base_git_env()
        self.assertNotIn("SSH_AUTH_SOCK", base)
        root = Path(self._tmp.name)
        key = root / "k"; key.write_text("k\n")
        ssh = su.ssh_env(str(key), "/dev/null")["GIT_SSH_COMMAND"]
        self.assertIn("IdentityAgent=none", ssh)
        self.assertIn("BatchMode=yes", ssh)
        self.assertIn("IdentitiesOnly=yes", ssh)
        self.assertIn(str(key), ssh)

    def test_midrun_recheck_detects_remote_race(self) -> None:
        # A remote that moved after reconnaissance must be caught by the
        # mid-run recheck before any mirror/marker push happens.
        env = {"x": "y"}
        with mock.patch.object(su, "remote_ref_sha", return_value="deadbeef"):
            with self.assertRaises(su.SyncError) as ctx:
                su.recheck_ref(env, "url", "refs/heads/main", "cafebabe", "GitLab")
        self.assertIn("changed mid-run", str(ctx.exception))
        self.assertIn("safe to rerun", str(ctx.exception))
        # Anchored tip passes silently.
        with mock.patch.object(su, "remote_ref_sha", return_value="cafebabe"):
            su.recheck_ref(env, "url", "refs/heads/main", "cafebabe", "GitLab")

    def test_credential_redaction(self) -> None:
        # URLs with embedded userinfo are redacted in error messages.
        url = "https://oauth2:sekrit@example.com/grp/proj.git"
        self.assertNotIn("sekrit", su.redact(url))
        # Error summaries never include raw git stderr (which may embed
        # secrets, encoded credentials, or arbitrary URLs).
        secret = "glpat-rawleak-0123456789abcdef"
        encoded = "Basic b2F1dGgyOnNla3JpdA=="
        msg = su.summarize_error(("push", "url"), 128, "nonzero exit")
        self.assertNotIn(secret, msg)
        self.assertNotIn(encoded, msg)
        self.assertNotIn("http", msg)  # no URLs at all
        # Trace vars are stripped by prefix, including exotic names.
        env_in = dict(os.environ)
        for var in ("GIT_TRACE", "GIT_TRACE2_EVENT", "GIT_TRACE_PERF",
                    "GIT_TRACE_CONFIG_PARAMS", "GIT_CURL_VERBOSE"):
            env_in[var] = "1"
        with mock.patch.dict(os.environ, env_in):
            base = su.base_git_env()
        for var in ("GIT_TRACE", "GIT_TRACE2_EVENT", "GIT_TRACE_PERF",
                    "GIT_TRACE_CONFIG_PARAMS", "GIT_CURL_VERBOSE"):
            self.assertNotIn(var, base)

    def test_docs_document_bootstrap_and_checkpoint(self) -> None:
        docs = (Path(__file__).resolve().parent.parent.parent
                / "docs" / "fork-sync.md").read_text()
        self.assertIn("maintenance/github-main", docs)
        self.assertIn("refs/heads/maintenance/github-main", docs)
        self.assertIn("GL_TOKEN", docs)
        self.assertIn("GITHUB_SSH_KEY", docs)

    def test_noop_race_full_orchestration_detected(self) -> None:
        # Fully-noop world (gh==gl==checkpoint); a remote moves AFTER the
        # script's reconnaissance. The unconditional final rechecks must
        # catch it and abort - end to end, not just at the helper.
        env = self.env()
        real_remote_ref_sha = su.remote_ref_sha
        state = {"calls": 0, "new_tip": None}

        def racing_ls_remote(env2, url, ref):
            calls = state["calls"]
            state["calls"] += 1
            # Reconnaissance reads four anchors. Advance the actual bare
            # GitLab remote just before the first post-fetch recheck.
            if calls == 4:
                state["new_tip"] = self.world.push_new(
                    self.world.gitlab, {"concurrent.txt": "preserve me"},
                    "concurrent fork commit",
                )
            return real_remote_ref_sha(env2, url, ref)

        with mock.patch.dict(os.environ, env):
            with mock.patch.object(su, "remote_ref_sha",
                                   side_effect=racing_ls_remote):
                with self.assertRaisesRegex(su.SyncError, "changed mid-run"):
                    su.do_sync()
        # Preserve the concurrent actor's commit and leave the mirror alone.
        self.assertIsNotNone(state["new_tip"])
        self.assertEqual(self.world.ref(self.world.gitlab), state["new_tip"])
        self.assertEqual(self.world.ref(self.world.github), self.fork_tip)
        self.assertEqual(self.world.ref(self.world.gitlab, su.CHECKPOINT_REF),
                         self.fork_tip)

    def test_checkpoint_race_gl_moves_before_marker_push(self) -> None:
        # Disjoint from the noop test: upstream advanced, so GL and GH
        # pushes happen; GitLab main is moved by a concurrent actor AFTER
        # the mirror push but BEFORE the checkpoint push. The pre-checkpoint
        # GitLab recheck must abort and preserve the concurrent commit.
        self.world.push_new(self.world.upstream, {"g.txt": "u"}, "up commit")
        env = self.env()
        real_remote_ref_sha = su.remote_ref_sha
        state = {"calls": 0, "new_tip": None}

        def racing_ls_remote(env2, url, ref):
            calls = state["calls"]
            state["calls"] += 1
            # Reconnaissance(4) + fetch_anchor calls + pre-mirror rechecks
            # precede the checkpoint phase; move GL main on the first call
            # after the mirror push landed (GitHub main == new GL tip now).
            if (calls > 6 and state["new_tip"] is None
                    and "gitlab" in url and ref == "refs/heads/main"):
                state["new_tip"] = self.world.push_new(
                    self.world.gitlab, {"concurrent.txt": "preserve"},
                    "concurrent commit",
                )
            return real_remote_ref_sha(env2, url, ref)

        with mock.patch.dict(os.environ, env):
            with mock.patch.object(su, "remote_ref_sha",
                                   side_effect=racing_ls_remote):
                with self.assertRaisesRegex(su.SyncError, "changed mid-run"):
                    su.do_sync()
        # Concurrent commit preserved; GitHub kept the mirrored tip;
        # checkpoint never advanced.
        self.assertIsNotNone(state["new_tip"])
        self.assertEqual(self.world.ref(self.world.gitlab), state["new_tip"])
        self.assertNotEqual(self.world.ref(self.world.github), self.fork_tip)
        self.assertEqual(self.world.ref(self.world.gitlab, su.CHECKPOINT_REF),
                         self.fork_tip)


if __name__ == "__main__":
    unittest.main()
