use super::*;
use crate::config::preferences::Preference;

#[test]
fn client_decorations_default_off_and_persist_through_preferences_and_cli() -> anyhow::Result<()> {
    assert!(!Config::default().request_client_side_decorations);
    assert!(!Config::parse("")?.request_client_side_decorations);
    assert!(!Config::parse(DEFAULT_CONFIG)?.request_client_side_decorations);
    let temp = TempDirectory::new()?;
    let path = temp.0.join("config-gpui.toml");
    let local = path.with_extension("local.toml");
    let daemon = temp.0.join("daemon.toml");
    Config::save_preference_path(Preference::RequestClientSideDecorations(true), &local)?;
    assert!(Config::check_files_at(&path, &daemon)?.request_client_side_decorations);
    assert_eq!(
        Config::effective_value_at(&path, "request_client_side_decorations")?,
        Some((toml::Value::Boolean(true), Layer::Local))
    );
    let before = fs::read_to_string(&local)?;
    assert!(
        Config::edit_key_at(
            &local,
            &daemon,
            "request_client_side_decorations",
            Some("yes".into())
        )
        .is_err()
    );
    assert_eq!(fs::read_to_string(&local)?, before);
    Config::edit_key_at(
        &local,
        &daemon,
        "request_client_side_decorations",
        Some(false.into()),
    )?;
    assert!(!Config::check_files_at(&path, &daemon)?.request_client_side_decorations);
    Config::edit_key_at(&local, &daemon, "request_client_side_decorations", None)?;
    assert_eq!(
        Config::effective_value_at(&path, "request_client_side_decorations")?,
        Some((toml::Value::Boolean(false), Layer::Default))
    );
    Ok(())
}
