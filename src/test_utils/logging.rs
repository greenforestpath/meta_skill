use std::time::Instant;

pub struct TestLogger {
    test_name: String,
    start_time: Instant,
}

impl TestLogger {
    pub fn new(test_name: &str) -> Self {
        let separator = "=".repeat(60);
        println!("\n{}", separator);
        println!("[TEST START] {}", test_name);
        println!("{}", separator);
        Self {
            test_name: test_name.to_string(),
            start_time: Instant::now(),
        }
    }

    pub fn log_input<T: std::fmt::Debug>(&self, name: &str, value: &T) {
        println!("[INPUT] {}: {:?}", name, value);
    }

    pub fn log_expected<T: std::fmt::Debug>(&self, value: &T) {
        println!("[EXPECTED] {:?}", value);
    }

    pub fn log_actual<T: std::fmt::Debug>(&self, value: &T) {
        println!("[ACTUAL] {:?}", value);
    }

    pub fn pass(&self) {
        let elapsed = self.start_time.elapsed();
        println!("[RESULT] PASSED in {:?}", elapsed);
        println!("{}\n", "=".repeat(60));
    }

    pub fn fail(&self, reason: &str) {
        let elapsed = self.start_time.elapsed();
        println!("[RESULT] FAILED in {:?}", elapsed);
        println!("[REASON] {}", reason);
        println!("{}\n", "=".repeat(60));
    }

    #[allow(dead_code)]
    pub fn test_name(&self) -> &str {
        &self.test_name
    }
}
