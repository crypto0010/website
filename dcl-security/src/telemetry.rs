// dcl-security/src/telemetry.rs
//! Telemetry and monitoring infrastructure for production DCL systems.
//! Provides structured logging, metrics collection, and performance tracking.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn, Level};

/// Global telemetry collector instance
static TELEMETRY: once_cell::sync::Lazy<Arc<Mutex<TelemetryCollector>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(TelemetryCollector::new())));

/// Initialize telemetry system with configuration
pub fn init_telemetry(config: TelemetryConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Configure tracing subscriber
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            tracing_subscriber::EnvFilter::new(config.log_level.as_str())
        });

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(config.max_level())
        .with_target(config.include_target)
        .with_thread_ids(config.include_thread_ids)
        .with_line_number(config.include_line_numbers)
        .with_env_filter(filter);

    if config.json_output {
        subscriber.json().init();
    } else {
        subscriber.init();
    }

    info!("Telemetry initialized with config: {:?}", config);
    Ok(())
}

/// Telemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    pub log_level: String,
    pub json_output: bool,
    pub include_target: bool,
    pub include_thread_ids: bool,
    pub include_line_numbers: bool,
    pub metrics_enabled: bool,
    pub performance_tracking: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        TelemetryConfig {
            log_level: "info".to_string(),
            json_output: false,
            include_target: true,
            include_thread_ids: false,
            include_line_numbers: true,
            metrics_enabled: true,
            performance_tracking: true,
        }
    }
}

impl TelemetryConfig {
    pub fn max_level(&self) -> Level {
        match self.log_level.to_lowercase().as_str() {
            "trace" => Level::TRACE,
            "debug" => Level::DEBUG,
            "info" => Level::INFO,
            "warn" => Level::WARN,
            "error" => Level::ERROR,
            _ => Level::INFO,
        }
    }

    pub fn production() -> Self {
        TelemetryConfig {
            log_level: "warn".to_string(),
            json_output: true,
            include_target: true,
            include_thread_ids: true,
            include_line_numbers: false,
            metrics_enabled: true,
            performance_tracking: true,
        }
    }

    pub fn development() -> Self {
        TelemetryConfig {
            log_level: "debug".to_string(),
            json_output: false,
            include_target: true,
            include_thread_ids: false,
            include_line_numbers: true,
            metrics_enabled: true,
            performance_tracking: true,
        }
    }
}

/// Telemetry collector for metrics and events
#[derive(Debug)]
pub struct TelemetryCollector {
    metrics: HashMap<String, MetricValue>,
    events: Vec<TelemetryEvent>,
    performance: HashMap<String, PerformanceStats>,
    start_time: Instant,
}

impl TelemetryCollector {
    pub fn new() -> Self {
        TelemetryCollector {
            metrics: HashMap::new(),
            events: Vec::new(),
            performance: HashMap::new(),
            start_time: Instant::now(),
        }
    }

    pub fn record_counter(&mut self, name: &str, value: u64) {
        self.metrics
            .entry(name.to_string())
            .and_modify(|v| {
                if let MetricValue::Counter(ref mut c) = v {
                    *c += value;
                }
            })
            .or_insert(MetricValue::Counter(value));
    }

    pub fn record_gauge(&mut self, name: &str, value: f64) {
        self.metrics
            .insert(name.to_string(), MetricValue::Gauge(value));
    }

    pub fn record_histogram(&mut self, name: &str, value: f64) {
        self.metrics
            .entry(name.to_string())
            .and_modify(|v| {
                if let MetricValue::Histogram(ref mut values) = v {
                    values.push(value);
                }
            })
            .or_insert(MetricValue::Histogram(vec![value]));
    }

    pub fn record_event(&mut self, event: TelemetryEvent) {
        debug!("Telemetry event: {:?}", event);
        self.events.push(event);
    }

    pub fn record_performance(&mut self, operation: &str, duration: Duration) {
        self.performance
            .entry(operation.to_string())
            .and_modify(|stats| {
                stats.count += 1;
                stats.total_duration += duration;
                stats.min_duration = stats.min_duration.min(duration);
                stats.max_duration = stats.max_duration.max(duration);
            })
            .or_insert(PerformanceStats {
                operation: operation.to_string(),
                count: 1,
                total_duration: duration,
                min_duration: duration,
                max_duration: duration,
            });
    }

    pub fn get_metric(&self, name: &str) -> Option<&MetricValue> {
        self.metrics.get(name)
    }

    pub fn get_performance(&self, operation: &str) -> Option<&PerformanceStats> {
        self.performance.get(operation)
    }

    pub fn snapshot(&self) -> TelemetrySnapshot {
        TelemetrySnapshot {
            uptime: self.start_time.elapsed(),
            metrics: self.metrics.clone(),
            performance: self.performance.values().cloned().collect(),
            event_count: self.events.len(),
        }
    }

    pub fn reset(&mut self) {
        self.metrics.clear();
        self.events.clear();
        self.performance.clear();
    }
}

/// Metric value types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram(Vec<f64>),
}

/// Telemetry event for significant operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEvent {
    pub timestamp: u64,
    pub level: EventLevel,
    pub category: String,
    pub message: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EventLevel {
    Info,
    Warning,
    Error,
    Critical,
}

impl TelemetryEvent {
    pub fn new(level: EventLevel, category: &str, message: &str) -> Self {
        TelemetryEvent {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            level,
            category: category.to_string(),
            message: message.to_string(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

/// Performance statistics for operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    pub operation: String,
    pub count: u64,
    pub total_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
}

impl PerformanceStats {
    pub fn average_duration(&self) -> Duration {
        if self.count == 0 {
            Duration::ZERO
        } else {
            self.total_duration / self.count as u32
        }
    }

    pub fn operations_per_second(&self, elapsed: Duration) -> f64 {
        if elapsed.as_secs_f64() == 0.0 {
            0.0
        } else {
            self.count as f64 / elapsed.as_secs_f64()
        }
    }
}

/// Snapshot of current telemetry state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    pub uptime: Duration,
    pub metrics: HashMap<String, MetricValue>,
    pub performance: Vec<PerformanceStats>,
    pub event_count: usize,
}

impl TelemetrySnapshot {
    pub fn print_summary(&self) {
        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║              TELEMETRY SNAPSHOT                          ║");
        println!("╚══════════════════════════════════════════════════════════╝\n");

        println!("Uptime: {:.2?}", self.uptime);
        println!("Events Recorded: {}", self.event_count);
        println!();

        if !self.metrics.is_empty() {
            println!("Metrics:");
            for (name, value) in &self.metrics {
                match value {
                    MetricValue::Counter(c) => println!("  {} = {} (counter)", name, c),
                    MetricValue::Gauge(g) => println!("  {} = {:.2} (gauge)", name, g),
                    MetricValue::Histogram(h) => {
                        let avg = h.iter().sum::<f64>() / h.len() as f64;
                        println!("  {} = {:.2} avg ({} samples)", name, avg, h.len());
                    }
                }
            }
            println!();
        }

        if !self.performance.is_empty() {
            println!("Performance:");
            for stats in &self.performance {
                println!("  {}:", stats.operation);
                println!("    Count: {}", stats.count);
                println!("    Average: {:.2?}", stats.average_duration());
                println!("    Min: {:.2?}", stats.min_duration);
                println!("    Max: {:.2?}", stats.max_duration);
                println!("    Ops/sec: {:.2}", stats.operations_per_second(self.uptime));
            }
            println!();
        }
    }
}

/// Performance timer for tracking operation duration
pub struct PerfTimer {
    operation: String,
    start: Instant,
}

impl PerfTimer {
    pub fn start(operation: &str) -> Self {
        debug!("Starting operation: {}", operation);
        PerfTimer {
            operation: operation.to_string(),
            start: Instant::now(),
        }
    }

    pub fn stop(self) {
        let duration = self.start.elapsed();
        debug!("Operation {} completed in {:.2?}", self.operation, duration);

        if let Ok(mut collector) = TELEMETRY.lock() {
            collector.record_performance(&self.operation, duration);
        }
    }
}

/// Global telemetry functions
pub fn record_counter(name: &str, value: u64) {
    if let Ok(mut collector) = TELEMETRY.lock() {
        collector.record_counter(name, value);
    }
}

pub fn record_gauge(name: &str, value: f64) {
    if let Ok(mut collector) = TELEMETRY.lock() {
        collector.record_gauge(name, value);
    }
}

pub fn record_histogram(name: &str, value: f64) {
    if let Ok(mut collector) = TELEMETRY.lock() {
        collector.record_histogram(name, value);
    }
}

pub fn record_event(event: TelemetryEvent) {
    if let Ok(mut collector) = TELEMETRY.lock() {
        collector.record_event(event);
    }
}

pub fn get_snapshot() -> TelemetrySnapshot {
    TELEMETRY
        .lock()
        .map(|c| c.snapshot())
        .unwrap_or_else(|_| TelemetrySnapshot {
            uptime: Duration::ZERO,
            metrics: HashMap::new(),
            performance: Vec::new(),
            event_count: 0,
        })
}

pub fn reset_telemetry() {
    if let Ok(mut collector) = TELEMETRY.lock() {
        collector.reset();
        info!("Telemetry data reset");
    }
}

/// Instrumented operations for DCL framework
pub mod instrumented {
    use super::*;
    use tracing::instrument;

    #[instrument(skip(f))]
    pub fn prime_generation<F>(count: usize, f: F) -> Vec<u64>
    where
        F: Fn() -> Option<u64>,
    {
        let timer = PerfTimer::start("prime_generation");
        let mut primes = Vec::with_capacity(count);

        for _ in 0..count {
            if let Some(prime) = f() {
                primes.push(prime);
            }
        }

        record_counter("primes_generated", primes.len() as u64);
        record_gauge("prime_generation_rate", primes.len() as f64 / count as f64);

        timer.stop();
        primes
    }

    #[instrument]
    pub fn gcd_computation(a: u64, b: u64, result: u64) {
        record_counter("gcd_computations", 1);
        record_histogram("gcd_input_size", (a.max(b) as f64).log2());

        if result == 1 {
            record_counter("coprime_pairs", 1);
        }
    }

    #[instrument]
    pub fn nist_test_execution(test_name: &str, passed: bool, p_value: f64) {
        record_counter(&format!("nist_test_{}", test_name), 1);

        if passed {
            record_counter(&format!("nist_test_{}_passed", test_name), 1);
        }

        record_histogram(&format!("nist_test_{}_p_value", test_name), p_value);
    }

    #[instrument]
    pub fn side_channel_operation(operation: &str, constant_time: bool) {
        record_counter(&format!("side_channel_{}", operation), 1);

        if constant_time {
            record_counter(&format!("side_channel_{}_constant_time", operation), 1);
        } else {
            warn!("Non-constant-time operation detected: {}", operation);
            record_event(TelemetryEvent::new(
                EventLevel::Warning,
                "side_channel",
                &format!("Non-constant-time: {}", operation),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_config() {
        let config = TelemetryConfig::default();
        assert_eq!(config.log_level, "info");
        assert_eq!(config.max_level(), Level::INFO);

        let prod_config = TelemetryConfig::production();
        assert_eq!(prod_config.log_level, "warn");
        assert!(prod_config.json_output);
    }

    #[test]
    fn test_collector_metrics() {
        let mut collector = TelemetryCollector::new();

        collector.record_counter("test_counter", 5);
        collector.record_counter("test_counter", 3);

        if let Some(MetricValue::Counter(c)) = collector.get_metric("test_counter") {
            assert_eq!(*c, 8);
        } else {
            panic!("Counter not found");
        }

        collector.record_gauge("test_gauge", 42.5);
        if let Some(MetricValue::Gauge(g)) = collector.get_metric("test_gauge") {
            assert_eq!(*g, 42.5);
        } else {
            panic!("Gauge not found");
        }
    }

    #[test]
    fn test_performance_tracking() {
        let mut collector = TelemetryCollector::new();

        collector.record_performance("test_op", Duration::from_millis(100));
        collector.record_performance("test_op", Duration::from_millis(200));

        let stats = collector.get_performance("test_op").unwrap();
        assert_eq!(stats.count, 2);
        assert_eq!(stats.min_duration, Duration::from_millis(100));
        assert_eq!(stats.max_duration, Duration::from_millis(200));
        assert_eq!(stats.average_duration(), Duration::from_millis(150));
    }

    #[test]
    fn test_telemetry_event() {
        let event = TelemetryEvent::new(EventLevel::Info, "test", "Test message")
            .with_metadata("key1", "value1")
            .with_metadata("key2", "value2");

        assert_eq!(event.category, "test");
        assert_eq!(event.message, "Test message");
        assert_eq!(event.metadata.len(), 2);
    }

    #[test]
    fn test_perf_timer() {
        let timer = PerfTimer::start("test_operation");
        std::thread::sleep(Duration::from_millis(10));
        timer.stop();

        // Verify it was recorded (would need to check global collector)
    }

    #[test]
    fn test_snapshot() {
        let mut collector = TelemetryCollector::new();
        collector.record_counter("test", 42);
        collector.record_performance("op", Duration::from_millis(100));

        let snapshot = collector.snapshot();
        assert_eq!(snapshot.metrics.len(), 1);
        assert_eq!(snapshot.performance.len(), 1);
    }
}
