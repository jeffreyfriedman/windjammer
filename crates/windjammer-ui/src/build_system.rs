// Build System Module
// Manages compilation, running, and stopping of Windjammer projects

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use std::path::PathBuf;
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use std::process::{Child, Command, Stdio};
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use std::sync::{Arc, Mutex};
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use std::time::{Duration, Instant};

// ============================================================================
// DATA MODEL (Pure Rust - easily portable)
// ============================================================================

#[derive(Clone, Debug, PartialEq)]
pub enum BuildStatus {
    Idle,
    Compiling,
    Running,
    Stopping,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum BuildTarget {
    Native,
    Wasm,
    Release,
}

#[derive(Clone, Debug)]
pub struct BuildConfig {
    pub target: BuildTarget,
    pub optimization_level: u8, // 0-3
    pub enable_debug_symbols: bool,
    pub enable_hot_reload: bool,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            target: BuildTarget::Native,
            optimization_level: 0,
            enable_debug_symbols: true,
            enable_hot_reload: false,
        }
    }
}

// ============================================================================
// BUILD SYSTEM (Platform-specific implementation)
// ============================================================================

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct BuildSystem {
    status: Arc<Mutex<BuildStatus>>,
    config: BuildConfig,
    project_path: Option<PathBuf>,
    running_process: Option<Child>,
    build_output: Arc<Mutex<Vec<String>>>,
    last_build_time: Option<Instant>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl BuildSystem {
    pub fn new() -> Self {
        Self {
            status: Arc::new(Mutex::new(BuildStatus::Idle)),
            config: BuildConfig::default(),
            project_path: None,
            running_process: None,
            build_output: Arc::new(Mutex::new(Vec::new())),
            last_build_time: None,
        }
    }

    pub fn set_project_path(&mut self, path: PathBuf) {
        self.project_path = Some(path);
    }

    pub fn set_config(&mut self, config: BuildConfig) {
        self.config = config;
    }

    pub fn get_config(&self) -> &BuildConfig {
        &self.config
    }

    pub fn get_status(&self) -> BuildStatus {
        self.status.lock().unwrap().clone()
    }

    pub fn get_output(&self) -> Vec<String> {
        self.build_output.lock().unwrap().clone()
    }

    pub fn clear_output(&mut self) {
        self.build_output.lock().unwrap().clear();
    }

    pub fn compile(&mut self) -> Result<(), String> {
        let project_path = self.project_path.as_ref()
            .ok_or_else(|| "No project path set".to_string())?
            .clone();

        // Update status
        *self.status.lock().unwrap() = BuildStatus::Compiling;
        self.clear_output();

        // Build command based on target
        let mut cmd = Command::new("wj");
        cmd.arg("build");
        cmd.arg(&project_path);

        match self.config.target {
            BuildTarget::Native => {
                cmd.arg("--target").arg("rust");
            }
            BuildTarget::Wasm => {
                cmd.arg("--target").arg("wasm");
            }
            BuildTarget::Release => {
                cmd.arg("--target").arg("rust");
                cmd.arg("--release");
            }
        }

        // Execute build
        self.add_output(format!("🔨 Building project: {}", project_path.display()));
        self.add_output(format!("Target: {:?}", self.config.target));

        let start_time = Instant::now();

        match cmd.output() {
            Ok(output) => {
                let duration = start_time.elapsed();
                self.last_build_time = Some(start_time);

                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                for line in stdout.lines() {
                    self.add_output(line.to_string());
                }

                for line in stderr.lines() {
                    self.add_output(format!("⚠️ {}", line));
                }

                if output.status.success() {
                    *self.status.lock().unwrap() = BuildStatus::Idle;
                    self.add_output(format!("✅ Build successful! ({:.2}s)", duration.as_secs_f32()));
                    Ok(())
                } else {
                    let error = format!("Build failed with exit code: {:?}", output.status.code());
                    *self.status.lock().unwrap() = BuildStatus::Failed(error.clone());
                    self.add_output(format!("❌ {}", error));
                    Err(error)
                }
            }
            Err(e) => {
                let error = format!("Failed to execute build command: {}", e);
                *self.status.lock().unwrap() = BuildStatus::Failed(error.clone());
                self.add_output(format!("❌ {}", error));
                Err(error)
            }
        }
    }

    pub fn run(&mut self) -> Result<(), String> {
        // First, ensure we're compiled
        if matches!(self.get_status(), BuildStatus::Idle) {
            self.compile()?;
        }

        let project_path = self.project_path.as_ref()
            .ok_or_else(|| "No project path set".to_string())?
            .clone();

        // Update status
        *self.status.lock().unwrap() = BuildStatus::Running;

        // Run command
        let mut cmd = Command::new("wj");
        cmd.arg("run");
        cmd.arg(&project_path);

        match self.config.target {
            BuildTarget::Native | BuildTarget::Release => {
                cmd.arg("--target").arg("rust");
            }
            BuildTarget::Wasm => {
                cmd.arg("--target").arg("wasm");
            }
        }

        // Spawn process (non-blocking)
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        self.add_output(format!("▶️ Running project: {}", project_path.display()));

        match cmd.spawn() {
            Ok(child) => {
                self.running_process = Some(child);
                self.add_output("✅ Process started successfully".to_string());
                Ok(())
            }
            Err(e) => {
                let error = format!("Failed to start process: {}", e);
                *self.status.lock().unwrap() = BuildStatus::Failed(error.clone());
                self.add_output(format!("❌ {}", error));
                Err(error)
            }
        }
    }

    pub fn stop(&mut self) -> Result<(), String> {
        if let Some(mut child) = self.running_process.take() {
            *self.status.lock().unwrap() = BuildStatus::Stopping;
            self.add_output("⏹️ Stopping process...".to_string());

            match child.kill() {
                Ok(_) => {
                    *self.status.lock().unwrap() = BuildStatus::Idle;
                    self.add_output("✅ Process stopped".to_string());
                    Ok(())
                }
                Err(e) => {
                    let error = format!("Failed to stop process: {}", e);
                    *self.status.lock().unwrap() = BuildStatus::Failed(error.clone());
                    self.add_output(format!("❌ {}", error));
                    Err(error)
                }
            }
        } else {
            Err("No running process".to_string())
        }
    }

    pub fn clean(&mut self) -> Result<(), String> {
        let project_path = self.project_path.as_ref()
            .ok_or_else(|| "No project path set".to_string())?
            .clone();

        self.add_output(format!("🧹 Cleaning project: {}", project_path.display()));

        let mut cmd = Command::new("wj");
        cmd.arg("clean");
        cmd.arg(&project_path);

        match cmd.output() {
            Ok(output) => {
                if output.status.success() {
                    self.add_output("✅ Clean successful".to_string());
                    Ok(())
                } else {
                    let error = "Clean failed".to_string();
                    self.add_output(format!("❌ {}", error));
                    Err(error)
                }
            }
            Err(e) => {
                let error = format!("Failed to execute clean command: {}", e);
                self.add_output(format!("❌ {}", error));
                Err(error)
            }
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self.get_status(), BuildStatus::Running)
    }

    pub fn is_compiling(&self) -> bool {
        matches!(self.get_status(), BuildStatus::Compiling)
    }

    pub fn get_last_build_time(&self) -> Option<Duration> {
        self.last_build_time.map(|t| t.elapsed())
    }

    fn add_output(&self, line: String) {
        self.build_output.lock().unwrap().push(line);
    }

    // Check if running process has exited
    pub fn update(&mut self) {
        if let Some(child) = &mut self.running_process {
            if let Ok(Some(status)) = child.try_wait() {
                // Process has exited
                self.running_process = None;
                *self.status.lock().unwrap() = BuildStatus::Idle;
                
                if status.success() {
                    self.add_output("✅ Process exited successfully".to_string());
                } else {
                    self.add_output(format!("⚠️ Process exited with code: {:?}", status.code()));
                }
            }
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for BuildSystem {
    fn default() -> Self {
        Self::new()
    }
}

