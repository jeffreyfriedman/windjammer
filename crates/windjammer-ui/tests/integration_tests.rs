// Integration Tests for Windjammer UI Systems
// Tests all major editor systems to ensure they work correctly

use windjammer_ui::undo_redo::*;

// ============================================================================
// UNDO/REDO SYSTEM TESTS
// ============================================================================

#[test]
fn test_undo_redo_transform_command() {
    let mut manager = UndoRedoManager::new();
    
    // Create and execute a transform command
    let cmd = CommandBuilder::transform("TestObject")
        .old_position(0.0, 0.0, 0.0)
        .new_position(5.0, 10.0, 15.0)
        .old_rotation(0.0, 0.0, 0.0)
        .new_rotation(45.0, 90.0, 0.0)
        .old_scale(1.0, 1.0, 1.0)
        .new_scale(2.0, 2.0, 2.0)
        .build();
    
    assert!(manager.execute(cmd).is_ok());
    assert!(manager.can_undo());
    assert!(!manager.can_redo());
    assert_eq!(manager.undo_count(), 1);
    
    // Undo
    assert!(manager.undo().is_ok());
    assert!(!manager.can_undo());
    assert!(manager.can_redo());
    assert_eq!(manager.redo_count(), 1);
    
    // Redo
    assert!(manager.redo().is_ok());
    assert!(manager.can_undo());
    assert!(!manager.can_redo());
}

#[test]
fn test_undo_redo_file_edit_command() {
    let mut manager = UndoRedoManager::new();
    
    let cmd = CommandBuilder::file_edit("test.wj")
        .old_content("fn main() { }".to_string())
        .new_content("fn main() { println!(\"Hello\"); }".to_string())
        .build();
    
    assert!(manager.execute(cmd).is_ok());
    assert_eq!(manager.get_undo_description(), Some("Edit test.wj".to_string()));
    
    assert!(manager.undo().is_ok());
    assert_eq!(manager.get_redo_description(), Some("Edit test.wj".to_string()));
}

#[test]
fn test_undo_redo_create_delete_object() {
    let mut manager = UndoRedoManager::new();
    
    // Create object
    let create_cmd = CommandBuilder::create_object(
        "Enemy",
        "{type: \"Enemy\", health: 100}".to_string(),
    );
    assert!(manager.execute(create_cmd).is_ok());
    
    // Delete object
    let delete_cmd = CommandBuilder::delete_object(
        "Enemy",
        "{type: \"Enemy\", health: 100}".to_string(),
    );
    assert!(manager.execute(delete_cmd).is_ok());
    
    assert_eq!(manager.undo_count(), 2);
    
    // Undo delete (recreates object)
    assert!(manager.undo().is_ok());
    assert_eq!(manager.undo_count(), 1);
    
    // Undo create (removes object)
    assert!(manager.undo().is_ok());
    assert_eq!(manager.undo_count(), 0);
}

#[test]
fn test_undo_redo_property_change() {
    let mut manager = UndoRedoManager::new();
    
    let cmd = CommandBuilder::property_change(
        "Player",
        "health",
        "100".to_string(),
        "150".to_string(),
    );
    
    assert!(manager.execute(cmd).is_ok());
    assert_eq!(manager.get_undo_description(), Some("Change Player.health".to_string()));
}

#[test]
fn test_undo_redo_history_limit() {
    let mut manager = UndoRedoManager::with_max_history(5);
    
    // Execute 10 commands
    for i in 0..10 {
        let cmd = CommandBuilder::transform(&format!("Object{}", i))
            .old_position(0.0, 0.0, 0.0)
            .new_position(i as f32, 0.0, 0.0)
            .build();
        assert!(manager.execute(cmd).is_ok());
    }
    
    // Should only keep last 5
    assert_eq!(manager.undo_count(), 5);
    
    // Verify we can undo 5 times
    for _ in 0..5 {
        assert!(manager.undo().is_ok());
    }
    
    // No more undos available
    assert!(!manager.can_undo());
}

#[test]
fn test_undo_redo_clear_history() {
    let mut manager = UndoRedoManager::new();
    
    // Add some commands
    for i in 0..3 {
        let cmd = CommandBuilder::transform(&format!("Obj{}", i))
            .old_position(0.0, 0.0, 0.0)
            .new_position(1.0, 0.0, 0.0)
            .build();
        manager.execute(cmd).unwrap();
    }
    
    assert_eq!(manager.undo_count(), 3);
    
    // Clear history
    manager.clear();
    
    assert_eq!(manager.undo_count(), 0);
    assert_eq!(manager.redo_count(), 0);
    assert!(!manager.can_undo());
    assert!(!manager.can_redo());
}

#[test]
fn test_undo_redo_multiple_operations() {
    let mut manager = UndoRedoManager::new();
    
    // Execute multiple different commands
    let cmd1 = CommandBuilder::transform("Player")
        .old_position(0.0, 0.0, 0.0)
        .new_position(1.0, 0.0, 0.0)
        .build();
    manager.execute(cmd1).unwrap();
    
    let cmd2 = CommandBuilder::file_edit("main.wj")
        .old_content("old".to_string())
        .new_content("new".to_string())
        .build();
    manager.execute(cmd2).unwrap();
    
    let cmd3 = CommandBuilder::property_change(
        "Enemy",
        "speed",
        "5.0".to_string(),
        "10.0".to_string(),
    );
    manager.execute(cmd3).unwrap();
    
    // Verify history
    let history = manager.get_undo_history();
    assert_eq!(history.len(), 3);
    assert_eq!(history[0], "Transform Player");
    assert_eq!(history[1], "Edit main.wj");
    assert_eq!(history[2], "Change Enemy.speed");
    
    // Undo all
    manager.undo().unwrap();
    manager.undo().unwrap();
    manager.undo().unwrap();
    
    // Verify redo history
    let redo_history = manager.get_redo_history();
    assert_eq!(redo_history.len(), 3);
}

#[test]
fn test_undo_redo_invalidates_redo_stack() {
    let mut manager = UndoRedoManager::new();
    
    // Execute command
    let cmd1 = CommandBuilder::transform("Obj1")
        .old_position(0.0, 0.0, 0.0)
        .new_position(1.0, 0.0, 0.0)
        .build();
    manager.execute(cmd1).unwrap();
    
    // Undo
    manager.undo().unwrap();
    assert!(manager.can_redo());
    
    // Execute new command (should clear redo stack)
    let cmd2 = CommandBuilder::transform("Obj2")
        .old_position(0.0, 0.0, 0.0)
        .new_position(2.0, 0.0, 0.0)
        .build();
    manager.execute(cmd2).unwrap();
    
    // Redo should no longer be available
    assert!(!manager.can_redo());
    assert_eq!(manager.redo_count(), 0);
}

// ============================================================================
// SCENE GIZMOS TESTS (Data Model)
// ============================================================================

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
mod gizmo_tests {
    use windjammer_ui::scene_gizmos::*;
    
    #[test]
    fn test_transform_creation() {
        let transform = Transform::new();
        assert_eq!(transform.position, [0.0, 0.0, 0.0]);
        assert_eq!(transform.rotation, [0.0, 0.0, 0.0]);
        assert_eq!(transform.scale, [1.0, 1.0, 1.0]);
    }
    
    #[test]
    fn test_transform_with_builders() {
        let transform = Transform::new()
            .with_position(1.0, 2.0, 3.0)
            .with_rotation(45.0, 90.0, 0.0)
            .with_scale(2.0, 2.0, 2.0);
        
        assert_eq!(transform.position, [1.0, 2.0, 3.0]);
        assert_eq!(transform.rotation, [45.0, 90.0, 0.0]);
        assert_eq!(transform.scale, [2.0, 2.0, 2.0]);
    }
    
    #[test]
    fn test_transform_translate() {
        let mut transform = Transform::new();
        transform.translate([5.0, 10.0, 15.0]);
        
        assert_eq!(transform.position, [5.0, 10.0, 15.0]);
    }
    
    #[test]
    fn test_transform_rotate() {
        let mut transform = Transform::new();
        transform.rotate([45.0, 90.0, 180.0]);
        
        assert_eq!(transform.rotation, [45.0, 90.0, 180.0]);
    }
    
    #[test]
    fn test_transform_scale() {
        let mut transform = Transform::new()
            .with_scale(1.0, 1.0, 1.0);
        transform.scale_by([2.0, 3.0, 4.0]);
        
        assert_eq!(transform.scale, [2.0, 3.0, 4.0]);
    }
    
    #[test]
    fn test_transform_combined_operations() {
        let mut transform = Transform::new();
        
        transform.translate([1.0, 0.0, 0.0]);
        transform.rotate([0.0, 45.0, 0.0]);
        transform.scale_by([2.0, 1.0, 1.0]);
        
        assert_eq!(transform.position, [1.0, 0.0, 0.0]);
        assert_eq!(transform.rotation, [0.0, 45.0, 0.0]);
        assert_eq!(transform.scale, [2.0, 1.0, 1.0]);
    }
    
    #[test]
    fn test_gizmo_system_creation() {
        let gizmo = GizmoSystem::new();
        assert_eq!(gizmo.get_mode(), GizmoMode::Translate);
        assert!(!gizmo.is_snap_enabled());
    }
    
    #[test]
    fn test_gizmo_mode_switching() {
        let mut gizmo = GizmoSystem::new();
        
        gizmo.set_mode(GizmoMode::Rotate);
        assert_eq!(gizmo.get_mode(), GizmoMode::Rotate);
        
        gizmo.set_mode(GizmoMode::Scale);
        assert_eq!(gizmo.get_mode(), GizmoMode::Scale);
        
        gizmo.set_mode(GizmoMode::Translate);
        assert_eq!(gizmo.get_mode(), GizmoMode::Translate);
    }
    
    #[test]
    fn test_gizmo_snap_toggle() {
        let mut gizmo = GizmoSystem::new();
        
        assert!(!gizmo.is_snap_enabled());
        
        gizmo.set_snap_enabled(true);
        assert!(gizmo.is_snap_enabled());
        
        gizmo.set_snap_enabled(false);
        assert!(!gizmo.is_snap_enabled());
    }
}

// ============================================================================
// ASSET BROWSER TESTS (Data Model)
// ============================================================================

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
mod asset_tests {
    use windjammer_ui::asset_browser::*;
    
    #[test]
    fn test_asset_type_from_extension() {
        assert_eq!(AssetType::from_extension("png"), AssetType::Image);
        assert_eq!(AssetType::from_extension("jpg"), AssetType::Image);
        assert_eq!(AssetType::from_extension("obj"), AssetType::Model);
        assert_eq!(AssetType::from_extension("mp3"), AssetType::Audio);
        assert_eq!(AssetType::from_extension("wj"), AssetType::Script);
        assert_eq!(AssetType::from_extension("mat"), AssetType::Material);
        assert_eq!(AssetType::from_extension("unknown"), AssetType::Other);
    }
    
    #[test]
    fn test_asset_type_icon() {
        assert_eq!(AssetType::Image.icon(), "🖼️");
        assert_eq!(AssetType::Model.icon(), "🎲");
        assert_eq!(AssetType::Audio.icon(), "🔊");
        assert_eq!(AssetType::Script.icon(), "📜");
    }
    
    #[test]
    fn test_view_mode() {
        let grid = ViewMode::Grid;
        let list = ViewMode::List;
        
        assert_ne!(grid, list);
    }
    
    #[test]
    fn test_sort_by() {
        let sort_name = SortBy::Name;
        let sort_type = SortBy::Type;
        let sort_size = SortBy::Size;
        let sort_modified = SortBy::Modified;
        
        assert_ne!(sort_name, sort_type);
        assert_ne!(sort_size, sort_modified);
    }
}

// ============================================================================
// BUILD SYSTEM TESTS (Data Model)
// ============================================================================

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
mod build_tests {
    use windjammer_ui::build_system::*;
    
    #[test]
    fn test_build_status() {
        let idle = BuildStatus::Idle;
        let compiling = BuildStatus::Compiling;
        let running = BuildStatus::Running;
        let stopping = BuildStatus::Stopping;
        let failed = BuildStatus::Failed("error".to_string());
        
        assert_ne!(idle, compiling);
        assert_ne!(running, stopping);
        
        match failed {
            BuildStatus::Failed(msg) => assert_eq!(msg, "error"),
            _ => panic!("Expected Failed status"),
        }
    }
    
    #[test]
    fn test_build_target() {
        let native = BuildTarget::Native;
        let wasm = BuildTarget::Wasm;
        let release = BuildTarget::Release;
        
        assert_ne!(native, wasm);
        assert_ne!(wasm, release);
    }
    
    #[test]
    fn test_build_config_default() {
        let config = BuildConfig::default();
        
        assert_eq!(config.target, BuildTarget::Native);
        assert_eq!(config.optimization_level, 0);
        assert!(config.enable_debug_symbols);
        assert!(!config.enable_hot_reload);
    }
    
    #[test]
    fn test_build_system_creation() {
        let build_system = BuildSystem::new();
        
        assert_eq!(build_system.get_status(), BuildStatus::Idle);
        assert!(!build_system.is_running());
        assert!(!build_system.is_compiling());
    }
    
    #[test]
    fn test_build_system_config() {
        let mut build_system = BuildSystem::new();
        
        let config = BuildConfig {
            target: BuildTarget::Wasm,
            optimization_level: 2,
            enable_debug_symbols: false,
            enable_hot_reload: true,
        };
        
        build_system.set_config(config.clone());
        assert_eq!(build_system.get_config().target, BuildTarget::Wasm);
        assert_eq!(build_system.get_config().optimization_level, 2);
    }
    
    #[test]
    fn test_build_system_output() {
        let build_system = BuildSystem::new();
        let output = build_system.get_output();
        
        assert!(output.is_empty());
    }
}

// ============================================================================
// INTEGRATION TESTS
// ============================================================================

#[test]
fn test_undo_redo_with_transform_workflow() {
    // Simulate a typical editing workflow
    let mut manager = UndoRedoManager::new();
    
    // Move object
    let move_cmd = CommandBuilder::transform("Player")
        .old_position(0.0, 0.0, 0.0)
        .new_position(5.0, 0.0, 0.0)
        .build();
    manager.execute(move_cmd).unwrap();
    
    // Rotate object
    let rotate_cmd = CommandBuilder::transform("Player")
        .old_rotation(0.0, 0.0, 0.0)
        .new_rotation(0.0, 90.0, 0.0)
        .build();
    manager.execute(rotate_cmd).unwrap();
    
    // Scale object
    let scale_cmd = CommandBuilder::transform("Player")
        .old_scale(1.0, 1.0, 1.0)
        .new_scale(2.0, 2.0, 2.0)
        .build();
    manager.execute(scale_cmd).unwrap();
    
    // Note: Commands may be merged if executed quickly (within merge window)
    // So we might have 1-3 operations depending on timing
    let undo_count = manager.undo_count();
    assert!(undo_count >= 1 && undo_count <= 3, "Expected 1-3 operations, got {}", undo_count);
    
    // Undo all operations
    while manager.can_undo() {
        manager.undo().unwrap();
    }
    
    // All undone
    assert!(!manager.can_undo());
    let redo_count = manager.redo_count();
    assert!(redo_count >= 1 && redo_count <= 3, "Expected 1-3 redo operations, got {}", redo_count);
    
    // Redo all
    while manager.can_redo() {
        manager.redo().unwrap();
    }
    
    assert!(!manager.can_redo());
    assert_eq!(manager.undo_count(), undo_count);
}

#[test]
fn test_complete_editor_workflow() {
    // Simulate a complete editing session
    let mut undo_manager = UndoRedoManager::with_max_history(50);
    
    // 1. Create a new object
    let create = CommandBuilder::create_object(
        "Enemy1",
        "{type: \"Enemy\"}".to_string(),
    );
    undo_manager.execute(create).unwrap();
    
    // 2. Move it
    let move_cmd = CommandBuilder::transform("Enemy1")
        .old_position(0.0, 0.0, 0.0)
        .new_position(10.0, 0.0, 5.0)
        .build();
    undo_manager.execute(move_cmd).unwrap();
    
    // 3. Edit a property
    let prop = CommandBuilder::property_change(
        "Enemy1",
        "health",
        "100".to_string(),
        "150".to_string(),
    );
    undo_manager.execute(prop).unwrap();
    
    // 4. Edit a file
    let file_edit = CommandBuilder::file_edit("enemy.wj")
        .old_content("// old code".to_string())
        .new_content("// new code".to_string())
        .build();
    undo_manager.execute(file_edit).unwrap();
    
    // Verify all operations recorded
    assert_eq!(undo_manager.undo_count(), 4);
    
    // Undo last two operations
    undo_manager.undo().unwrap(); // Undo file edit
    undo_manager.undo().unwrap(); // Undo property change
    
    // Continue with new operation (clears redo)
    let new_obj = CommandBuilder::create_object(
        "Enemy2",
        "{type: \"Enemy\"}".to_string(),
    );
    undo_manager.execute(new_obj).unwrap();
    
    // Redo should be cleared
    assert_eq!(undo_manager.redo_count(), 0);
    assert_eq!(undo_manager.undo_count(), 3);
}
