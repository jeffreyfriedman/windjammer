use std::sync::{Arc, Mutex};
use windjammer_ui::components::*;
use windjammer_ui::desktop_renderer::DesktopRenderer;
/// Test the actual editor UI with kittest
use windjammer_ui::prelude::*;
use windjammer_ui::simple_vnode::VNode;

#[test]
fn test_editor_layout_and_buttons() {
    println!("🧪 Testing editor layout and button clicks...");

    // Create the exact same UI as editor_simple.wj
    let click_log = Arc::new(Mutex::new(Vec::new()));

    let click_log_1 = click_log.clone();
    let click_log_2 = click_log.clone();
    let click_log_3 = click_log.clone();
    let click_log_4 = click_log.clone();

    let vnode = Container::new()
        .child(
            Panel::new("".to_string())
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
                                    click_log_1.lock().unwrap().push("New Project");
                                    println!("🎉 New Project clicked!");
                                }),
                        )
                        .child(
                            Button::new("💾 Save".to_string())
                                .variant(ButtonVariant::Success)
                                .on_click(move || {
                                    click_log_2.lock().unwrap().push("Save");
                                    println!("💾 Save clicked!");
                                }),
                        )
                        .child(
                            Button::new("▶️ Run".to_string())
                                .variant(ButtonVariant::Success)
                                .on_click(move || {
                                    click_log_3.lock().unwrap().push("Run");
                                    println!("▶️ Run clicked!");
                                }),
                        )
                        .child(
                            Button::new("🔨 Build".to_string())
                                .variant(ButtonVariant::Primary)
                                .on_click(move || {
                                    click_log_4.lock().unwrap().push("Build");
                                    println!("🔨 Build clicked!");
                                }),
                        ),
                ),
        )
        .to_vnode();

    // Create renderer
    let mut renderer = DesktopRenderer::new();

    // Create kittest harness
    let mut harness = egui_kittest::Harness::new_ui(|ui| {
        renderer.render(ui.ctx(), &vnode);
    });

    // Run initial frame
    println!("📸 Taking initial screenshot...");
    harness.run();

    // Try to take a screenshot
    match harness.try_wgpu_snapshot("editor_initial") {
        Some(image) => {
            println!(
                "✅ Screenshot captured: {}x{}",
                image.width(),
                image.height()
            );
            // Save it
            if let Err(e) = image.save("/tmp/editor_initial.png") {
                println!("⚠️  Failed to save screenshot: {}", e);
            } else {
                println!("✅ Screenshot saved to /tmp/editor_initial.png");
            }
        }
        None => {
            println!(
                "⚠️  Could not capture wgpu screenshot (might need to run with --features=wgpu)"
            );
        }
    }

    // Try to find buttons
    println!("\n🔍 Looking for buttons...");

    // Try to click "New Project" button
    if let Some(button) = harness.try_get_by_label("📁 New Project") {
        println!("✅ Found 'New Project' button, clicking...");
        button.click();
        harness.run();

        let clicks = click_log.lock().unwrap();
        if clicks.contains(&"New Project") {
            println!("✅ Button click was registered!");
        } else {
            println!("❌ Button click was NOT registered!");
        }
    } else {
        println!("❌ Could not find 'New Project' button");

        // Try alternative search methods
        println!("🔍 Trying to find button by partial text...");
        if let Some(button) = harness.try_get_by_label("New Project") {
            println!("✅ Found button without emoji");
            button.click();
            harness.run();
        } else {
            println!("❌ Still can't find button");
        }
    }

    // Take another screenshot after click
    println!("\n📸 Taking post-click screenshot...");
    harness.run();
    match harness.try_wgpu_snapshot("editor_after_click") {
        Some(image) => {
            if let Err(e) = image.save("/tmp/editor_after_click.png") {
                println!("⚠️  Failed to save screenshot: {}", e);
            } else {
                println!("✅ Screenshot saved to /tmp/editor_after_click.png");
            }
        }
        None => {}
    }

    // Report results
    let clicks = click_log.lock().unwrap();
    println!("\n📊 Test Results:");
    println!("   Clicks registered: {:?}", *clicks);
    println!("   Total clicks: {}", clicks.len());

    if clicks.is_empty() {
        println!("\n❌ BUTTONS ARE NOT WORKING");
        println!("   The handlers are not being triggered by egui clicks.");
    } else {
        println!("\n✅ BUTTONS ARE WORKING");
    }
}
