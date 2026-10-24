use super::*;

#[test]
fn appearance_limitations_default_for_old_payloads_and_print_in_window_output() -> anyhow::Result<()>
{
    let mut window: WindowInfo = serde_json::from_value(serde_json::json!({
        "id": 1, "title": "Herdr UI", "active": true, "fullscreen": false,
        "width": 800, "height": 600, "workspace": null, "hosts": []
    }))?;
    assert!(window.appearance_limitations.is_empty());
    window
        .appearance_limitations
        .push("No usable blur protocol".into());
    let mut text = String::new();
    describe_window(&window, &mut text);
    assert!(text.contains("    appearance: No usable blur protocol\n"));
    let json = serde_json::to_value(&window)?;
    assert_eq!(
        json["appearance_limitations"],
        serde_json::json!(["No usable blur protocol"])
    );
    assert_eq!(serde_json::from_value::<WindowInfo>(json)?, window);
    Ok(())
}
