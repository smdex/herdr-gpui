//! The status bar uses the shared popup shell and preference writer.
use crate::HerdrWindow;
use crate::config::Config;
use gpui::{prelude::*, *};

struct Item {
    label: &'static str,
    key: &'static str,
    shown: fn(&Config) -> bool,
}

const ITEMS: &[Item] = &[
    Item {
        label: "Usage",
        key: "usage.show",
        shown: |c| c.usage.show,
    },
    Item {
        label: "CPU and memory",
        key: "show_system_load",
        shown: |c| c.show_system_load,
    },
    Item {
        label: "Network quality",
        key: "show_network_quality",
        shown: |c| c.show_network_quality,
    },
    Item {
        label: "Listening ports",
        key: "show_listening_ports",
        shown: |c| c.show_listening_ports,
    },
    Item {
        label: "Keep Awake",
        key: "status_bar.keep_awake",
        shown: |c| c.status_bar.keep_awake,
    },
    Item {
        label: "Theme",
        key: "status_bar.show_theme",
        shown: |c| {
            c.status_bar
                .theme
                .with_visibility(c.status_bar.show_theme)
                .shown()
        },
    },
    Item {
        label: "Shortcuts",
        key: "status_bar.show_shortcuts",
        shown: |c| {
            c.status_bar
                .shortcuts
                .with_visibility(c.status_bar.show_shortcuts)
                .shown()
        },
    },
    Item {
        label: "Report issue",
        key: "status_bar.show_report_issue",
        shown: |c| {
            c.status_bar
                .report_issue
                .with_visibility(c.status_bar.show_report_issue)
                .shown()
        },
    },
];

#[cfg(test)]
type Save = dyn Fn(&str, bool) -> crate::Result<(Config, crate::config::Theme)> + Send + Sync;

/// Replaces only the filesystem boundary, leaving save/reload and menu input real.
#[cfg(test)]
pub(crate) struct StatusBarIo(pub std::sync::Arc<Save>);
#[cfg(test)]
impl Global for StatusBarIo {}

impl HerdrWindow {
    pub(crate) fn status_bar_menu_count(&self) -> usize {
        ITEMS.len() + usize::from(self.config.status_bar.keep_awake)
    }

    pub(crate) fn activate_status_bar_menu(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(item) = ITEMS.get(index) {
            let (key, shown) = (item.key, !(item.shown)(&self.config));
            self.dismiss_menu(window, cx);
            #[cfg(test)]
            if let Some(io) = cx.try_global::<StatusBarIo>() {
                let save = io.0.clone();
                self.load_gui_config_with(move || save(key, shown), cx);
                return;
            }
            self.save_preference(move || Config::set_key(key, shown.into()), cx);
        } else if index == ITEMS.len() && self.config.status_bar.keep_awake {
            self.dismiss_menu(window, cx);
            let report = Self::caffeine_report(cx);
            crate::caffeine::toggle_until_agents_finish(cx, report);
        }
    }

    pub(crate) fn render_status_bar_menu(
        &self,
        mut panel: Stateful<Div>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        for index in 0..self.status_bar_menu_count() {
            let (label, checked) = if let Some(item) = ITEMS.get(index) {
                (
                    format!("Show {}", item.label),
                    Some((item.shown)(&self.config)),
                )
            } else {
                ("Keep Awake until agents finish".into(), None)
            };
            let selector = ITEMS.get(index).map_or_else(
                || "status-awake-until-agents".to_owned(),
                |item| format!("status-show-{}", item.label),
            );
            panel = panel.child(
                div()
                    .id(SharedString::from(selector.clone()))
                    .debug_selector(move || selector.clone())
                    .role(if checked.is_some() {
                        Role::MenuItemCheckBox
                    } else {
                        Role::MenuItem
                    })
                    .aria_label(label.clone())
                    .when_some(checked, |row, checked| {
                        row.aria_toggled(if checked {
                            Toggled::True
                        } else {
                            Toggled::False
                        })
                    })
                    .min_h(px(self.config.ui.line_height() + 12.))
                    .px(px(8.))
                    .cursor_pointer()
                    .when(self.menu.selected == Some(index), |row| {
                        row.aria_active_descendant().bg(rgb(self.theme.active))
                    })
                    .hover(|row| row.bg(rgb(self.theme.active)))
                    .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                        if *hovered {
                            this.menu.selected = Some(index);
                        } else if this.menu.selected == Some(index) {
                            this.menu.selected = None;
                        }
                        cx.notify();
                    }))
                    .child(format!(
                        "{} {label}",
                        if checked == Some(true) { "✓" } else { "  " }
                    ))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.activate_status_bar_menu(index, window, cx)
                    })),
            );
        }
        panel
    }
}
