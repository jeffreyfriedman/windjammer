use smallvec::{SmallVec, smallvec};

use windjammer_ui::platform::capability_impl::{Filesystem, GPS, Camera, Clipboard, Notifications, Location, Notification, CameraImage};


fn demo_filesystem() {
    println!("📁 Filesystem API Demo");
    println!("=" * 40);
    let fs: SmallVec<[_; 4]> = Filesystem::new().into();
    fs.request_permission("/tmp").unwrap();
    println!("✅ Permission granted for /tmp");
    let path = "/tmp/windjammer_test.txt";
    fs.write_text(path, "Hello from Windjammer UI!").unwrap();
    println!(format!("✅ Wrote file: {}", path));
    let content = fs.read_text(path).unwrap();
    println!(format!("✅ Read file: "{}"", content));
    println!("✅ File exists: {}", fs.exists(path));
    let files = fs.list_dir("/tmp").unwrap();
    println!("✅ Files in /tmp: {} files", files.len());
    println!("")
}

fn demo_gps() {
    println!("📍 GPS/Location API Demo");
    println!("=" * 40);
    let gps: SmallVec<[_; 4]> = GPS::new().into();
    gps.enable().unwrap();
    println!("✅ GPS enabled");
    let location = Location { latitude: 37.7749, longitude: -122.4194, altitude: Some(10.0), accuracy: 5.0, timestamp: 1234567890 };
    gps.set_location(location);
    println!("✅ Location set (simulated)");
    let current = gps.get_location().unwrap();
    println!("✅ Current location:");
    println!("   Latitude: {current.latitude}");
    println!("   Longitude: {current.longitude}");
    println!("   Altitude: {current.altitude:?}");
    println!("   Accuracy: ±{current.accuracy}m");
    println!("")
}

fn demo_camera() {
    println!("📷 Camera API Demo");
    println!("=" * 40);
    let camera: SmallVec<[_; 4]> = Camera::new().into();
    camera.request_permission().unwrap();
    println!("✅ Camera permission granted");
    let image = camera.capture().unwrap();
    println!("✅ Image captured:");
    println!("   Size: {}x{}", image.width, image.height);
    println!("   Format: {image.format:?}");
    println!("   Data size: {} bytes", image.data::len());
    camera.capture().unwrap();
    println!("✅ Second image captured");
    let images = camera.get_captured_images();
    println!("✅ Total images: {images.len()}");
    println!("")
}

#[inline]
fn demo_clipboard() {
    println!("📋 Clipboard API Demo");
    println!("=" * 40);
    let clipboard: SmallVec<[_; 4]> = Clipboard::new().into();
    clipboard.write_text("Windjammer UI is awesome!").unwrap();
    println!("✅ Wrote to clipboard");
    let content = clipboard.read_text().unwrap();
    println!(format!("✅ Read from clipboard: "{}"", content));
    println!("✅ Has content: {clipboard.has_content()}");
    println!("")
}

fn demo_notifications() {
    println!("🔔 Notifications API Demo");
    println!("=" * 40);
    let notifications: SmallVec<[_; 4]> = Notifications::new().into();
    notifications.request_permission().unwrap();
    println!("✅ Notification permission granted");
    notifications.send(Notification { title: "Welcome!".to_string(), body: "Windjammer UI is running".to_string(), icon: Some("icon.png".to_string()) }).unwrap();
    println!("✅ Notification sent: Welcome");
    notifications.send(Notification { title: "New Message".to_string(), body: "You have 3 unread messages".to_string(), icon: None }).unwrap();
    println!("✅ Notification sent: New Message");
    let sent = notifications.get_sent();
    println!("✅ Total notifications sent: {sent.len()}");
    println!("")
}

fn main() {
    println!("=== Platform Capabilities Example ===
");
    demo_filesystem();
    demo_gps();
    demo_camera();
    demo_clipboard();
    demo_notifications();
    println!("🎯 Key Features:");
    println!("  ✅ Permission-based security model");
    println!("  ✅ Cross-platform API (Web, Desktop, Mobile)");
    println!("  ✅ Filesystem access with path restrictions");
    println!("  ✅ GPS/Location services");
    println!("  ✅ Camera capture");
    println!("  ✅ Clipboard read/write");
    println!("  ✅ Native notifications");
    println!("
🔒 Security:");
    println!("  • All capabilities require explicit permission");
    println!("  • Filesystem access is path-restricted");
    println!("  • Follows principle of least privilege");
    println!("  • Permission denials are graceful");
    println!("
🌍 Platform Support:");
    println!("  • Web: Uses Web APIs (File API, Geolocation API, etc)");
    println!("  • Desktop: Native file access, system notifications");
    println!("  • Mobile: Native APIs (iOS/Android)")
}

