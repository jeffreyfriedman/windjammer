//! Command-line argument parsing
//!
//! Windjammer's `std::cli` module maps to these functions.

use clap::{Arg, ArgMatches, Command};

/// Create a new CLI application
pub fn app(name: &str, version: &str, about: &str) -> CliApp {
    CliApp {
        name: name.to_string(),
        version: version.to_string(),
        about: about.to_string(),
        args: Vec::new(),
    }
}

/// CLI Application builder
#[derive(Debug, Clone)]
pub struct CliApp {
    name: String,
    version: String,
    about: String,
    args: Vec<CliArg>,
}

/// CLI Argument definition
#[derive(Debug, Clone)]
pub struct CliArg {
    name: String,
    short: Option<char>,
    long: Option<String>,
    help: String,
    required: bool,
    takes_value: bool,
}

impl CliApp {
    /// Add an argument
    pub fn arg(
        mut self,
        name: &str,
        short: Option<char>,
        long: Option<String>,
        help: &str,
        required: bool,
    ) -> Self {
        self.args.push(CliArg {
            name: name.to_string(),
            short,
            long,
            help: help.to_string(),
            required,
            takes_value: true,
        });
        self
    }

    /// Add a flag (no value)
    pub fn flag(
        mut self,
        name: &str,
        short: Option<char>,
        long: Option<String>,
        help: &str,
    ) -> Self {
        self.args.push(CliArg {
            name: name.to_string(),
            short,
            long,
            help: help.to_string(),
            required: false,
            takes_value: false,
        });
        self
    }

    /// Parse command-line arguments
    pub fn parse(self) -> CliMatches {
        // Use Box::leak to get 'static strings (acceptable for CLI parsing which happens once)
        let name: &'static str = Box::leak(self.name.into_boxed_str());
        let version: &'static str = Box::leak(self.version.into_boxed_str());
        let about: &'static str = Box::leak(self.about.into_boxed_str());

        let mut cmd = Command::new(name).version(version).about(about);

        for arg_def in self.args {
            let arg_name: &'static str = Box::leak(arg_def.name.into_boxed_str());
            let arg_help: &'static str = Box::leak(arg_def.help.into_boxed_str());

            let mut arg = Arg::new(arg_name).help(arg_help).required(arg_def.required);

            if let Some(short) = arg_def.short {
                arg = arg.short(short);
            }
            if let Some(long) = arg_def.long {
                let long_static: &'static str = Box::leak(long.into_boxed_str());
                arg = arg.long(long_static);
            }
            if arg_def.takes_value {
                arg = arg.num_args(1);
            } else {
                arg = arg.action(clap::ArgAction::SetTrue);
            }

            cmd = cmd.arg(arg);
        }

        let matches = cmd.get_matches();
        CliMatches { matches }
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

    /// Check if flag is present
    pub fn is_present(&self, name: &str) -> bool {
        self.matches.get_flag(name)
    }

    /// Get all values for an argument
    pub fn get_many(&self, name: &str) -> Vec<String> {
        self.matches
            .get_many::<String>(name)
            .map(|vals| vals.cloned().collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_app_creation() {
        let app = app("test", "1.0", "Test app");
        assert_eq!(app.name, "test");
        assert_eq!(app.version, "1.0");
        assert_eq!(app.about, "Test app");
    }

    #[test]
    fn test_add_arg() {
        let app = app("test", "1.0", "Test app").arg(
            "input",
            Some('i'),
            Some("input".to_string()),
            "Input file",
            true,
        );
        assert_eq!(app.args.len(), 1);
        assert_eq!(app.args[0].name, "input");
    }

    #[test]
    fn test_add_flag() {
        let app = app("test", "1.0", "Test app").flag(
            "verbose",
            Some('v'),
            Some("verbose".to_string()),
            "Verbose output",
        );
        assert_eq!(app.args.len(), 1);
        assert!(!app.args[0].takes_value);
    }
}
