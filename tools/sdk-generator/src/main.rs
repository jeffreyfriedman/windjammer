use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use std::fs;
use std::path::PathBuf;
use windjammer_game_framework::sdk_codegen::{CodeGenerator, Language};
use windjammer_game_framework::sdk_idl::ApiDefinition;

#[derive(Parser)]
#[command(name = "wj-sdk-gen")]
#[command(about = "Windjammer SDK Code Generator", long_about = None)]
struct Cli {
    /// Path to API definition JSON file
    #[arg(short, long, default_value = "api/windjammer_api.json")]
    api: PathBuf,

    /// Target language(s) to generate
    #[arg(short, long, value_enum)]
    languages: Vec<TargetLanguage>,

    /// Output directory
    #[arg(short, long, default_value = "sdks")]
    output: PathBuf,

    /// Generate all languages
    #[arg(long)]
    all: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum TargetLanguage {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    CSharp,
    Cpp,
    Go,
    Java,
    Kotlin,
    Lua,
    Swift,
    Ruby,
}

impl From<TargetLanguage> for Language {
    fn from(target: TargetLanguage) -> Self {
        match target {
            TargetLanguage::Rust => Language::Rust,
            TargetLanguage::Python => Language::Python,
            TargetLanguage::JavaScript => Language::JavaScript,
            TargetLanguage::TypeScript => Language::TypeScript,
            TargetLanguage::CSharp => Language::CSharp,
            TargetLanguage::Cpp => Language::Cpp,
            TargetLanguage::Go => Language::Go,
            TargetLanguage::Java => Language::Java,
            TargetLanguage::Kotlin => Language::Java, // Kotlin uses Java codegen for now
            TargetLanguage::Lua => Language::Lua,
            TargetLanguage::Swift => Language::Swift,
            TargetLanguage::Ruby => Language::Ruby,
        }
    }
}

impl TargetLanguage {
    fn directory_name(&self) -> &str {
        match self {
            TargetLanguage::Rust => "rust",
            TargetLanguage::Python => "python",
            TargetLanguage::JavaScript => "javascript",
            TargetLanguage::TypeScript => "typescript",
            TargetLanguage::CSharp => "csharp",
            TargetLanguage::Cpp => "cpp",
            TargetLanguage::Go => "go",
            TargetLanguage::Java => "java",
            TargetLanguage::Kotlin => "kotlin",
            TargetLanguage::Lua => "lua",
            TargetLanguage::Swift => "swift",
            TargetLanguage::Ruby => "ruby",
        }
    }

    fn all() -> Vec<Self> {
        vec![
            TargetLanguage::Rust,
            TargetLanguage::Python,
            TargetLanguage::JavaScript,
            TargetLanguage::TypeScript,
            TargetLanguage::CSharp,
            TargetLanguage::Cpp,
            TargetLanguage::Go,
            TargetLanguage::Java,
            TargetLanguage::Kotlin,
            TargetLanguage::Lua,
            TargetLanguage::Swift,
            TargetLanguage::Ruby,
        ]
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load API definition
    println!("Loading API definition from {:?}", cli.api);
    let api_json = fs::read_to_string(&cli.api)
        .with_context(|| format!("Failed to read API definition from {:?}", cli.api))?;
    
    let api: ApiDefinition = serde_json::from_str(&api_json)
        .context("Failed to parse API definition JSON")?;

    println!("Loaded API: {} v{}", api.name, api.version);
    println!("  - {} structs", api.structs.len());
    println!("  - {} classes", api.classes.len());
    println!("  - {} enums", api.enums.len());
    println!("  - {} functions", api.functions.len());

    // Determine which languages to generate
    let languages = if cli.all {
        TargetLanguage::all()
    } else if cli.languages.is_empty() {
        eprintln!("Error: No languages specified. Use --languages or --all");
        std::process::exit(1);
    } else {
        cli.languages
    };

    // Generate SDKs for each language
    for target_lang in languages {
        println!("\nGenerating SDK for {:?}...", target_lang);
        
        let lang: Language = target_lang.into();
        let generator = CodeGenerator::new(lang);
        
        let generated = generator.generate(&api)
            .with_context(|| format!("Failed to generate code for {:?}", target_lang))?;

        // Create output directory
        let output_dir = cli.output.join(target_lang.directory_name()).join("generated");
        fs::create_dir_all(&output_dir)
            .with_context(|| format!("Failed to create output directory {:?}", output_dir))?;

        // Write generated files
        for (filename, content) in &generated.files {
            let file_path = output_dir.join(filename);
            fs::write(&file_path, content)
                .with_context(|| format!("Failed to write file {:?}", file_path))?;
            println!("  ✓ Generated {}", filename);
        }

        println!("  ✓ SDK for {:?} generated successfully", target_lang);
    }

    println!("\n✅ All SDKs generated successfully!");
    Ok(())
}

