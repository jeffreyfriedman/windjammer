//! Filter dependency metadata so locally-defined types always win during inference.

use std::collections::HashSet;

/// Unqualified struct name from an associated-method registry key (`Type::method` or `mod::Type::method`).
pub fn struct_name_from_method_key(name: &str) -> Option<&str> {
    let (parent, method) = name.rsplit_once("::")?;
    if method.contains("::") {
        return None;
    }
    let struct_name = parent.rsplit("::").next().unwrap_or(parent);
    if !struct_name.chars().next().is_some_and(|c| c.is_uppercase()) {
        return None;
    }
    Some(struct_name)
}

/// True when `name` is an associated method on a struct defined in the current crate.
pub fn signature_targets_local_struct(name: &str, local_struct_names: &HashSet<String>) -> bool {
    struct_name_from_method_key(name).is_some_and(|s| local_struct_names.contains(s))
}

/// Remove dependency signatures for types the current crate defines locally.
///
/// Module-qualified keys (`quick_start::voxel_scene::VoxelScene::new`) are retained:
/// alphabetical codegen order can visit call sites before the defining module, and
/// those paths disambiguate homonyms like bare `VoxelScene::new`.
pub fn drop_dependency_signatures_for_local_types<T>(
    signatures: &mut std::collections::HashMap<String, T>,
    local_struct_names: &HashSet<String>,
) {
    signatures.retain(|name, _| {
        if !signature_targets_local_struct(name, local_struct_names) {
            return true;
        }
        is_module_qualified_method_key(name)
    });
}

/// True for registry keys with at least one module segment before `Type::method`.
fn is_module_qualified_method_key(name: &str) -> bool {
    let Some((parent, method)) = name.rsplit_once("::") else {
        return false;
    };
    if method.contains("::") {
        return false;
    }
    parent.contains("::")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_qualified_local_struct_is_detected() {
        let locals: HashSet<String> = ["DialogueNodeTree".to_string()].into_iter().collect();
        assert!(signature_targets_local_struct(
            "dialogue::tree::DialogueNodeTree::get_node",
            &locals
        ));
        assert!(signature_targets_local_struct(
            "DialogueNodeTree::get_node",
            &locals
        ));
        assert!(!signature_targets_local_struct(
            "dialogue_tree::DialogueTree::get_node",
            &locals
        ));
    }

    #[test]
    fn drop_keeps_module_qualified_local_type_metadata() {
        let locals: HashSet<String> = ["VoxelScene".to_string()].into_iter().collect();
        let mut sigs = std::collections::HashMap::from([
            (
                "VoxelScene::new".to_string(),
                "bare_homonym",
            ),
            (
                "quick_start::voxel_scene::VoxelScene::new".to_string(),
                "module_qualified",
            ),
        ]);
        drop_dependency_signatures_for_local_types(&mut sigs, &locals);
        assert!(!sigs.contains_key("VoxelScene::new"));
        assert_eq!(
            sigs.get("quick_start::voxel_scene::VoxelScene::new"),
            Some(&"module_qualified")
        );
    }
}
