# Plan: status bar, menu bar, and Wayland window chrome

Status: design agreed; bounded section-3 checkpoint recorded below. Deliver the three changes below
independently. Do not modify the Herdr daemon or replace GPUI's window backend.

## Existing feature: GLM Coding Plan usage

No new provider is needed. `crates/herdr-usage/src/providers/zai.rs` already registers **z.ai /
GLM**, reading five-hour and weekly quota windows (and additional available
usage details). Enable **Settings > Status bar > Usage** and configure an API
key in the user's `config-gpui.local.toml`, normally under `~/.config/herdr/`:

```toml
[usage.providers.zai]
api_key = "YOUR_Z_AI_API_KEY"
```

The provider also accepts `Z_AI_API_KEY` and supports a separate
`region = "bigmodel-cn"` configuration for China. It displays the selected
host's account, including supported SSH hosts. Do not commit keys or log them.
If usage is missing, investigate detection and provider errors before proposing
a second GLM integration. See [usage-providers.md](usage-providers.md).

## 1. Status-bar context menu

Right-click anywhere in the main status bar to open one menu of checkable
**Show** items: **Usage**, **CPU and memory**, **Network quality**,
**Listening ports**, **Keep Awake**, **Theme**, **Shortcuts**, and
**Report issue**. Hidden items remain in the menu so they can be restored.
Changes take effect immediately, persist across windows/restarts, and agree
with **Settings > Status bar** (or **General** for listening ports).

Reuse the existing visibility settings rather than creating a second per-bar
state: `[usage] show`, `show_system_load`, `show_network_quality`,
`show_listening_ports`, and `[status_bar]` button/Keep Awake settings.
Consequently, hiding Usage stops quota polling, hiding CPU/memory also affects
sidebar host readings, and hiding listening ports also affects the sidebar.
Keep the existing detailed/compact and icon/label choices unchanged when an
item is hidden and restored.

**Keep Awake exception:** hiding its cup must also release an active sleep
inhibition, whether indefinite or until agents finish. Restoring the cup
does **not** automatically reactivate the hold. If release fails, keep the
active indicator accessible and show the existing error path rather than
silently hiding a live inhibition. This deliberately changes the current
behavior in `crates/herdr-gpui/src/window/status_bar.rs`, which keeps an active cup visible even
when its visibility setting is false.

Do not add switches for the focused agent's model, connection/error messages,
or version/update indicator. These remain available as the bar's essential
information. A context-menu press must not trigger the underlying button,
change focus to a terminal, or send a right-click to a terminal pane.

Acceptance: headless input tests open/dismiss the menu, check every item,
toggle an item off and back on, verify Settings/config synchronization, and
verify Keep Awake release and failure behavior. Verify the menu still works
when optional items are all hidden and in a narrow window.

## 2. Hide the in-window menu bar

On **Linux and Windows**, add one saved, application-wide visibility setting
for the **main windows**, initially **visible** for existing users. Put the
setting in the existing interface preferences; every open main window follows
changes immediately. macOS retains its native system menu behavior.

**Ctrl+Shift+M** toggles persistent visibility. **F10** activates the menu
temporarily even when hidden; **Alt** alone does so on Windows. The temporary
menu is an overlay at the window's top, not an inserted layout row: it must
not shift tabs, terminals, or pointer geometry. Escape/F10 dismiss it and
restore prior focus; normal menu navigation continues to work. These keys
must not leak as input to the daemon while handling the menu. Keep the
shortcut rebindable through the existing keymap machinery.

For Wayland windows using client-side decorations, hiding the menu bar moves
minimize/maximize/close into the existing titlebar or top tab/header row.
Never strand a window without its close or move affordance. When server-side
decorations are active, leave window controls with the compositor, except for
the existing Linux `show_window_controls` override. **Approved exception:** keep
that setting effective in the header/tab row and zen band when the menu is
hidden. It defaults to `true` on Linux and may duplicate compositor buttons;
users with a working server titlebar can set it to `false` to avoid duplication.
Client-decorated windows retain controls even when the setting is `false`.
**Approved layout tradeoff:** whenever the hidden-menu window needs controls,
retain the existing full-width header rather than merging it into a tab strip,
even at wide widths. This avoids sizing dynamic Git badges and split-group
strips; badges may clip before the account and window buttons. Zen mode keeps
its draggable control band. Visible-menu and no-controls layouts are unchanged.
Preserve menu accessibility with the bar hidden and under
keyboard-only operation.

Acceptance: tests cover persistence, two simultaneous windows, F10 and Windows
Alt activation, focus restoration, no terminal input leakage, narrow layouts,
and window controls with both client- and server-decorated modes. Native input
QA is required for OS keyboard delivery and compositors.

## 3. Opt-in Wayland client-side decorations and blur

The **main window only** gets an opt-in **request client-side decorations**
preference on Wayland; default behavior remains GPUI/compositor negotiation.
The preference takes effect when reopening the window. Settings, Logs, and
other secondary windows retain their current decoration behavior. X11,
Windows, and macOS are unaffected.

If the compositor honors the request, use the existing client decoration
frame, resize handles, titlebar moves, and supported window controls. Extend
the existing backdrop's opacity consistently into the custom titlebar; retain
shadows and resize handles outside the content area, including correct tiled
and maximized edges. If the compositor refuses client decorations, preserve
the working server frame and explain the limitation. Never simulate an
unnegotiated frameless window or require users to change their window-manager
configuration.

The existing **Translucent** and **Blur behind** options stay independent.
When blur is enabled for a translucent window, request it through an
advertised compositor protocol: `ext-background-effect-v1` (including KWin
in Plasma 6.7+) or the older KDE blur protocol where available. Respect the
new protocol's advertised blur capability, requested region, lifecycle, and
surface commits; transparency alone is not a blur request. Avoid assuming
that all Hyprland or other Wayland versions offer either protocol. If neither
works, keep translucency without blur and report the limitation, without
external KWin scripts, window rules, or compositor-specific hacks.

GPUI is pinned to **0.3.6**. Check its actual Wayland API and protocol
support before implementation; request missing functionality through the
GPUI/platform layer rather than guessing that current window options can
force decorations or that `WindowBackgroundAppearance::Blurred` implements
both protocols. Do not modify the external Herdr daemon or sibling checkouts.
Reference: [Wayland background-effect protocol](https://wayland.app/protocols/ext-background-effect-v1),
[KDE Plasma 6.7 changelog](https://kde.org/announcements/changelogs/plasma/6/6.6.5-6.7.0/).

Acceptance: tests exercise decoration negotiation/fallback, titlebar actions,
resize/tiling geometry, opacity, blur capability and region lifetime. Native
QA is required on **KWin/Plasma 6 Wayland** (including a version supporting
the new protocol) and **Hyprland**; check normal and tiled/maximized windows,
opaque/translucent/blurred modes, keyboard focus, and shutdown.

### Bounded Herdr-side checkpoint (not full section-3 completion)

Implemented the default-off `request_client_side_decorations` saved preference,
General settings/help and config CLI access. Only centralized main-window
creation requests `Client`, and only on the actual Wayland backend; reopen to
apply. Secondary constructors are untouched. Rendering still follows observed
`Decorations`, with the existing supported controls and tiled resize geometry.
The header already uses backdrop `Fill::tint`, the menu uses `Fill::panel`, and
the zen band receives `Fill::ground`; no additional opacity layer was needed.

### Local platform-status consumption checkpoint

Herdr now consumes the reviewed local GPUI decoration-negotiation and blur-support
queries. A separate dismissible appearance card reports only the initial Client
request with processed Configured Server fallback, or an actual Blurred background
request with Unsupported support. Pending and unreported (`None`) statuses are
silent. Existing appearance notifications and backdrop synchronization refresh
these limitations; unchanged updates preserve dismissal, resolution clears them,
and changed limitations can reappear. Connection/local-error diagnostics remain
independent. Existing `status` and `window list` expose the same current limitation
text as additive, defaulted `appearance_limitations`, independently of dismissal.
Supported means an advertised usable request path, not confirmed pixels.

Verification uses temporary external Cargo patches to the reviewed local
`gpui-pre` and `gpui-pre-linux` backport, not a durable dependency update. The
original Herdr lockfile is restored byte-for-byte afterward. Immutable source
selection and Nix hashes remain a parent delivery gate; the unchanged registry
pin does not yet provide these APIs. Native KWin (old and new protocol), Hyprland,
Windows, full CI and full section-3 acceptance remain outstanding. Local focused
checks are not final reproducible or compositor validation.

### Final integration status

The local-patch verification above is historical. Both GPUI packages now use the
published immutable backport revision and verified Nix hash documented in the
[root README](../README.md#from-source). Full application CI, Linux lint, the
release build/CLI checks, and the final Nix snapshot build passed before the final
text-only documentation and Settings help corrections. Windows lint was attempted
but blocked by missing target core/std; native compositor and Windows/macOS QA
remain outstanding. This is not full section-3 acceptance.

## Delivery and verification

Keep each feature a separate, reviewable change: (1) status-bar menu,
(2) menu-bar visibility, (3) Wayland decorations/blur. Reuse existing config,
preference saving, menu/input isolation, and window chrome; avoid a new
settings subsystem. For every change, document behavior in the relevant
README/settings help and add regression tests. Run focused tests, `just format`,
`just ci`, and Linux/Windows lint gates where platform code is affected.
Headless tests are not a substitute for native compositor QA. The original plan was design-only; the section-3 checkpoint above records the
current implementation boundary, not full acceptance.