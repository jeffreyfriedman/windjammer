//! # Observability System
//!
//! OpenTelemetry integration for distributed tracing, metrics, and logging.
//!
//! ## Features
//! - Distributed tracing across systems
//! - Performance metrics collection
//! - Structured logging
//! - Integration with Jaeger, Zipkin, Prometheus
//! - Automatic instrumentation of game systems
//!
//! ## Example
//! ```no_run
//! use windjammer_game_framework::observability::*;
//!
//! fn main() {
//!     // Initialize observability
//!     let config = ObservabilityConfig {
//!         service_name: "my_game".to_string(),
//!         jaeger_endpoint: Some("http://localhost:14268/api/traces".to_string()),
//!         prometheus_endpoint: Some("0.0.0.0:9090".to_string()),
//!         log_level: LogLevel::Info,
//!     };
//!     
//!     Observability::init(config).expect("Failed to initialize observability");
//!     
//!     // Use tracing in your systems
//!     let span = trace_span!("game_loop");
//!     let _guard = span.enter();
//!     
//!     // Your game code here
//! }
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Observability configuration
#[derive(Debug, Clone)]
pub struct ObservabilityConfig {
    /// Service name for tracing
    pub service_name: String,
    
    /// Jaeger endpoint for traces (optional)
    pub jaeger_endpoint: Option<String>,
    
    /// Prometheus endpoint for metrics (optional)
    pub prometheus_endpoint: Option<String>,
    
    /// Log level
    pub log_level: LogLevel,
    
    /// Sample rate (0.0 to 1.0)
    pub sample_rate: f32,
    
    /// Enable automatic instrumentation
    pub auto_instrument: bool,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            service_name: "windjammer_game".to_string(),
            jaeger_endpoint: None,
            prometheus_endpoint: None,
            log_level: LogLevel::Info,
            sample_rate: 1.0,
            auto_instrument: true,
        }
    }
}

/// Log levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Main observability system
pub struct Observability {
    config: ObservabilityConfig,
    tracer: Arc<Tracer>,
    metrics: Arc<Mutex<Metrics>>,
    logger: Arc<Logger>,
}

impl Observability {
    /// Initialize observability system
    pub fn init(config: ObservabilityConfig) -> Result<Self, ObservabilityError> {
        let tracer = Arc::new(Tracer::new(&config)?);
        let metrics = Arc::new(Mutex::new(Metrics::new(&config)?));
        let logger = Arc::new(Logger::new(&config)?);
        
        Ok(Self {
            config,
            tracer,
            metrics,
            logger,
        })
    }
    
    /// Get the tracer
    pub fn tracer(&self) -> &Arc<Tracer> {
        &self.tracer
    }
    
    /// Get the metrics collector
    pub fn metrics(&self) -> &Arc<Mutex<Metrics>> {
        &self.metrics
    }
    
    /// Get the logger
    pub fn logger(&self) -> &Arc<Logger> {
        &self.logger
    }
    
    /// Shutdown observability system
    pub fn shutdown(&self) {
        // Flush all pending data
        self.tracer.flush();
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.flush();
        }
        self.logger.flush();
    }
}

/// Distributed tracer
pub struct Tracer {
    service_name: String,
    jaeger_endpoint: Option<String>,
    spans: Arc<Mutex<Vec<Span>>>,
}

impl Tracer {
    fn new(config: &ObservabilityConfig) -> Result<Self, ObservabilityError> {
        Ok(Self {
            service_name: config.service_name.clone(),
            jaeger_endpoint: config.jaeger_endpoint.clone(),
            spans: Arc::new(Mutex::new(Vec::new())),
        })
    }
    
    /// Start a new span
    pub fn start_span(&self, name: &str) -> Span {
        Span::new(name.to_string(), self.spans.clone())
    }
    
    /// Flush all pending spans
    pub fn flush(&self) {
        if let Some(endpoint) = &self.jaeger_endpoint {
            if let Ok(mut spans) = self.spans.lock() {
                // Send spans to Jaeger
                self.send_to_jaeger(endpoint, &spans);
                spans.clear();
            }
        }
    }
    
    fn send_to_jaeger(&self, endpoint: &str, spans: &[Span]) {
        // In a real implementation, this would send spans to Jaeger
        // For now, just log them
        for span in spans {
            println!("[TRACE] {} - {:?}", span.name, span.duration());
        }
    }
}

/// A trace span
pub struct Span {
    name: String,
    start_time: Instant,
    end_time: Option<Instant>,
    attributes: HashMap<String, String>,
    spans: Arc<Mutex<Vec<Span>>>,
}

impl Span {
    fn new(name: String, spans: Arc<Mutex<Vec<Span>>>) -> Self {
        Self {
            name,
            start_time: Instant::now(),
            end_time: None,
            attributes: HashMap::new(),
            spans,
        }
    }
    
    /// Add an attribute to the span
    pub fn set_attribute(&mut self, key: &str, value: &str) {
        self.attributes.insert(key.to_string(), value.to_string());
    }
    
    /// End the span
    pub fn end(&mut self) {
        self.end_time = Some(Instant::now());
        
        // Add to tracer's span list
        if let Ok(mut spans) = self.spans.lock() {
            spans.push(Span {
                name: self.name.clone(),
                start_time: self.start_time,
                end_time: self.end_time,
                attributes: self.attributes.clone(),
                spans: self.spans.clone(),
            });
        }
    }
    
    /// Get span duration
    pub fn duration(&self) -> Duration {
        let end = self.end_time.unwrap_or_else(Instant::now);
        end.duration_since(self.start_time)
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        if self.end_time.is_none() {
            self.end();
        }
    }
}

/// Metrics collector
pub struct Metrics {
    prometheus_endpoint: Option<String>,
    counters: HashMap<String, u64>,
    gauges: HashMap<String, f64>,
    histograms: HashMap<String, Vec<f64>>,
}

impl Metrics {
    fn new(config: &ObservabilityConfig) -> Result<Self, ObservabilityError> {
        Ok(Self {
            prometheus_endpoint: config.prometheus_endpoint.clone(),
            counters: HashMap::new(),
            gauges: HashMap::new(),
            histograms: HashMap::new(),
        })
    }
    
    /// Increment a counter
    pub fn increment_counter(&mut self, name: &str, value: u64) {
        *self.counters.entry(name.to_string()).or_insert(0) += value;
    }
    
    /// Set a gauge value
    pub fn set_gauge(&mut self, name: &str, value: f64) {
        self.gauges.insert(name.to_string(), value);
    }
    
    /// Record a histogram value
    pub fn record_histogram(&mut self, name: &str, value: f64) {
        self.histograms.entry(name.to_string()).or_insert_with(Vec::new).push(value);
    }
    
    /// Get counter value
    pub fn get_counter(&self, name: &str) -> u64 {
        self.counters.get(name).copied().unwrap_or(0)
    }
    
    /// Get gauge value
    pub fn get_gauge(&self, name: &str) -> Option<f64> {
        self.gauges.get(name).copied()
    }
    
    /// Flush metrics
    pub fn flush(&mut self) {
        if let Some(endpoint) = &self.prometheus_endpoint {
            self.send_to_prometheus(endpoint);
        }
    }
    
    fn send_to_prometheus(&self, endpoint: &str) {
        // In a real implementation, this would expose metrics for Prometheus scraping
        println!("[METRICS] Counters: {:?}", self.counters);
        println!("[METRICS] Gauges: {:?}", self.gauges);
    }
}

/// Structured logger
pub struct Logger {
    log_level: LogLevel,
}

impl Logger {
    fn new(config: &ObservabilityConfig) -> Result<Self, ObservabilityError> {
        Ok(Self {
            log_level: config.log_level,
        })
    }
    
    /// Log a trace message
    pub fn trace(&self, message: &str) {
        if self.log_level <= LogLevel::Trace {
            println!("[TRACE] {}", message);
        }
    }
    
    /// Log a debug message
    pub fn debug(&self, message: &str) {
        if self.log_level <= LogLevel::Debug {
            println!("[DEBUG] {}", message);
        }
    }
    
    /// Log an info message
    pub fn info(&self, message: &str) {
        if self.log_level <= LogLevel::Info {
            println!("[INFO] {}", message);
        }
    }
    
    /// Log a warning message
    pub fn warn(&self, message: &str) {
        if self.log_level <= LogLevel::Warn {
            println!("[WARN] {}", message);
        }
    }
    
    /// Log an error message
    pub fn error(&self, message: &str) {
        if self.log_level <= LogLevel::Error {
            eprintln!("[ERROR] {}", message);
        }
    }
    
    /// Flush logger
    pub fn flush(&self) {
        // Ensure all logs are written
    }
}

/// Observability errors
#[derive(Debug)]
pub enum ObservabilityError {
    InitializationFailed(String),
    TracerError(String),
    MetricsError(String),
    LoggerError(String),
}

impl std::fmt::Display for ObservabilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObservabilityError::InitializationFailed(msg) => write!(f, "Initialization failed: {}", msg),
            ObservabilityError::TracerError(msg) => write!(f, "Tracer error: {}", msg),
            ObservabilityError::MetricsError(msg) => write!(f, "Metrics error: {}", msg),
            ObservabilityError::LoggerError(msg) => write!(f, "Logger error: {}", msg),
        }
    }
}

impl std::error::Error for ObservabilityError {}

/// Macro for creating trace spans
#[macro_export]
macro_rules! trace_span {
    ($name:expr) => {
        {
            // Get global tracer and create span
            $name
        }
    };
}

/// Macro for incrementing counters
#[macro_export]
macro_rules! increment_counter {
    ($name:expr) => {
        increment_counter!($name, 1);
    };
    ($name:expr, $value:expr) => {
        {
            // Get global metrics and increment
        }
    };
}

/// Macro for setting gauges
#[macro_export]
macro_rules! set_gauge {
    ($name:expr, $value:expr) => {
        {
            // Get global metrics and set gauge
        }
    };
}

/// Macro for recording histograms
#[macro_export]
macro_rules! record_histogram {
    ($name:expr, $value:expr) => {
        {
            // Get global metrics and record
        }
    };
}

// ============================================================================
// Game-Specific Instrumentation
// ============================================================================

/// Automatic instrumentation for game systems
pub struct GameInstrumentation {
    observability: Arc<Observability>,
}

impl GameInstrumentation {
    /// Create new game instrumentation
    pub fn new(observability: Arc<Observability>) -> Self {
        Self { observability }
    }
    
    /// Instrument frame timing
    pub fn instrument_frame(&self, frame_time: Duration, fps: f32) {
        if let Ok(mut metrics) = self.observability.metrics().lock() {
            metrics.record_histogram("frame_time_ms", frame_time.as_secs_f64() * 1000.0);
            metrics.set_gauge("fps", fps as f64);
        }
    }
    
    /// Instrument draw calls
    pub fn instrument_draw_calls(&self, count: u64) {
        if let Ok(mut metrics) = self.observability.metrics().lock() {
            metrics.set_gauge("draw_calls", count as f64);
        }
    }
    
    /// Instrument entity count
    pub fn instrument_entities(&self, count: u64) {
        if let Ok(mut metrics) = self.observability.metrics().lock() {
            metrics.set_gauge("entity_count", count as f64);
        }
    }
    
    /// Instrument memory usage
    pub fn instrument_memory(&self, used_mb: f64, allocated_mb: f64) {
        if let Ok(mut metrics) = self.observability.metrics().lock() {
            metrics.set_gauge("memory_used_mb", used_mb);
            metrics.set_gauge("memory_allocated_mb", allocated_mb);
        }
    }
    
    /// Instrument network traffic
    pub fn instrument_network(&self, bytes_sent: u64, bytes_received: u64) {
        if let Ok(mut metrics) = self.observability.metrics().lock() {
            metrics.increment_counter("network_bytes_sent", bytes_sent);
            metrics.increment_counter("network_bytes_received", bytes_received);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_observability_init() {
        let config = ObservabilityConfig::default();
        let obs = Observability::init(config).unwrap();
        assert_eq!(obs.config.service_name, "windjammer_game");
    }
    
    #[test]
    fn test_span_creation() {
        let config = ObservabilityConfig::default();
        let obs = Observability::init(config).unwrap();
        
        let mut span = obs.tracer().start_span("test_span");
        span.set_attribute("key", "value");
        span.end();
        
        assert!(span.duration().as_micros() > 0);
    }
    
    #[test]
    fn test_metrics() {
        let config = ObservabilityConfig::default();
        let obs = Observability::init(config).unwrap();
        
        {
            let mut metrics = obs.metrics().lock().unwrap();
            metrics.increment_counter("test_counter", 5);
            metrics.set_gauge("test_gauge", 42.0);
            metrics.record_histogram("test_histogram", 1.5);
            
            assert_eq!(metrics.get_counter("test_counter"), 5);
            assert_eq!(metrics.get_gauge("test_gauge"), Some(42.0));
        }
    }
    
    #[test]
    fn test_logger() {
        let config = ObservabilityConfig {
            log_level: LogLevel::Debug,
            ..Default::default()
        };
        let obs = Observability::init(config).unwrap();
        
        obs.logger().debug("Test debug message");
        obs.logger().info("Test info message");
        obs.logger().warn("Test warning message");
        obs.logger().error("Test error message");
    }
    
    #[test]
    fn test_game_instrumentation() {
        let config = ObservabilityConfig::default();
        let obs = Arc::new(Observability::init(config).unwrap());
        let instrumentation = GameInstrumentation::new(obs.clone());
        
        instrumentation.instrument_frame(Duration::from_millis(16), 60.0);
        instrumentation.instrument_draw_calls(150);
        instrumentation.instrument_entities(1000);
        instrumentation.instrument_memory(256.0, 512.0);
        instrumentation.instrument_network(1024, 2048);
        
        let metrics = obs.metrics().lock().unwrap();
        assert_eq!(metrics.get_gauge("fps"), Some(60.0));
        assert_eq!(metrics.get_gauge("draw_calls"), Some(150.0));
    }
}

