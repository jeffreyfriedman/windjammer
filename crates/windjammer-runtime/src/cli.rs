//! Command-line argument parsing with builder pattern
//!
//! Windjammer's `std::cli` module maps to these functions.
//! Provides a fluent builder API similar to clap.

use clap::{Arg, ArgAction, ArgMatches, Command};

/// Create a new CLI application builder
pub fn new(name: &str) -> AppBuilder {
    AppBuilder {
        name: name.to_string(),
        version: None,
        author: None,
        about: None,
        args: Vec::new(),
    }
}

/// Create a new argument (positional or required)
pub fn arg(name: &str) -> ArgBuilder {
    ArgBuilder {
        name: name.to_string(),
        short: None,
        long: None,
        help: None,
        required: false,
        multiple: false,
        default_value: None,
        arg_type: ArgType::Positional,
    }
}

/// Create a new flag (boolean, no value)
pub fn flag(name: &str) -> ArgBuilder {
    ArgBuilder {
        name: name.to_string(),
        short: None,
        long: None,
        help: None,
        required: false,
        multiple: false,
        default_value: None,
        arg_type: ArgType::Flag,
    }
}

/// Create a new option (takes a value)
pub fn option(name: &str) -> ArgBuilder {
    ArgBuilder {
        name: name.to_string(),
        short: None,
        long: None,
        help: None,
        required: false,
        multiple: false,
        default_value: None,
        arg_type: ArgType::Option,
    }
}

#[derive(Debug, Clone)]
enum ArgType {
    Positional,
    Flag,
    Option,
}

/// CLI Application builder
#[derive(Debug, Clone)]
pub struct AppBuilder {
    name: String,
    version: Option<String>,
    author: Option<String>,
    about: Option<String>,
    args: Vec<ArgBuilder>,
}

/// CLI Argument builder
#[derive(Debug, Clone)]
pub struct ArgBuilder {
    name: String,
    short: Option<String>,
    long: Option<String>,
    help: Option<String>,
    required: bool,
    multiple: bool,
    default_value: Option<String>,
    arg_type: ArgType,
}

impl AppBuilder {
    /// Set the version
    pub fn version(mut self, version: &str) -> Self {
        self.version = Some(version.to_string());
        self
    }

    /// Set the author
    pub fn author(mut self, author: &str) -> Self {
        self.author = Some(author.to_string());
        self
    }

    /// Set the about text
    pub fn about(mut self, about: &str) -> Self {
        self.about = Some(about.to_string());
        self
    }

    /// Add an argument
    pub fn arg(mut self, arg: ArgBuilder) -> Self {
        self.args.push(arg);
        self
    }

    /// Parse command-line arguments
    pub fn parse(self) -> CliMatches {
        // Use Box::leak to get 'static strings (acceptable for CLI parsing which happens once)
        let name: &'static str = Box::leak(self.name.into_boxed_str());
        let mut cmd = Command::new(name);

        if let Some(version) = self.version {
            let version_static: &'static str = Box::leak(version.into_boxed_str());
            cmd = cmd.version(version_static);
        }
        if let Some(author) = self.author {
            let author_static: &'static str = Box::leak(author.into_boxed_str());
            cmd = cmd.author(author_static);
        }
        if let Some(about) = self.about {
            let about_static: &'static str = Box::leak(about.into_boxed_str());
            cmd = cmd.about(about_static);
        }

        for arg_builder in self.args {
            // Use Box::leak to get 'static strings (acceptable for CLI parsing which happens once)
            let name: &'static str = Box::leak(arg_builder.name.into_boxed_str());
            
            let mut arg = Arg::new(name);

            if let Some(help_str) = arg_builder.help {
                let help: &'static str = Box::leak(help_str.into_boxed_str());
                arg = arg.help(help);
            }

            if let Some(short_str) = arg_builder.short {
                if let Some(c) = short_str.chars().next() {
                    arg = arg.short(c);
                }
            }

            if let Some(long_str) = arg_builder.long {
                let long: &'static str = Box::leak(long_str.into_boxed_str());
                arg = arg.long(long);
            }

            match arg_builder.arg_type {
                ArgType::Positional => {
                    arg = arg.required(arg_builder.required);
                    if arg_builder.multiple {
                        arg = arg.num_args(1..);
                    }
                    if let Some(default_str) = arg_builder.default_value {
                        let default: &'static str = Box::leak(default_str.into_boxed_str());
                        arg = arg.default_value(default);
                    }
                }
                ArgType::Flag => {
                    arg = arg.action(ArgAction::SetTrue);
                }
                ArgType::Option => {
                    arg = arg.required(arg_builder.required);
                    if arg_builder.multiple {
                        arg = arg.num_args(0..);
                        arg = arg.action(ArgAction::Append);
                    } else {
                        arg = arg.num_args(0..=1);
                    }
                    if let Some(default_str) = arg_builder.default_value {
                        let default: &'static str = Box::leak(default_str.into_boxed_str());
                        arg = arg.default_value(default);
                    }
                }
            }

            cmd = cmd.arg(arg);
        }

        let matches = cmd.get_matches();
        CliMatches { matches }
    }

    /// Get matches without consuming (for testing)
    pub fn get_matches(self) -> CliMatches {
        self.parse()
    }
}

impl ArgBuilder {
    /// Set the help text
    pub fn help(mut self, help: &str) -> Self {
        self.help = Some(help.to_string());
        self
    }

    /// Set the short flag (single character)
    pub fn short(mut self, short: &str) -> Self {
        self.short = Some(short.to_string());
        self
    }

    /// Set the long flag
    pub fn long(mut self, long: &str) -> Self {
        self.long = Some(long.to_string());
        self
    }

    /// Mark as required
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Allow multiple values
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// Set default value
    pub fn default_value(mut self, value: &str) -> Self {
        self.default_value = Some(value.to_string());
        self
    }
}

/// Parsed CLI matches
pub struct CliMatches {
    matches: ArgMatches,
}

impl CliMatches {
    /// Get string value
    pub fn get(&self, name: &str) -> Option<String> {
        self.matches.get_one::<String>(name).cloned()
    }

    /// Get string value (returns Option for unwrap/unwrap_or chaining)
    pub fn value_of(&self, name: &str) -> Option<String> {
        self.get(name)
    }

    /// Check if flag is present
    pub fn is_present(&self, name: &str) -> bool {
        self.matches.get_flag(name)
    }

    /// Get all values for an argument
    pub fn get_many(&self, name: &str) -> Vec<String> {
        self.matches
            .get_many::<String>(name)
            .map(|vals| vals.map(|s| s.to_string()).collect())
            .unwrap_or_default()
    }

    /// Get all values for an argument (returns Option for unwrap/unwrap_or chaining)
    pub fn values_of(&self, name: &str) -> Option<Vec<String>> {
        let vals = self.matches
            .get_many::<String>(name)
            .map(|vals| vals.map(|s| s.to_string()).collect::<Vec<String>>());
        vals
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_builder() {
        let app = new("test")
            .version("1.0")
            .author("Test Author")
            .about("Test app")
            .arg(arg("pattern").help("Pattern to search").required(true))
            .arg(flag("verbose").short("v").help("Verbose output"))
            .arg(
                option("threads")
                    .short("j")
                    .help("Number of threads")
                    .default_value("4"),
            );

        assert_eq!(app.name, "test");
        assert_eq!(app.version, Some("1.0".to_string()));
        assert_eq!(app.args.len(), 3);
    }

    #[test]
    fn test_arg_builder() {
        let arg = arg("pattern").help("Pattern").required(true);
        assert_eq!(arg.name, "pattern");
        assert!(arg.required);
    }

    #[test]
    fn test_flag_builder() {
        let flag = flag("verbose").short("v").help("Verbose");
        assert_eq!(flag.name, "verbose");
        matches!(flag.arg_type, ArgType::Flag);
    }

    #[test]
    fn test_option_builder() {
        let opt = option("threads").short("j").default_value("4");
        assert_eq!(opt.name, "threads");
        matches!(opt.arg_type, ArgType::Option);
    }
}
