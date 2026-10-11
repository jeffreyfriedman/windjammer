use std::fs;
use std::path::Path;

use super::new_templates;

pub fn handle_new_command(name: &str, template: &str) -> Result<(), String> {
    // Validate project name
    if name.is_empty() {
        return Err("Project name cannot be empty".to_string());
    }

    if name.contains('/') || name.contains('\\') {
        return Err("Project name cannot contain path separators".to_string());
    }

    // Validate template
    let valid_templates = new_templates::TEMPLATE_NAMES;
    if !valid_templates.contains(&template) {
        return Err(format!(
            "Invalid template '{}'. Valid templates: {}",
            template,
            valid_templates.join(", ")
        ));
    }

    // Check if directory already exists
    if Path::new(name).exists() {
        return Err(format!("Directory '{}' already exists", name));
    }

    println!("Creating Windjammer project: {}", name);
    println!("  Template: {}", template);
    println!();

    // Create project from template
    create_project(name, template)?;

    println!();
    println!("✓ Project created successfully!");
    println!();
    println!("To get started:");
    println!("  cd {}", name);

    match template {
        "cli" | "web" => println!("  wj run src/main.wj"),
        "lib" => println!("  wj test"),
        "wasm" => {
            println!("  wj build --target wasm");
            println!("  cd www && python3 -m http.server 8000");
        }
        _ => {}
    }

    Ok(())
}

fn create_project(name: &str, template: &str) -> Result<(), String> {
    let project_path = Path::new(name);
    fs::create_dir_all(project_path.join("src"))
        .map_err(|e| format!("Failed to create project directory: {}", e))?;
    println!("  ✓ Created directory structure");

    // Templates are embedded in the binary: no filesystem lookup, cwd-independent.
    for (t, rel, body) in new_templates::EMBEDDED {
        if *t != template {
            continue;
        }
        let dest = project_path.join(rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create {}: {}", parent.display(), e))?;
        }
        fs::write(&dest, body.replace("{{PROJECT_NAME}}", name))
            .map_err(|e| format!("Failed to write {}: {}", rel, e))?;
        println!("  ✓ Created {}", rel);
    }

    init_git_repo(name)?;
    Ok(())
}

fn init_git_repo(project_name: &str) -> Result<(), String> {
    use std::process::Command;

    // Check if git is available
    let git_check = Command::new("git").arg("--version").output();

    if git_check.is_err() {
        // Git not available, skip
        return Ok(());
    }

    // Initialize git repository
    let output = Command::new("git")
        .arg("init")
        .current_dir(project_name)
        .output()
        .map_err(|e| format!("Failed to initialize git repository: {}", e))?;

    if output.status.success() {
        println!("  ✓ Initialized git repository");
    }

    Ok(())
}
