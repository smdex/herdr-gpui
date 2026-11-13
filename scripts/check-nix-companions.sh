#!/usr/bin/env bash
# Regression for the flake's companion outputs (acp, remoteControl, automate).
#
# Evaluates the package's full postInstall script for the Linux and Darwin
# flavors, then executes it in a sandbox against stub executables. Asserts
# each companion output receives a regular-file copy of its program (never a
# symlink into the GUI bundle, which would drag the GUI runtime closure into a
# standalone companion install) plus the legacy herdr-* alias and the
# remoteControl -> acp sibling link.
#
# Fails against the pre-fix flake, where the Darwin branch never populated
# those outputs.
#
# Repository files are never mutated: the script runs in a temporary cwd
# holding copies of the resource files the Darwin branch installs, and only
# the immutable /nix/store/... input paths for sheprd-nearby and herdr-core
# are substituted with synthetic stubs.
set -euo pipefail
cd "$(dirname "$0")/.."
REPO=$PWD
# Optional flake reference to test; defaults to this checkout. Pass e.g.
# git+file://<repo>?rev=<sha> to assert an older revision fails as expected.
FLAKE=${1:-.}

fail() { echo "FAIL: $*" >&2; exit 1; }

# Clean the shared sandbox when the harness exits, including failed controls.
SANDBOX=$(mktemp -d)
trap 'rm -rf "$SANDBOX"' EXIT

run_platform() (
  local system=$1
  local sandbox="$SANDBOX/$system"
  mkdir -p "$sandbox"

  # The evaluated postInstall, raw (no shell re-quoting hazards).
  local script
  script=$(nix eval --raw "${FLAKE}#packages.${system}.sheprd.postInstall") \
    || fail "cannot evaluate postInstall for $system"
  [ -n "$script" ] || fail "empty postInstall for $system"

  # Sandbox cwd: copies of the source files the script installs (Darwin bundle
  # resources and the Linux desktop/icon files). Nothing is written to the repo.
  mkdir -p "$sandbox/src/assets/macos" "$sandbox/src/assets/icons" \
    "$sandbox/src/crates/sheprd-gpui" "$sandbox/src/scripts/release"
  for file in assets/macos/Info.plist assets/icons/Sheprd.icns \
    assets/icons/Sheprd.car assets/icons/sheprd-linux.svg \
    scripts/release/org.sheprd.Sheprd.desktop LICENSE NOTICE \
    crates/sheprd-gpui/SOUND-NOTICE.md assets/icons/LICENSE-octicons \
    scripts/release/herdr-core-licenses.txt; do
    if [ -e "$REPO/$file" ]; then
      cp "$REPO/$file" "$sandbox/src/$file"
    else
      fail "missing repository resource the script installs: $file"
    fi
  done

  # Stub derivation outputs the script addresses: out holds the built
  # binaries; acp, remoteControl, automate are declared companions.
  local out acp remoteControl automate inputs
  out="$sandbox/out" acp="$sandbox/acp" remoteControl="$sandbox/remoteControl" \
    automate="$sandbox/automate"
  mkdir -p "$out/bin" "$acp/bin" "$remoteControl/bin" "$automate/bin"
  # Distinct stub binaries, so byte-equality assertions prove each companion
  # got its own program, not any executable.
  for program in sheprd sheprd-acp sheprd-remote-control sheprd-automate; do
    printf '#!/bin/sh
echo %s
' "$program" > "$out/bin/$program"
    chmod 755 "$out/bin/$program"
  done

  # Substitute only the immutable Nix store inputs: ${nearby} and
  # ${herdrCore} appear as /nix/store/<hash>-*/bin/<program> paths. Point them
  # at synthetic stubs so the Darwin bundle steps run untouched.
  inputs="$sandbox/inputs/bin"
  mkdir -p "$inputs"
  for program in sheprd-nearby herdr-core; do
    printf '#!/bin/sh
echo %s
' "$program" > "$inputs/$program"
    chmod 755 "$inputs/$program"
  done
  script=$(printf '%s' "$script" | sed -E 's#/nix/store/[a-z0-9]+-sheprd-nearby[a-z0-9.-]*/bin/sheprd-nearby#"'"$inputs"'"/sheprd-nearby#g; s#/nix/store/[a-z0-9]+-herdr-core[a-z0-9.-]*/bin/herdr-core#"'"$inputs"'"/herdr-core#g')

  # Execute the whole evaluated script with Nix's set -e semantics.
  (
    cd "$sandbox/src"
    export out acp remoteControl automate
    set -e
    bash -e <<<"$script"
  ) || fail "postInstall execution failed for $system"

  # Companion assertions: real copies, legacy aliases, ACP sibling link.
  local output program path alias
  for pair in "acp sheprd-acp" "remoteControl sheprd-remote-control" "automate sheprd-automate"; do
    read -r output program <<<"$pair"
    path="$sandbox/$output/bin/$program"
    [ -f "$path" ] || fail "$system: $path missing"
    [ -L "$path" ] && fail "$system: $path is a symlink (GUI closure leak into companion)"
    cmp -s "$path" "$out/bin/$program" \
      || fail "$system: $path does not byte-match its own stub binary"
    alias=${program/sheprd-/herdr-}
    [ -e "$sandbox/$output/bin/$alias" ] \
      || fail "$system: legacy alias $alias missing in $output"
    [ -L "$sandbox/$output/bin/$alias" ] \
      || fail "$system: legacy alias $alias must be a symlink"
    echo "ok $system: $output/bin/$program is a real copy of its own binary with alias $alias"
  done
  [ -L "$sandbox/remoteControl/bin/sheprd-acp" ] \
    || fail "$system: remoteControl does not discover ACP beside its executable"
  [ "$(readlink "$sandbox/remoteControl/bin/sheprd-acp")" = "$acp/bin/sheprd-acp" ] \
    || fail "$system: remoteControl ACP link points outside the acp output"
  echo "ok $system: remoteControl/bin/sheprd-acp -> acp output"
)

run_platform x86_64-linux
run_platform aarch64-darwin
echo "companion postInstall regression passed for linux and darwin ($FLAKE)"
