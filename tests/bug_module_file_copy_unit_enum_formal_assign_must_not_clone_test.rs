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

//! Regression: module file copy unit enum formal assign must not clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

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

pub fn weather_to_atmosphere_bundle(weather: WeatherType, intensity: WeatherIntensity) -> i32 {
    0
}

pub struct WeatherSystem {
    pub current_weather: WeatherType,
    pub intensity: WeatherIntensity,
    pub atmosphere: i32,
}

impl WeatherSystem {
    pub fn new() -> WeatherSystem {
        let weather = WeatherType::Clear
        let intensity = WeatherIntensity::Light
        WeatherSystem {
            current_weather: weather,
            intensity: intensity,
            atmosphere: weather_to_atmosphere_bundle(weather, intensity),
        }
    }

    pub fn set_weather(self, weather: WeatherType, intensity: WeatherIntensity) {
        self.current_weather = weather
        self.intensity = intensity
        self.atmosphere = weather_to_atmosphere_bundle(weather, intensity)
        // weather reused after the call (comparisons) — still must not clone.
        if weather == WeatherType::Fog {
            let _ = intensity
        }
    }
}
"#;

#[test]
fn module_file_copy_unit_enum_formal_assign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("weather.clone()") || rs.contains("intensity.clone()");
    assert!(
        !bad,
        "RED: Copy unit-enum formal/local cloned into assign/struct lit:\n{rs}"
    );
    let _ = test.cargo_check();
}
