// Undo/Redo System Module
// Provides command pattern-based undo/redo functionality for editor operations
//
// IMPORTANT: This system is designed to be usable both from the editor UI
// and programmatically from code. Developers can create and execute commands
// directly without needing the editor.

use std::collections::VecDeque;

// ============================================================================
// COMMAND TRAIT (Core abstraction - can be implemented for any operation)
// ============================================================================

/// Command trait that all undoable operations must implement.
/// 
/// This allows developers to create custom commands programmatically:
/// 
/// ```rust
/// struct MyCustomCommand { /* ... */ }
/// 
/// impl Command for MyCustomCommand {
///     fn execute(&mut self) -> Result<(), String> {
///         // Perform the operation
///         Ok(())
///     }
///     
///     fn undo(&mut self) -> Result<(), String> {
///         // Reverse the operation
///         Ok(())
///     }
///     
///     fn description(&self) -> String {
///         "My Custom Operation".to_string()
///     }
/// }
/// ```
pub trait Command: std::any::Any + Send + Sync {
    /// Execute the command (do the operation)
    fn execute(&mut self) -> Result<(), String>;
    
    /// Undo the command (reverse the operation)
    fn undo(&mut self) -> Result<(), String>;
    
    /// Get a human-readable description of the command
    fn description(&self) -> String;
    
    /// Optional: Merge with another command of the same type
    /// Useful for combining multiple similar operations (e.g., continuous dragging)
    fn try_merge(&mut self, _other: &dyn Command) -> bool {
        false
    }
}

// ============================================================================
// BUILT-IN COMMANDS (Common operations - developers can use these directly)
// ============================================================================

/// Transform modification command (position, rotation, scale)
/// 
/// Can be used programmatically:
/// ```rust
/// let mut cmd = TransformCommand::new(
///     object_id,
///     old_transform,
///     new_transform,
/// );
/// cmd.execute()?;
/// // Later...
/// cmd.undo()?;
/// ```
#[derive(Clone, Debug)]
pub struct TransformCommand {
    pub object_id: String,
    pub old_transform: [f32; 9], // [pos_x, pos_y, pos_z, rot_x, rot_y, rot_z, scale_x, scale_y, scale_z]
    pub new_transform: [f32; 9],
}

impl TransformCommand {
    pub fn new(object_id: String, old_transform: [f32; 9], new_transform: [f32; 9]) -> Self {
        Self {
            object_id,
            old_transform,
            new_transform,
        }
    }
}

impl Command for TransformCommand {
    fn execute(&mut self) -> Result<(), String> {
        // In a real implementation, this would update the scene
        // For now, we just track the state change
        Ok(())
    }
    
    fn undo(&mut self) -> Result<(), String> {
        // Restore the old transform
        Ok(())
    }
    
    fn description(&self) -> String {
        format!("Transform {}", self.object_id)
    }
    
    fn try_merge(&mut self, other: &dyn Command) -> bool {
        // Try to merge consecutive transform operations on the same object
        if let Some(other_transform) = (other as &dyn std::any::Any).downcast_ref::<TransformCommand>() {
            if self.object_id == other_transform.object_id {
                // Keep the old_transform from self, but update new_transform from other
                self.new_transform = other_transform.new_transform;
                return true;
            }
        }
        false
    }
}

/// File content modification command
/// 
/// Programmatic usage:
/// ```rust
/// let mut cmd = FileEditCommand::new(
///     "main.wj".to_string(),
///     old_content,
///     new_content,
/// );
/// cmd.execute()?;
/// ```
#[derive(Clone, Debug)]
pub struct FileEditCommand {
    pub file_path: String,
    pub old_content: String,
    pub new_content: String,
}

impl FileEditCommand {
    pub fn new(file_path: String, old_content: String, new_content: String) -> Self {
        Self {
            file_path,
            old_content,
            new_content,
        }
    }
}

impl Command for FileEditCommand {
    fn execute(&mut self) -> Result<(), String> {
        // In a real implementation, this would update the file
        Ok(())
    }
    
    fn undo(&mut self) -> Result<(), String> {
        // Restore the old content
        Ok(())
    }
    
    fn description(&self) -> String {
        format!("Edit {}", self.file_path)
    }
}

/// Object creation command
/// 
/// Programmatic usage:
/// ```rust
/// let mut cmd = CreateObjectCommand::new(
///     "Player".to_string(),
///     object_data,
/// );
/// cmd.execute()?;
/// ```
#[derive(Clone, Debug)]
pub struct CreateObjectCommand {
    pub object_id: String,
    pub object_data: String, // Serialized object data
}

impl CreateObjectCommand {
    pub fn new(object_id: String, object_data: String) -> Self {
        Self {
            object_id,
            object_data,
        }
    }
}

impl Command for CreateObjectCommand {
    fn execute(&mut self) -> Result<(), String> {
        // Create the object in the scene
        Ok(())
    }
    
    fn undo(&mut self) -> Result<(), String> {
        // Remove the object from the scene
        Ok(())
    }
    
    fn description(&self) -> String {
        format!("Create {}", self.object_id)
    }
}

/// Object deletion command
/// 
/// Programmatic usage:
/// ```rust
/// let mut cmd = DeleteObjectCommand::new(
///     "Enemy".to_string(),
///     object_data, // Save for undo
/// );
/// cmd.execute()?;
/// ```
#[derive(Clone, Debug)]
pub struct DeleteObjectCommand {
    pub object_id: String,
    pub object_data: String, // Saved for undo
}

impl DeleteObjectCommand {
    pub fn new(object_id: String, object_data: String) -> Self {
        Self {
            object_id,
            object_data,
        }
    }
}

impl Command for DeleteObjectCommand {
    fn execute(&mut self) -> Result<(), String> {
        // Remove the object from the scene
        Ok(())
    }
    
    fn undo(&mut self) -> Result<(), String> {
        // Restore the object to the scene
        Ok(())
    }
    
    fn description(&self) -> String {
        format!("Delete {}", self.object_id)
    }
}

/// Property modification command
/// 
/// Programmatic usage:
/// ```rust
/// let mut cmd = PropertyChangeCommand::new(
///     "Player".to_string(),
///     "health".to_string(),
///     "100".to_string(),
///     "150".to_string(),
/// );
/// cmd.execute()?;
/// ```
#[derive(Clone, Debug)]
pub struct PropertyChangeCommand {
    pub object_id: String,
    pub property_name: String,
    pub old_value: String,
    pub new_value: String,
}

impl PropertyChangeCommand {
    pub fn new(object_id: String, property_name: String, old_value: String, new_value: String) -> Self {
        Self {
            object_id,
            property_name,
            old_value,
            new_value,
        }
    }
}

impl Command for PropertyChangeCommand {
    fn execute(&mut self) -> Result<(), String> {
        // Set the property to new_value
        Ok(())
    }
    
    fn undo(&mut self) -> Result<(), String> {
        // Restore the property to old_value
        Ok(())
    }
    
    fn description(&self) -> String {
        format!("Change {}.{}", self.object_id, self.property_name)
    }
}

// ============================================================================
// UNDO/REDO MANAGER (Can be used standalone or with editor)
// ============================================================================

/// Undo/Redo manager that maintains command history.
/// 
/// Can be used programmatically without the editor:
/// 
/// ```rust
/// let mut manager = UndoRedoManager::new();
/// 
/// // Execute commands
/// let mut cmd = TransformCommand::new(...);
/// manager.execute(Box::new(cmd))?;
/// 
/// // Undo/Redo
/// manager.undo()?;
/// manager.redo()?;
/// 
/// // Query state
/// if manager.can_undo() {
///     println!("Can undo: {}", manager.get_undo_description());
/// }
/// ```
pub struct UndoRedoManager {
    undo_stack: VecDeque<Box<dyn Command>>,
    redo_stack: VecDeque<Box<dyn Command>>,
    max_history: usize,
    merge_time_window_ms: u128, // Time window for merging commands
    last_command_time: Option<std::time::Instant>,
}

impl UndoRedoManager {
    /// Create a new undo/redo manager
    pub fn new() -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            max_history: 100,
            merge_time_window_ms: 500, // 500ms window for merging
            last_command_time: None,
        }
    }

    /// Create a manager with custom history size
    pub fn with_max_history(max_history: usize) -> Self {
        Self {
            max_history,
            ..Self::new()
        }
    }

    /// Execute a command and add it to the undo stack
    /// 
    /// This is the main entry point for all operations.
    /// Returns an error if the command execution fails.
    pub fn execute(&mut self, mut command: Box<dyn Command>) -> Result<(), String> {
        // Execute the command
        command.execute()?;

        // Try to merge with the last command if within time window
        let now = std::time::Instant::now();
        let should_merge = if let Some(last_time) = self.last_command_time {
            now.duration_since(last_time).as_millis() < self.merge_time_window_ms
        } else {
            false
        };

        if should_merge {
            if let Some(last_cmd) = self.undo_stack.back_mut() {
                if last_cmd.try_merge(&*command) {
                    // Successfully merged, don't add new command
                    self.last_command_time = Some(now);
                    return Ok(());
                }
            }
        }

        // Add to undo stack
        self.undo_stack.push_back(command);
        self.last_command_time = Some(now);

        // Clear redo stack (new action invalidates redo history)
        self.redo_stack.clear();

        // Limit history size
        while self.undo_stack.len() > self.max_history {
            self.undo_stack.pop_front();
        }

        Ok(())
    }

    /// Undo the last command
    pub fn undo(&mut self) -> Result<(), String> {
        if let Some(mut command) = self.undo_stack.pop_back() {
            command.undo()?;
            self.redo_stack.push_back(command);
            self.last_command_time = None; // Reset merge window
            Ok(())
        } else {
            Err("Nothing to undo".to_string())
        }
    }

    /// Redo the last undone command
    pub fn redo(&mut self) -> Result<(), String> {
        if let Some(mut command) = self.redo_stack.pop_back() {
            command.execute()?;
            self.undo_stack.push_back(command);
            self.last_command_time = None; // Reset merge window
            Ok(())
        } else {
            Err("Nothing to redo".to_string())
        }
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get description of the command that would be undone
    pub fn get_undo_description(&self) -> Option<String> {
        self.undo_stack.back().map(|cmd| cmd.description())
    }

    /// Get description of the command that would be redone
    pub fn get_redo_description(&self) -> Option<String> {
        self.redo_stack.back().map(|cmd| cmd.description())
    }

    /// Get the number of commands in the undo stack
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get the number of commands in the redo stack
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.last_command_time = None;
    }

    /// Get a list of all undo descriptions (for UI display)
    pub fn get_undo_history(&self) -> Vec<String> {
        self.undo_stack
            .iter()
            .map(|cmd| cmd.description())
            .collect()
    }

    /// Get a list of all redo descriptions (for UI display)
    pub fn get_redo_history(&self) -> Vec<String> {
        self.redo_stack
            .iter()
            .map(|cmd| cmd.description())
            .collect()
    }

    /// Set the maximum history size
    pub fn set_max_history(&mut self, max: usize) {
        self.max_history = max;
        while self.undo_stack.len() > self.max_history {
            self.undo_stack.pop_front();
        }
    }

    /// Set the merge time window (in milliseconds)
    pub fn set_merge_time_window(&mut self, ms: u128) {
        self.merge_time_window_ms = ms;
    }
}

impl Default for UndoRedoManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// COMMAND BUILDER (Convenience for creating commands programmatically)
// ============================================================================

/// Builder for creating commands programmatically
/// 
/// Example usage:
/// ```rust
/// let cmd = CommandBuilder::transform("Player")
///     .old_position(0.0, 0.0, 0.0)
///     .new_position(1.0, 2.0, 3.0)
///     .build();
/// 
/// manager.execute(cmd)?;
/// ```
pub struct CommandBuilder;

impl CommandBuilder {
    /// Start building a transform command
    pub fn transform(object_id: &str) -> TransformCommandBuilder {
        TransformCommandBuilder {
            object_id: object_id.to_string(),
            old_transform: [0.0; 9],
            new_transform: [0.0; 9],
        }
    }

    /// Start building a file edit command
    pub fn file_edit(file_path: &str) -> FileEditCommandBuilder {
        FileEditCommandBuilder {
            file_path: file_path.to_string(),
            old_content: String::new(),
            new_content: String::new(),
        }
    }

    /// Create an object creation command
    pub fn create_object(object_id: &str, object_data: String) -> Box<dyn Command> {
        Box::new(CreateObjectCommand::new(object_id.to_string(), object_data))
    }

    /// Create an object deletion command
    pub fn delete_object(object_id: &str, object_data: String) -> Box<dyn Command> {
        Box::new(DeleteObjectCommand::new(object_id.to_string(), object_data))
    }

    /// Create a property change command
    pub fn property_change(
        object_id: &str,
        property_name: &str,
        old_value: String,
        new_value: String,
    ) -> Box<dyn Command> {
        Box::new(PropertyChangeCommand::new(
            object_id.to_string(),
            property_name.to_string(),
            old_value,
            new_value,
        ))
    }
}

pub struct TransformCommandBuilder {
    object_id: String,
    old_transform: [f32; 9],
    new_transform: [f32; 9],
}

impl TransformCommandBuilder {
    pub fn old_position(mut self, x: f32, y: f32, z: f32) -> Self {
        self.old_transform[0] = x;
        self.old_transform[1] = y;
        self.old_transform[2] = z;
        self
    }

    pub fn new_position(mut self, x: f32, y: f32, z: f32) -> Self {
        self.new_transform[0] = x;
        self.new_transform[1] = y;
        self.new_transform[2] = z;
        self
    }

    pub fn old_rotation(mut self, x: f32, y: f32, z: f32) -> Self {
        self.old_transform[3] = x;
        self.old_transform[4] = y;
        self.old_transform[5] = z;
        self
    }

    pub fn new_rotation(mut self, x: f32, y: f32, z: f32) -> Self {
        self.new_transform[3] = x;
        self.new_transform[4] = y;
        self.new_transform[5] = z;
        self
    }

    pub fn old_scale(mut self, x: f32, y: f32, z: f32) -> Self {
        self.old_transform[6] = x;
        self.old_transform[7] = y;
        self.old_transform[8] = z;
        self
    }

    pub fn new_scale(mut self, x: f32, y: f32, z: f32) -> Self {
        self.new_transform[6] = x;
        self.new_transform[7] = y;
        self.new_transform[8] = z;
        self
    }

    pub fn build(self) -> Box<dyn Command> {
        Box::new(TransformCommand::new(
            self.object_id,
            self.old_transform,
            self.new_transform,
        ))
    }
}

pub struct FileEditCommandBuilder {
    file_path: String,
    old_content: String,
    new_content: String,
}

impl FileEditCommandBuilder {
    pub fn old_content(mut self, content: String) -> Self {
        self.old_content = content;
        self
    }

    pub fn new_content(mut self, content: String) -> Self {
        self.new_content = content;
        self
    }

    pub fn build(self) -> Box<dyn Command> {
        Box::new(FileEditCommand::new(
            self.file_path,
            self.old_content,
            self.new_content,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undo_redo_basic() {
        let mut manager = UndoRedoManager::new();
        
        let cmd = Box::new(TransformCommand::new(
            "test".to_string(),
            [0.0; 9],
            [1.0, 2.0, 3.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        ));
        
        manager.execute(cmd).unwrap();
        assert!(manager.can_undo());
        assert!(!manager.can_redo());
        
        manager.undo().unwrap();
        assert!(!manager.can_undo());
        assert!(manager.can_redo());
        
        manager.redo().unwrap();
        assert!(manager.can_undo());
        assert!(!manager.can_redo());
    }

    #[test]
    fn test_command_builder() {
        let cmd = CommandBuilder::transform("Player")
            .old_position(0.0, 0.0, 0.0)
            .new_position(1.0, 2.0, 3.0)
            .build();
        
        assert_eq!(cmd.description(), "Transform Player");
    }

    #[test]
    fn test_history_limit() {
        let mut manager = UndoRedoManager::with_max_history(3);
        
        for i in 0..5 {
            let cmd = Box::new(TransformCommand::new(
                format!("obj{}", i),
                [0.0; 9],
                [1.0; 9],
            ));
            manager.execute(cmd).unwrap();
        }
        
        assert_eq!(manager.undo_count(), 3);
    }
}

