//! GUI settings. The daemon's own config is read only where the GUI honors a
//! preference the user already expressed there, never written and never used
//! to change daemon behavior. Managed defaults are refreshed from the binary;
//! `config-gpui.local.toml` holds persistent user overrides.
// i18n: migrated
use crate::{
    Result,
    contrast::Contrast,
    keymap::{Binding, DaemonKeys, Keymap, PaneKeys},
};
mod accent;
mod agent_skill;
mod agents_placement;
mod azure;
pub(crate) mod backdrop;
mod bitmap_fonts;
mod boat;
mod boxd;
pub(crate) mod chat;
mod cjk_fonts;
mod cloud;
mod coder;
mod container;
mod daemon;
mod daytona;
mod devices;
mod edit;
mod error;
pub(crate) mod favorite_themes;
mod features;
mod files;
mod font_weight;
mod fonts;
mod github;
mod keybindings;
mod layout;
mod load;
pub(crate) use load::{config_root, home};
mod notifications;
mod omarchy;
mod opensandbox;
mod option_as_alt;
mod padding;
pub(crate) mod preferences;
pub(crate) use sheprd_remote_control::app::config as remote;
pub(crate) mod sidebar;
mod sidebar_style;
mod sprites;
pub(crate) mod ssh_hosts;
pub(crate) mod status_bar;
mod status_icons;
mod tab_labels;
pub(crate) mod tab_layout;
#[cfg(any(
    feature = "azure",
    feature = "boat",
    feature = "boxd",
    feature = "coder",
    feature = "container",
    feature = "daytona",
    feature = "opensandbox",
    feature = "sprites"
))]
mod table;
pub(crate) use sidebar_style::parse_hex;
mod theme;
mod updates;
pub(crate) mod watch;
mod worktrees;
mod zen;

pub use accent::Accent;
pub use agent_skill::{AgentSkillConfig, SkillInstall};
pub use agents_placement::AgentsPlacement;
pub use backdrop::{Backdrop, Material};
pub use chat::{ChatLauncher, ChatLayoutMode};
pub use cloud::CloudConfig;
#[cfg(test)]
use daemon::MAX_DAEMON_CONFIG_BYTES;
pub(crate) use daemon::daemon_config_path;
use daemon::{Daemon, daemon_settings};
pub(crate) use devices::{DeviceSettings, KeybindingSource};
pub(crate) use edit::{Layer, parse_value};
pub(crate) use error::{Error, ThemeParseError};
pub use features::{Feature, Features};
use files::write_config;
pub use font_weight::bold as bold_font_weight;
pub(crate) use fonts::FontFace;
use fonts::FontSettings;
#[cfg(any(test, feature = "integration-test"))]
pub use fonts::symbol_fallbacks;
pub use fonts::{FONT_SIZE_RANGE, FONT_SIZE_STEP, FontConfig};
pub(crate) use github::MAX_ACCOUNTS;
pub use github::{GitHubAccount, GitHubConfig};
pub use layout::{Density, Layout, LayoutMode, Style};
pub(crate) use notifications::NotificationDelivery;
pub use notifications::{
    BellConfig, ClipboardToast, ClipboardToastPosition, NotificationConfig, SoundConfig,
};
use notifications::{ClipboardToastSettings, NotificationSettings};
pub use option_as_alt::{OptionAsAlt, OptionKeys};
pub use padding::Padding;
pub(crate) use padding::{Axis, MAX_PADDING, Side, Sides};
pub use remote::RemoteConfig;
use serde::Deserialize;
pub use sheprd_orchestrator::gitlab::Config as GitLabConfig;
pub use sheprd_remote::config::AzureConfig;
#[cfg(feature = "azure")]
pub(crate) use sheprd_remote::config::AzureFields;
pub use sheprd_remote::config::BoatConfig;
#[cfg(feature = "boat")]
pub(crate) use sheprd_remote::config::BoatFields;
pub use sheprd_remote::config::BoxdConfig;
#[cfg(feature = "boxd")]
pub(crate) use sheprd_remote::config::BoxdFields;
pub use sheprd_remote::config::CoderConfig;
#[cfg(feature = "coder")]
pub(crate) use sheprd_remote::config::CoderFields;
pub use sheprd_remote::config::ContainerConfig;
#[cfg(feature = "container")]
pub(crate) use sheprd_remote::config::ContainerFields;
pub use sheprd_remote::config::DaytonaConfig;
#[cfg(feature = "daytona")]
pub(crate) use sheprd_remote::config::DaytonaFields;
pub use sheprd_remote::config::OpenSandboxConfig;
#[cfg(feature = "opensandbox")]
pub(crate) use sheprd_remote::config::OpenSandboxFields;
pub use sheprd_remote::config::SpritesConfig;
#[cfg(feature = "sprites")]
pub(crate) use sheprd_remote::config::SpritesFields;
pub(crate) use sidebar::{
    AgentLayout, AgentToken, MAX_ROWS as MAX_SIDEBAR_ROWS, Rows, SidebarConfigError, SidebarLayout,
    SidebarScope, SpaceLayout, SpaceToken, TokenStyle,
};
pub(crate) use sidebar_style::{ChangeCounts, DeviceFooter};
pub use sidebar_style::{SelectMode, SidebarOverrides, SidebarStyle};
pub use ssh_hosts::SshConfig;
pub use status_bar::StatusBar;
pub use status_icons::StatusIcons;
use std::{collections::BTreeMap, fs, io::ErrorKind, path::Path};
pub use tab_labels::TabLabels;
pub use tab_layout::TabLayoutMode;
pub use theme::Theme;
pub(crate) use theme::ThemeName;
pub(crate) use theme::mix;
pub use updates::{UpdateChannel, UpdatesConfig};
pub use zen::{Zen, ZenWidth};

const DEFAULT_CONFIG: &str = include_str!("../config-gpui.example.toml");
const MANAGED_HEADER: &str = sheprd_client::settings_files::MANAGED_HEADER;
/// Seeds the overrides file on first launch only.
const LOCAL_CONFIG: &str = sheprd_client::settings_files::OVERRIDES_SEED;

/// Shared logical-pixel radii for native-style chrome, independent of the
/// terminal grid. Small badges/keycaps retain a tighter curve than controls.
pub(crate) mod corners {
    pub(crate) const PANEL: f32 = 12.;
    pub(crate) const CONTROL: f32 = 8.;
    pub(crate) const SMALL: f32 = 4.;
}

#[derive(Clone, Debug)]
pub struct Config {
    // keep-sorted start
    /// Where the app's own accent color comes from.
    pub accent: Accent,
    /// Snapshot a checkout's files each time one of its agents starts or
    /// finishes a turn, so they can be rolled back.
    pub agent_checkpoints: bool,
    /// Where the managed agent skill may go on its own.
    pub agent_skill: AgentSkillConfig,
    /// Where the sidebar lists agents while `show_agents` is on.
    pub agents_placement: AgentsPlacement,
    /// What shows behind the main window's terminals and chrome.
    pub azure: AzureConfig,
    pub backdrop: Backdrop,
    pub bell: BellConfig,
    pub boat: BoatConfig,
    /// The boxd account whose machines are devices.
    pub boxd: BoxdConfig,
    /// What chats started from the app run with.
    pub chat: chat::ChatConfig,
    /// Only the keys this GUI's own file sets, so Settings can tell an
    /// override from the Herdr value it shadows.
    pub(crate) clipboard_overrides: ClipboardToastSettings,
    pub clipboard_toast: ClipboardToast,
    /// What a new cloud device runs the first time it connects.
    pub cloud: CloudConfig,
    pub code: CodeConfig,
    pub coder: CoderConfig,
    pub confirm_close_pane: bool,
    pub confirm_close_tab: bool,
    /// Whether a workspace close shows its dialog when the check finds no
    /// unpublished work. Uncommitted or unpushed work always asks.
    pub confirm_close_workspace: bool,
    /// How new containers on this computer's Docker or Podman engine are made.
    pub container: ContainerConfig,
    /// How far the app's own marks and labels stand off its chrome.
    pub contrast: Contrast,
    pub daytona: DaytonaConfig,
    /// Per saved device, by catalog profile ID.
    pub(crate) devices: BTreeMap<String, DeviceSettings>,
    /// A bar on the leading edge of each working, blocked, or done row,
    /// whichever status icons are drawn.
    pub edge_rail: bool,
    /// How the editor starts; the pane's `$VISUAL` or `$EDITOR` when unset.
    pub(crate) editor_command: Option<crate::editor::EditorCommand>,
    /// Themes starred in the Color Scheme picker, which lists them first.
    pub favorite_themes: Vec<String>,
    pub features: Features,
    /// Focus the split terminal pane under the pointer without clicking.
    pub focus_follows_mouse: bool,
    pub github: GitHubConfig,
    /// Where GitLab tokens pasted in the account menu are saved.
    pub gitlab: GitLabConfig,
    /// New tabs run in herdr-core (Herdr 2) rather than Herdr 1, for their
    /// native panes. Off by default while Herdr 1 holds the user's sessions;
    /// without Herdr 1 installed, terminals are Herdr 2's anyway.
    pub herdr_2_terminals: bool,
    /// Font resolution searched for an icon font for the terminal and found
    /// none installed, so Private Use Area glyphs draw as missing-glyph boxes.
    /// Never set when the config names the terminal's `fallback` itself.
    pub(crate) icon_font_missing: bool,
    /// Whether a terminal selection stays highlighted, and readable by
    /// selection tools, after it is copied.
    pub keep_selection_after_copy: bool,
    /// The `[keybindings]` table `keybindings` was built from, kept so a
    /// device's server keys can be layered under the same GUI overrides.
    pub(crate) keybinding_overrides: BTreeMap<String, Binding>,
    pub keybindings: Keymap,
    /// The language the app shows: `system`, or a BCP 47 tag (docs/i18n.md).
    pub language: sheprd_i18n::Language,
    pub layout: Layout,
    /// Configured families, sorted, that font resolution found no installed
    /// family for; `diagnostic` reports them.
    pub(crate) missing_fonts: Vec<String>,
    /// The terminal's configured weight, when its family has no face at it.
    pub(crate) missing_weight: Option<font_weight::MissingWeight>,
    pub mods: crate::mods::Config,
    pub(crate) notification_overrides: NotificationSettings,
    pub notifications: NotificationConfig,
    /// Where a link-modifier click on a printed file path opens it.
    pub open_files_in: FileTarget,
    pub open_links_in: LinkTarget,
    /// The self-hosted OpenSandbox server whose sandboxes are devices.
    pub opensandbox: OpenSandboxConfig,
    pub option_as_alt: OptionAsAlt,
    pub palette: crate::palette::PaletteConfig,
    /// The `[pane_keys]` table `keybindings` was built from, for the same
    /// reason.
    pub(crate) pane_keys: PaneKeys,
    pub(crate) px0: crate::px0::Config,
    /// Remote control from a phone, through the `herdr-remote-control` daemon.
    pub remote: RemoteConfig,
    /// Whether a linked worktree whose pull request is merged is removed, and
    /// its workspace closed, without asking. The focused workspace still asks.
    pub remove_merged_worktrees: bool,
    /// Request Wayland client decorations for newly opened main windows only.
    pub request_client_side_decorations: bool,
    pub show_agents: bool,
    /// The "Agents" title row above the agents list, which also holds the sort
    /// toggle. The sort then follows Herdr's `ui.agent_panel_sort` unless it
    /// was toggled earlier, unless a plugin sets the agent view, which keeps the
    /// header visible. Set in Settings > Sidebar or with
    /// `sheprd config set show_agents_header false`.
    pub show_agents_header: bool,
    /// The mark before a device's name in the sidebar, in the layouts that
    /// draw one: where the device runs and how it is reached.
    pub show_device_icons: bool,
    /// The selected host's GPU and video memory in the status bar. Off by
    /// default; see [`Self::gpu_sampled`].
    pub show_gpu_load: bool,
    /// Ports each workspace listens on, in the sidebar and the status bar.
    pub show_listening_ports: bool,
    /// Saved visibility of the main-window menu row on Linux and Windows.
    pub show_menu_bar: bool,
    /// Latency and loss from this machine, by timed TCP connects, in the status bar.
    pub show_network_quality: bool,
    /// The repository owner's avatar, or the GitHub mark, before a space's name.
    pub show_repository_icons: bool,
    /// Each host's GPU and video memory on a sidebar line under its CPU and
    /// memory. Off by default; see [`Self::gpu_sampled`].
    pub show_sidebar_gpu_load: bool,
    /// The search field at the top of the sidebar's spaces list.
    pub show_sidebar_search: bool,
    /// CPU and memory of the selected host in the status bar.
    pub show_system_load: bool,
    /// Draw minimize, maximize, and close buttons in the menu bar on Linux.
    /// When true the window needs no title bar from the window manager.
    pub show_window_controls: bool,
    pub sidebar: FontConfig,
    /// Daemon sidebar rows, falling back to defaults when invalid.
    pub sidebar_layout: SidebarLayout,
    /// Spacing overrides, host colours, and selection marking for the sidebar.
    pub sidebar_style: SidebarStyle,
    /// Wheel scrollback draws panes between rows, following the OS deltas.
    pub smooth_scroll: bool,
    /// How loud agent notification sounds play.
    pub sound: SoundConfig,
    /// The Fly Sprites account whose sprites are devices.
    pub sprites: SpritesConfig,
    /// SSH hosts whose host keys the user chose not to verify.
    pub ssh: SshConfig,
    /// Which status bar items show, and how compactly.
    pub status_bar: StatusBar,
    /// The set agent status is drawn with.
    pub status_icons: StatusIcons,
    /// What a tab with no name of its own is called.
    pub tab_labels: TabLabels,
    /// How tab strips draw their tabs, from View > Tab Layout.
    pub tab_layout: TabLayoutMode,
    pub tabs: FontConfig,
    pub terminal: FontConfig,
    /// Blank space around every terminal's cells.
    pub terminal_padding: Padding,
    pub theme: String,
    pub ui: FontConfig,
    /// Keys the file names that this build does not know, sorted. They are
    /// ignored, as Herdr ignores its own, so a config written by a newer
    /// build or with a typo still loads; `diagnostic` reports them.
    pub unknown_keys: Vec<String>,
    /// Which releases the in-app updater offers.
    pub updates: UpdatesConfig,
    pub usage: crate::usage::UsageConfig,
    /// How linked worktrees are marked apart from their main checkout.
    pub worktrees: crate::worktree_identity::Worktrees,
    /// What Toggle Zen Mode does beyond hiding the chrome.
    pub zen: Zen,
    // keep-sorted end
}

/// Where a link-modifier click on a terminal's web link opens it. Holding
/// Alt (Option on macOS) as well opens it in the other one.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LinkTarget {
    #[default]
    System,
    /// A browser tab in the workspace, where the build can show pages.
    BrowserTab,
}

/// VS Code as a tab of a space's editor groups, served by
/// `code serve-web`. Each space that shows it gets
/// its own page, which starts at the server's address and then goes
/// wherever it navigates.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct CodeConfig {
    /// Where the server comes from. Unset, it is `address` when `url` is
    /// set, as before there was a choice, else `start` once VS Code is found.
    pub(crate) mode: Option<CodeMode>,
    /// The server to use in `address` mode. `None` leaves the panel empty,
    /// with a hint to set it.
    pub(crate) url: Option<crate::browser::WebUrl>,
    /// The port of the server the app starts, picked once and kept: VS Code
    /// keeps its settings in the storage of the page's origin.
    pub(crate) port: Option<std::num::NonZeroU16>,
    /// The user accepted VS Code's server license in the app. Until then
    /// the app starts nothing, since it would accept it for them.
    pub(crate) license_accepted: bool,
}

/// Where VS Code tabs' server comes from.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CodeMode {
    /// The app starts `code serve-web` itself.
    Start,
    /// A server the user runs, at `url`.
    Address,
}

/// One change to the `[code]` table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CodeEdit {
    /// Sets the address, or removes it with `None`.
    Url(Option<crate::browser::WebUrl>),
    Mode(CodeMode),
    Port(std::num::NonZeroU16),
    AcceptLicense,
}

/// Where a clicked file path opens. Alt-click (Option on macOS) opens it in
/// the other one.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum FileTarget {
    /// The terminal editor, in a pane beside the one that printed the path.
    /// Folders, images, and PDFs still open in the system's application.
    #[default]
    Editor,
    /// The system's default application.
    System,
}

impl Default for Config {
    fn default() -> Self {
        let fonts::DefaultFonts {
            monospace,
            sans: ui,
            ..
        } = fonts::PLATFORM_FONTS;
        let font = |family: &str, size| FontConfig {
            family: family.into(),
            size,
            fallbacks: None,
            line_height_multiple: None,
            weight: gpui::FontWeight::NORMAL,
        };
        Self {
            // keep-sorted start
            accent: Accent::default(),
            agent_checkpoints: true,
            agent_skill: AgentSkillConfig::default(),
            agents_placement: AgentsPlacement::default(),
            azure: AzureConfig::default(),
            backdrop: Backdrop::default(),
            bell: BellConfig::default(),
            boat: BoatConfig::default(),
            boxd: BoxdConfig::default(),
            chat: chat::ChatConfig::default(),
            clipboard_overrides: ClipboardToastSettings::default(),
            clipboard_toast: ClipboardToast::default(),
            cloud: CloudConfig::default(),
            code: CodeConfig::default(),
            coder: CoderConfig::default(),
            confirm_close_pane: true,
            confirm_close_tab: true,
            confirm_close_workspace: true,
            container: ContainerConfig::default(),
            contrast: Contrast::default(),
            daytona: DaytonaConfig::default(),
            devices: BTreeMap::new(),
            edge_rail: false,
            editor_command: None,
            favorite_themes: Vec::new(),
            features: Features::default(),
            focus_follows_mouse: true,
            github: GitHubConfig::default(),
            gitlab: GitLabConfig::default(),
            herdr_2_terminals: false,
            icon_font_missing: false,
            keep_selection_after_copy: true,
            keybinding_overrides: BTreeMap::new(),
            keybindings: Keymap::default(),
            language: sheprd_i18n::Language::default(),
            layout: Layout::default(),
            missing_fonts: Vec::new(),
            missing_weight: None,
            mods: crate::mods::Config::default(),
            notification_overrides: NotificationSettings::default(),
            notifications: NotificationConfig::default(),
            open_files_in: FileTarget::default(),
            open_links_in: LinkTarget::default(),
            opensandbox: OpenSandboxConfig::default(),
            option_as_alt: OptionAsAlt::default(),
            palette: crate::palette::PaletteConfig::default(),
            pane_keys: PaneKeys::new(),
            px0: crate::px0::Config::default(),
            remote: RemoteConfig::default(),
            remove_merged_worktrees: false,
            request_client_side_decorations: false,
            show_agents: true,
            show_agents_header: true,
            show_device_icons: true,
            show_gpu_load: false,
            show_listening_ports: true,
            show_menu_bar: true,
            show_network_quality: false,
            show_repository_icons: true,
            show_sidebar_gpu_load: false,
            show_sidebar_search: true,
            show_system_load: true,
            show_window_controls: cfg!(target_os = "linux"),
            sidebar: font(monospace, 12.0),
            sidebar_layout: SidebarLayout::default(),
            sidebar_style: SidebarStyle::default(),
            smooth_scroll: true,
            sound: SoundConfig::default(),
            sprites: SpritesConfig::default(),
            ssh: SshConfig::default(),
            status_bar: StatusBar::default(),
            status_icons: StatusIcons::default(),
            tab_labels: TabLabels::default(),
            tab_layout: TabLayoutMode::default(),
            // Tabs are terminal chrome, so they read in the monospace face the
            // sidebar and terminal use, as they do in the reference UI.
            tabs: font(monospace, 12.0),
            terminal: font(monospace, 14.0),
            terminal_padding: Padding::default(),
            theme: "Default".into(), // i18n: data
            ui: font(ui, 12.0),
            unknown_keys: Vec::new(),
            updates: UpdatesConfig::default(),
            usage: crate::usage::UsageConfig::default(),
            worktrees: crate::worktree_identity::Worktrees::default(),
            zen: Zen::default(),
            // keep-sorted end
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct Settings {
    // keep-sorted start
    accent: Accent,
    agent_checkpoints: Option<bool>,
    agent_skill: AgentSkillConfig,
    agents_placement: AgentsPlacement,
    azure: AzureConfig,
    backdrop: Backdrop,
    bell: BellConfig,
    boat: BoatConfig,
    boxd: BoxdConfig,
    chat: chat::ChatConfig,
    clipboard_toast: ClipboardToastSettings,
    cloud: CloudConfig,
    code: CodeConfig,
    coder: CoderConfig,
    confirm_close_pane: Option<bool>,
    confirm_close_tab: Option<bool>,
    confirm_close_workspace: Option<bool>,
    container: ContainerConfig,
    contrast: Contrast,
    daytona: DaytonaConfig,
    devices: BTreeMap<String, DeviceSettings>,
    edge_rail: Option<bool>,
    editor_command: Option<crate::editor::EditorCommand>,
    favorite_themes: Vec<String>,
    features: Features,
    focus_follows_mouse: Option<bool>,
    github: GitHubConfig,
    gitlab: GitLabConfig,
    herdr_2_terminals: Option<bool>,
    keep_selection_after_copy: Option<bool>,
    keybindings: BTreeMap<String, Binding>,
    language: Option<sheprd_i18n::Language>,
    layout: Layout,
    mods: crate::mods::Config,
    notifications: NotificationSettings,
    open_files_in: FileTarget,
    open_links_in: LinkTarget,
    opensandbox: OpenSandboxConfig,
    option_as_alt: OptionAsAlt,
    palette: crate::palette::PaletteConfig,
    pane_keys: PaneKeys,
    px0: crate::px0::Config,
    remote: RemoteConfig,
    remove_merged_worktrees: Option<bool>,
    request_client_side_decorations: Option<bool>,
    show_agents: Option<bool>,
    show_agents_header: Option<bool>,
    show_device_icons: Option<bool>,
    show_gpu_load: Option<bool>,
    show_listening_ports: Option<bool>,
    show_menu_bar: Option<bool>,
    show_network_quality: Option<bool>,
    show_repository_icons: Option<bool>,
    show_sidebar_gpu_load: Option<bool>,
    show_sidebar_search: Option<bool>,
    show_system_load: Option<bool>,
    show_window_controls: Option<bool>,
    sidebar: sidebar_style::SidebarSettings,
    smooth_scroll: Option<bool>,
    sound: SoundConfig,
    sprites: SpritesConfig,
    ssh: SshConfig,
    status_bar: StatusBar,
    status_icons: Option<status_icons::SavedIcons>,
    tab_labels: TabLabels,
    tab_layout: TabLayoutMode,
    tabs: FontSettings,
    terminal: FontSettings,
    theme: Option<String>,
    ui: FontSettings,
    updates: UpdatesConfig,
    usage: crate::usage::UsageConfig,
    worktrees: worktrees::WorktreeSettings,
    zen: zen::ZenSettings,
    // keep-sorted end
}

#[cfg(test)]
mod tests;
