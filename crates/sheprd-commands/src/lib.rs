//! The command catalog: every command the app offers in menus, the palette,
//! and key bindings, its name and default shortcuts per platform, and the
//! daemon request a command becomes against a snapshot. No GPUI; the app
//! binds the shortcuts and runs the commands that stay in the window.

mod catalog;
mod request;
mod shortcuts;

pub use catalog::{COMMANDS, Command, CommandInfo};
pub use request::request;
pub use shortcuts::CTRL_SHORTCUTS;
#[cfg(any(test, feature = "test-support"))]
pub use shortcuts::platform_default;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests;
