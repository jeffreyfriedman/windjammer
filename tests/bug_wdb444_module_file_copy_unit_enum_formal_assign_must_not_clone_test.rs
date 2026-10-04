#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
    feature = "codegen_tests",
))]

//! WDB-444: Copy unit-enum formal into field assign / struct lit must not `.clone()`.
//!
//! Product `weather/weather_system.rs`:
//!   `self.current_weather = weather.clone()`
//!   `self.intensity = intensity.clone()`
//! WJ uses bare `weather` / `intensity`.
//! Distinct from WDB-384/392 (`Direction::Variant.clone()` path exprs),
//! WDB-440 (f32 into struct lit), and WDB-437 (i32 formal into let).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum WeatherType {
    Clear,
    Fog,
}

pub enum WeatherIntensity {
    Light,
    Medium,
    Heavy,
}

pub struct WeatherSystem {
    pub current_weather: WeatherType,
    pub intensity: WeatherIntensity,
}

impl WeatherSystem {
    pub fn new() -> WeatherSystem {
        let weather = WeatherType::Clear
        let intensity = WeatherIntensity::Light
        WeatherSystem {
            current_weather: weather,
            intensity: intensity,
        }
    }

    pub fn set_weather(self, weather: WeatherType, intensity: WeatherIntensity) {
        self.current_weather = weather
        self.intensity = intensity
    }
}
"#;

#[test]
fn wdb444_module_file_copy_unit_enum_formal_assign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-444 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-444 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("weather.clone()") || rs.contains("intensity.clone()");
    assert!(
        !bad,
        "WDB-444 RED: Copy unit-enum formal/local cloned into assign/struct lit:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb444_search_roots() -> Vec<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut roots = vec![manifest.clone()];
    let git = manifest.join(".git");
    if git.is_file() {
        if let Ok(text) = std::fs::read_to_string(&git) {
            if let Some(line) = text.lines().find(|l| l.starts_with("gitdir:")) {
                let gitdir = PathBuf::from(line.trim_start_matches("gitdir:").trim());
                if let Some(repo) = gitdir.ancestors().nth(3) {
                    roots.push(repo.to_path_buf());
                    if let Some(src_wj) = repo.parent() {
                        roots.push(src_wj.to_path_buf());
                    }
                }
            }
        }
    }
    let mut walked = manifest;
    for _ in 0..8 {
        roots.push(walked.clone());
        if let Some(parent) = walked.parent() {
            walked = parent.to_path_buf();
        } else {
            break;
        }
    }
    roots
}

#[test]
fn wdb444_tip_out_game_core_weather_system_enum_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb444_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/weather/weather_system.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/weather/weather_system.rs"));
    }
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("product");
        let bad = text.lines().any(|line| {
            let t = line.trim_start();
            !t.starts_with("//")
                && (t.contains("weather.clone()") || t.contains("intensity.clone()"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-444: weather_system product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-444 RED: tip/product Copy unit-enum formal clone:\n  {}",
        bad_paths.join("\n  ")
    );
}
