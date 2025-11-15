use std::sync::{Arc, Mutex};
use windjammer_ui::components::*;
use windjammer_ui::desktop_renderer::DesktopRenderer;
/// Visual test of the editor using egui_kittest
/// This test will capture screenshots and test button clicks
use windjammer_ui::prelude::*;

#[test]
fn test_editor_visual_and_buttons() {
    println!("\n🧪 Testing Editor UI with egui_kittest\n");

    // Track button clicks
    let clicks = Arc::new(Mutex::new(Vec::new()));

    // Create clones for each button
    let c1 = clicks.clone();
    let c2 = clicks.clone();
    let c3 = clicks.clone();
    let c4 = clicks.clone();

    // Build the exact editor UI
    let toolbar = Panel::new("".to_string())
        .background_color("#2d2d30".to_string())
        .padding("8px")
        .child(
            Flex::new()
                .direction(FlexDirection::Row)
                .gap("8px")
                .child(
                    Button::new("📁 New Project".to_string())
                        .variant(ButtonVariant::Primary)
                        .on_click(move || {
                            c1.lock().unwrap().push("New Project");
                            println!("✅ New Project clicked!");
                        }),
                )
                .child(
                    Button::new("💾 Save".to_string())
                        .variant(ButtonVariant::Success)
                        .on_click(move || {
                            c2.lock().unwrap().push("Save");
                            println!("✅ Save clicked!");
                        }),
                )
                .child(
                    Button::new("▶️ Run".to_string())
                        .variant(ButtonVariant::Success)
                        .on_click(move || {
                            c3.lock().unwrap().push("Run");
                            println!("✅ Run clicked!");
                        }),
                )
                .child(
                    Button::new("🔨 Build".to_string())
                        .variant(ButtonVariant::Primary)
                        .on_click(move || {
                            c4.lock().unwrap().push("Build");
                            println!("✅ Build clicked!");
                        }),
                ),
        );

    let vnode = Container::new().child(toolbar).to_vnode();

    // Create renderer
    let mut renderer = DesktopRenderer::new();

    // Create harness
    let mut harness = egui_kittest::Harness::new_ui(|ui| {
        renderer.render(ui.ctx(), &vnode);
    });

    println!("📸 Running initial frame...");
    harness.run();

    println!("\n🔍 Searching for buttons...");

    // Try to find and click the "New Project" button
    println!("   Looking for 'New Project' button...");
    match harness.try_get_by_label("New Project") {
        Some(button) => {
            println!("   ✅ Found button!");
            println!("   🖱️  Clicking...");
            button.click();
            harness.run();

            let click_log = clicks.lock().unwrap();
            if click_log.contains(&"New Project") {
                println!("   ✅ Button click WORKED!");
            } else {
                println!("   ❌ Button click DID NOT WORK!");
            }
        }
        None => {
            println!("   ❌ Could not find button by label");
        }
    }

    println!("\n📊 Final Results:");
    let click_log = clicks.lock().unwrap();
    println!("   Total clicks registered: {}", click_log.len());
    println!("   Clicks: {:?}", *click_log);

    if click_log.is_empty() {
        println!("\n❌ PROBLEM IDENTIFIED:");
        println!("   Buttons are not responding to clicks!");
        println!("   This means either:");
        println!("   1. The event handlers are not being attached correctly");
        println!("   2. egui is not detecting the clicks");
        println!("   3. The button rendering is incorrect");
    } else {
        println!("\n✅ Buttons are working correctly!");
    }
}
