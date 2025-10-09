use std::fs;
use std::path::Path;

pub fn handle_new_command(name: &str, template: &str) -> Result<(), String> {
    // Validate project name
    if name.is_empty() {
        return Err("Project name cannot be empty".to_string());
    }

    if name.contains('/') || name.contains('\\') {
        return Err("Project name cannot contain path separators".to_string());
    }

    // Validate template
    let valid_templates = ["cli", "web", "lib", "wasm"];
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
    // Get template directory
    let exe_path =
        std::env::current_exe().map_err(|e| format!("Failed to get executable path: {}", e))?;

    let exe_dir = exe_path
        .parent()
        .ok_or_else(|| "Failed to get executable directory".to_string())?;

    // Try multiple locations for templates
    let possible_template_dirs = [
        exe_dir.join("templates").join(template),
        exe_dir.join("..").join("templates").join(template),
        exe_dir
            .join("..")
            .join("..")
            .join("templates")
            .join(template),
        // For development: relative to cargo workspace root
        Path::new("templates").join(template),
        Path::new("..").join("templates").join(template),
    ];

    let template_dir = possible_template_dirs
        .iter()
        .find(|p| p.exists())
        .ok_or_else(|| {
            format!(
                "Template directory not found. Searched:\n{}",
                possible_template_dirs
                    .iter()
                    .map(|p| format!("  - {}", p.display()))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        })?;

    // Create project directory
    fs::create_dir_all(name).map_err(|e| format!("Failed to create project directory: {}", e))?;

    // Create src directory
    let src_dir = Path::new(name).join("src");
    fs::create_dir_all(&src_dir).map_err(|e| format!("Failed to create src directory: {}", e))?;

    println!("  ✓ Created directory structure");

    // Copy template files
    copy_template_files(template_dir, name, template)?;

    // Initialize git repository
    init_git_repo(name)?;

    Ok(())
}

fn copy_template_files(
    template_dir: &Path,
    project_name: &str,
    template: &str,
) -> Result<(), String> {
    let project_path = Path::new(project_name);

    // Determine main file name
    let main_file = if template == "lib" {
        "lib.wj"
    } else {
        "main.wj"
    };

    // Copy main source file
    let src_file = template_dir.join(main_file);
    if src_file.exists() {
        let dest = project_path.join("src").join(main_file);
        fs::copy(&src_file, &dest).map_err(|e| format!("Failed to copy {}: {}", main_file, e))?;
        println!("  ✓ Created src/{}", main_file);
    }

    // Copy wj.toml with project name substitution
    let toml_file = template_dir.join("wj.toml");
    if toml_file.exists() {
        let content =
            fs::read_to_string(&toml_file).map_err(|e| format!("Failed to read wj.toml: {}", e))?;
        let content = content.replace("{{PROJECT_NAME}}", project_name);

        let dest = project_path.join("wj.toml");
        fs::write(&dest, content).map_err(|e| format!("Failed to write wj.toml: {}", e))?;
        println!("  ✓ Created wj.toml");
    }

    // Copy .gitignore
    let gitignore_file = template_dir.join("gitignore");
    if gitignore_file.exists() {
        let dest = project_path.join(".gitignore");
        fs::copy(&gitignore_file, &dest)
            .map_err(|e| format!("Failed to copy .gitignore: {}", e))?;
        println!("  ✓ Created .gitignore");
    }

    // Copy README.md with project name substitution
    let readme_file = template_dir.join("README.md");
    if readme_file.exists() {
        let content = fs::read_to_string(&readme_file)
            .map_err(|e| format!("Failed to read README.md: {}", e))?;
        let content = content.replace("{{PROJECT_NAME}}", project_name);

        let dest = project_path.join("README.md");
        fs::write(&dest, content).map_err(|e| format!("Failed to write README.md: {}", e))?;
        println!("  ✓ Created README.md");
    }

    // For WASM template, copy www directory
    if template == "wasm" {
        let www_dir = template_dir.join("www");
        if www_dir.exists() {
            let dest_www = project_path.join("www");
            fs::create_dir_all(&dest_www)
                .map_err(|e| format!("Failed to create www directory: {}", e))?;

            // Copy index.html with project name substitution
            let index_file = www_dir.join("index.html");
            if index_file.exists() {
                let content = fs::read_to_string(&index_file)
                    .map_err(|e| format!("Failed to read index.html: {}", e))?;
                let content = content.replace("{{PROJECT_NAME}}", project_name);

                let dest = dest_www.join("index.html");
                fs::write(&dest, content)
                    .map_err(|e| format!("Failed to write index.html: {}", e))?;
                println!("  ✓ Created www/index.html");
            }
        }
    }

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
