{
  description = "Herdr UI: a native GPUI client for an existing local Herdr daemon";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    # Build with exactly the toolchain rust-toolchain.toml pins, not whatever
    # rustc the nixpkgs revision happens to carry.
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      inherit (nixpkgs) lib;
      # GPUI's Linux backend is the only one this flake builds; macOS users
      # install the signed DMG or the Homebrew cask instead.
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems =
        f:
        lib.genAttrs systems (
          system:
          f (
            import nixpkgs {
              inherit system;
              overlays = [ rust-overlay.overlays.default ];
            }
          )
        );
      package =
        pkgs:
        let
          toolchain = (pkgs.extend rust-overlay.overlays.default).rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          rustPlatform = pkgs.makeRustPlatform {
            cargo = toolchain;
            rustc = toolchain;
          };
          # Loaded with dlopen at runtime, so the linker never records them.
          runtimeLibraries = [
            pkgs.vulkan-loader
            pkgs.wayland
          ];
        in
        rustPlatform.buildRustPackage {
          pname = "herdr-gpui";
          version = "0-unstable-${lib.substring 0 8 (self.lastModifiedDate or "19700101")}";
          src = lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              ./Cargo.toml
              ./Cargo.lock
              ./rust-toolchain.toml
              ./crates
              ./assets
              # The agent skill is compiled into the executable.
              ./skills
              ./scripts/release/so.pen.herdr-gpui.desktop
            ];
          };
          outputs = [
            "out"
            "acp"
            "remoteControl"
            "automate"
          ];
          cargoLock = {
            lockFile = ./Cargo.lock;
            # Both patched packages share this Git checkout; one hash covers it.
            outputHashes."gpui-pre-0.3.6" = "sha256-s6zLgvhRJHDGIuoxbB4giMQ1a8g9mxi4p0SqTgjKakM=";
          };
          # The app starts chats with the herdr-acp beside its executable,
          # runs the herdr-remote-control beside it as a user service, and
          # runs automations in the herdr-automate there.
          cargoBuildFlags = [
            "-p"
            "herdr-gpui"
            "-p"
            "herdr-acp"
            "-p"
            "herdr-remote-control"
            "-p"
            "herdr-automate"
          ];
          # The optimized CLI suite CI's main-branch build runs; the rest of
          # the workspace suite needs a desktop or a live daemon.
          cargoTestFlags = [
            "-p"
            "herdr-gpui"
            "--test"
            "cli"
          ];
          nativeBuildInputs = [
            pkgs.makeWrapper
            pkgs.pkg-config
            rustPlatform.bindgenHook
          ];
          buildInputs = [
            pkgs.alsa-lib
            pkgs.fontconfig
            pkgs.freetype
            pkgs.libxkbcommon
            pkgs.libxcb
          ]
          ++ runtimeLibraries;
          # No updater key is embedded, so this build never replaces itself;
          # Nix owns the store path and upgrades come from the flake.
          postInstall = ''
            # Real copies keep current_exe beside the bundle's companions.
            install -Dm755 $out/bin/herdr-acp $acp/bin/herdr-acp
            install -Dm755 $out/bin/herdr-remote-control $remoteControl/bin/herdr-remote-control
            install -Dm755 $out/bin/herdr-automate $automate/bin/herdr-automate
            # The remote daemon discovers ACP beside its own real executable.
            ln -s $acp/bin/herdr-acp $remoteControl/bin/herdr-acp
            install -Dm644 scripts/release/so.pen.herdr-gpui.desktop \
              $out/share/applications/so.pen.herdr-gpui.desktop
            install -Dm644 assets/icons/herdr-linux.svg \
              $out/share/icons/hicolor/scalable/apps/herdr-gpui.svg
          '';
          postFixup = ''
            patchelf --add-rpath ${lib.makeLibraryPath runtimeLibraries} $out/bin/herdr-gpui
            wrapProgram $out/bin/herdr-gpui \
              --prefix PATH : ${lib.makeBinPath [ pkgs.wl-clipboard pkgs.xclip ]}
          '';
          # Every build runs each shipped program's safe CLI and checks ldd,
          # as the release smoke test does, so a package missing a companion
          # or linking outside the store fails to build. Each check prints and
          # exits before any UI, socket, or daemon starts; a wrapper that
          # still runs the real program passes too.
          doInstallCheck = true;
          installCheckPhase = ''
            runHook preInstallCheck
            for program in herdr-gpui herdr-acp herdr-remote-control herdr-automate; do
              test -x "$out/bin/$program"
              # Capture before matching: with pipefail, grep -q exiting early
              # would fail the writer with SIGPIPE (as the release smoke notes).
              linked=$(ldd "$out/bin/$program" 2>/dev/null || true)
              if grep "not found" <<<"$linked"; then
                echo "$program has libraries the output does not carry" >&2
                exit 1
              fi
            done
            $out/bin/herdr-gpui --help >/dev/null
            $out/bin/herdr-acp --help >/dev/null
            $out/bin/herdr-remote-control --version >/dev/null
            $out/bin/herdr-automate --version >/dev/null
            $acp/bin/herdr-acp --help >/dev/null
            $remoteControl/bin/herdr-remote-control --version >/dev/null
            $remoteControl/bin/herdr-acp --help >/dev/null
            $automate/bin/herdr-automate --version >/dev/null
            for standalone in "$acp" "$remoteControl" "$automate"; do
              test ! -e "$standalone/bin/herdr-gpui"
              # A split output must not retain the bundle through ELF RPATHs.
              if grep -r -a -l -F "$out" "$standalone"; then
                echo "standalone output references the GUI bundle" >&2
                exit 1
              fi
            done
            runHook postInstallCheck
          '';
          meta = {
            description = "Native GPUI client for an existing local Herdr daemon";
            homepage = "https://herdrui.dev";
            license = lib.licenses.asl20;
            mainProgram = "herdr-gpui";
            platforms = systems;
          };
        };
      packagesFor =
        pkgs:
        let
          bundle = package pkgs;
          companion =
            output: program:
            output
            // {
              meta = bundle.meta // {
                description = "Herdr companion: ${program}";
                mainProgram = program;
                outputsToInstall = [ output.outputName ];
              };
            };
        in
        {
          herdr-acp = companion bundle.acp "herdr-acp";
          herdr-automate = companion bundle.automate "herdr-automate";
          herdr-gpui = bundle;
          herdr-remote-control = companion bundle.remoteControl "herdr-remote-control";
        };
    in
    {
      packages = forAllSystems (
        pkgs:
        let
          packages = packagesFor pkgs;
        in
        packages // { default = packages.herdr-gpui; }
      );
      overlays.default = final: _prev: packagesFor final;
    };
}
