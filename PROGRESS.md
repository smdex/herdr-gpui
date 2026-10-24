# Progress journal: upstream rebase + Nix companion packaging

Portable record of the work; no machine-local paths beyond this checkout.
All timestamps local to this machine.

## Checkpoint BEFORE fetch/rebase (recorded 2026-10-27, ~01:30)

Repository: /home/smaximov/src/herdr-gpui (jj colocated with git)

- Remotes:
  - origin  = git@github.com:smdex/herdr-gpui (user fork)
  - upstream = https://github.com/penso/herdr-gpui (authoritative)
- jj state at checkpoint:
  - `@` (working copy) = change `wxprsokswslt`, commit `28c32f8baf6e5d9396f33e83d3fb004cbfad9ca8`, empty, no description
  - `@-` = change `motuuznwkrzp`, commit `e3d418282f5429da0049f6d0171e67a615a2118f` — "feat(chrome): customize status and menu bars with Wayland appearance support" (71 files; includes Cargo.toml gpui git patches, Cargo.lock entries, flake.nix cargoLock outputHashes)
  - local `main` bookmark = `qnrmlkltxlqm` commit `ef0df293540cbb6543650d13aaacb97467f7b139` ("test(credentials): run the Credential Manager tests one at a time (#810)") — tracks penso upstream main
  - main@origin (fork) is older: `0ca50aa964eab2420ce10bae47dbcfe4ea2f5125`
  - Working copy clean ("The working copy has no changes.")
- User work to preserve (all inside the single commit `motuuznw`):
  - feature: chrome/status/menu bar customization with Wayland appearance
  - Cargo.toml `[patch.crates-io]`: `gpui-pre` and `gpui-pre-linux` pinned to
    git https://github.com/smdex/herdr-gpui-platform-backport rev `d4c5aa52664dfb5e75789101e50a67d0aa187bc8`
    ("Reviewed platform backport; patch both packages to keep one GPUI core identity")
  - flake.nix: `cargoLock.outputHashes."gpui-pre-0.3.6" = "sha256-s6zLgvhRJHDGIuoxbB4giMQ1a8g9mxi4p0SqTgjKakM="`
- No reset/discard/force-push; rebase rewrites of the feature commit are authorized.

## Environment notes

- jj 0.45.1; nix (Determinate 3.23.1 / nix 2.35.2); no herdr-gpui-family binaries
  installed in any profile yet (only the `herdr` daemon via home-manager) — so
  "existing wrappers/symlinks" concerns Nix layout conventions (store symlinks,
  possible `.*-wrapped` wrappers), not a previously installed package.
- Disk: root 79G free; /tmp tmpfs 5.2G free (Nix builds run in the store, fine).

## Step 1: fetch upstream — IN PROGRESS
