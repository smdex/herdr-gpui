# Fork revision, November 13, 2026

## Baseline and disposition

The local `sync/upstream-20261110` branch was stale. A branch-only fetch found
GitHub `origin/main` and GitLab `main` both at
`51fc1fcbbeb8d283e2cf2afe87a0dbde1b2af2f1`, already rebased by scheduled
maintenance onto upstream
`ec2ebf0e186da908a949cf8109e5cb4a56dc3c10`. The published stack, not the stale
checkout, is the baseline for this revision. No upstream commits needed replaying.
Existing tags with divergent local identities were not overwritten.

| Fork change | Disposition |
| --- | --- |
| Menu visibility, persistent and temporary commands, focus/input isolation | Retain. Upstream's temporary toggle does not replace the fork's persistent saved preference. |
| Status-bar visibility and Keep Awake lifetime | Retain. These remain fork functionality layered on existing preference and worker paths. |
| Wayland client-decoration request | Retain as a default-off request for new main windows, using upstream GPUI and observed decorations. |
| Former GPUI platform backport and capability diagnostics | Obsolete dependency assumption. Do not restore the private pin. Correct stale help/docs, retain the additive control-wire field as empty when support cannot be established. |
| Standalone Nix companions and legacy names | Retain, repair installation on every declared platform without retaining the GUI runtime output. |
| Command/config extraction from older snapshots | Already superseded in the published stack by current upstream organization. Do not resurrect stale split files. |
| Localization/test compatibility follow-ups | Retain current upstream-shaped code, not historical patch implementations. |
| Scheduled rebase/mirror and history-retention guard | Retain fail-closed behavior, isolated Git tests and explicit mirror checkpoint. |

## Regressions repaired

- Restore upstream Azure DevOps account attention and sign-in routing removed
  by an older titlebar carry patch, preserving the fork's window-control layout.
- Restore the Wayland client-decoration preference in General settings after
  upstream's settings-page migration, using the existing typed save path.
- Populate standalone companion outputs on Darwin as well as Linux. Keep
  installation policy shared rather than adding one repair per package name.
- Restore overlay composition with plain consumer Nixpkgs: obtain the pinned
  Rust toolchain through a local rust-overlay extension rather than assuming
  the consumer already imported that overlay.
- Restore the upstream agent-group CLI regression alongside the fork's wire
  compatibility test, and update a hidden-menu fixture to set both the saved
  preference and current per-window visibility after their intentional split.
- Give the fork its own validation workflow. Upstream's repository/actor gates
  intentionally skip this fork and are left unchanged. Every push to `main`
  must validate even when scheduled maintenance changes only source files.
- Use `20261113.1-smdex.1` as the Nix package version and
  `v20261113.1-smdex.1` as the immutable source tag. Do not change Cargo's
  workspace version or upstream updater's calendar-version parser. Nix builds
  still embed no update key and must not replace themselves.

## Forward-update policy

Keep upstream dependency/toolchain pins and protocol contracts. Prefer small
additive fork files over editing upstream CI's security gates. Rebase conflicts
and remote races still abort maintenance rather than receiving guessed fixes.
A compile-clean rebase is not proof of behavioral equivalence: review deleted
upstream behavior, run topic regressions, and build the resulting Nix outputs.
Published tags are immutable snapshots even when scheduled maintenance rewrites
`main`; pin the tag or its full commit SHA in `llm-agents.nix`.

## Verification

Native GUI/compositor and Darwin/Windows execution require their own platforms.
No active desktop is available in this Linux agent session. The companion
install-script regression is synthetic layout verification, not a Darwin build.

Passed locally with the pinned Nix toolchain (`nix develop .#sheprd`):

- `cargo test --locked -p sheprd-commands`: 21 tests.
- `cargo test --locked -p sheprd-gpui --lib menus::in_window::tests`: 32 tests.
- `cargo test --locked -p sheprd-gpui --lib orchestrator::view::tests::azure_titlebar`:
  one test of the actual attention dot and account-button click routing.
- `cargo test --locked -p sheprd-gpui --lib app::wayland_decorations`: two tests.
- `cargo test --locked -p sheprd-gpui --lib config::tests::wayland_decorations`:
  one test.
- `cargo test --locked -p sheprd-gpui --lib config::tests::status_bar_visibility`:
  three tests.
- `cargo test --locked -p sheprd-gpui --lib control::client::app::tests`: two tests.
- `cargo check --locked -p sheprd-gpui --all-features`: optional Moonframe and
  integration-test paths compile. This is not an all-feature test run.

Other local checks:

- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/ci/test_sync_upstream.py -q`:
  17 isolated Git regression tests passed.
- `bash scripts/check-nix-companions.sh`: actual evaluated Linux and Darwin
  installation scripts passed against distinct synthetic binaries. The same
  harness against pre-fix `51fc1fcbb` failed at the missing Darwin ACP output,
  as expected. It cleans temporary files on both outcomes.
- `nix flake check --no-build --all-systems`: all declared platforms evaluated,
  no platform build claimed.
- Plain Nixpkgs with only `overlays.default` evaluates `sheprd.drvPath`, without
  a separately imported Rust overlay.
- File-size, sorted-region, localization, changed-file Rust formatting, Nix
  formatting, shell syntax, and `git diff --check` gates passed.

Full Nix release builds and real companion runtime-closure checks are delegated
to the new fork workflow. Full workspace tests, Windows/Darwin builds and native
GUI/compositor QA were not run locally. The fork workflow builds Linux only,
so native Darwin and Windows verification remains separate outstanding work.
No signed upstream-style release or daemon installation is part of this revision.
