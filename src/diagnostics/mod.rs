//! Diagnostics module for checking system, network, and API status
//!
//! Checks the chain: [User PC] -> [Internet] -> [Claude API] -> [OpenCode]

pub mod api;
pub mod gpu;
pub mod processes;
pub mod settings;

use std::time::Instant;
use sysinfo::System;
use std::collections::VecDeque;

pub use settings::DiagnosticSettings;

/// Status of a single check
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CheckStatus {
    Ok,
    Warning,
    Error,
    #[allow(dead_code)]
    Unknown,
    Inactive, // For VPN when not detected
}

impl CheckStatus {
    pub fn label(&self) -> &'static str {
        match self {
            CheckStatus::Ok => "OK",
            CheckStatus::Warning => "WARN",
            CheckStatus::Error => "ERROR",
            CheckStatus::Unknown => "...",
            CheckStatus::Inactive => "--",
        }
    }
}

/// Result of a diagnostic check
#[derive(Clone)]
pub struct CheckResult {
    pub name: String,
    pub status: CheckStatus,
    pub details: String,
    pub message: Option<String>,
}

impl CheckResult {
    pub fn new(name: &str, status: CheckStatus, details: &str) -> Self {
        Self {
            name: name.to_string(),
            status,
            details: details.to_string(),
            message: None,
        }
    }

    pub fn with_message(mut self, msg: &str) -> Self {
        self.message = Some(msg.to_string());
        self
    }
}

/// All diagnostic results
#[derive(Clone, Default)]
pub struct DiagnosticReport {
    pub local_resources: Option<CheckResult>,
    pub gpu: Option<CheckResult>,
    pub internet: Option<CheckResult>,
    pub claude_api: Option<CheckResult>,
    pub openai_api: Option<CheckResult>,
    pub google_api: Option<CheckResult>,
    pub opencode: Option<CheckResult>,
    pub terminals: Option<CheckResult>,
    pub diagnosis: Option<String>,
    pub timestamp: Option<String>,
}

impl DiagnosticReport {
    pub fn new() -> Self {
        Self::default()
    }

    /// Run diagnostics based on settings
    pub fn run_with_settings(&mut self, settings: &DiagnosticSettings) {
        self.timestamp = Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
        
        // System checks
        if settings.check_cpu_ram {
            self.local_resources = Some(check_local_resources());
        }
        
        if settings.check_gpu {
            self.gpu = Some(gpu::check_gpu());
        }
        
        // Network
        if settings.check_internet {
            self.internet = Some(check_internet());
        }
        
        // API checks
        if settings.check_claude {
            self.claude_api = Some(api::check_claude_api());
        }
        
        if settings.check_openai {
            self.openai_api = Some(api::check_openai_api());
        }
        
        if settings.check_google_ai {
            self.google_api = Some(api::check_google_api());
        }
        
        // Process checks
        if settings.check_opencode {
            self.opencode = Some(processes::check_opencode_process());
        }
        
        if settings.check_terminals {
            self.terminals = Some(processes::check_terminals());
        }
        
        // Generate diagnosis
        self.diagnosis = Some(self.generate_diagnosis());
    }

    fn generate_diagnosis(&self) -> String {
        // Check each component and find the issue
        if let Some(ref check) = self.local_resources {
            if check.status == CheckStatus::Error {
                return "System resources critical. Close other applications.".to_string();
            }
        }

        if let Some(ref check) = self.gpu {
            if check.status == CheckStatus::Error {
                return "GPU overloaded. Close GPU-heavy applications.".to_string();
            }
            if check.status == CheckStatus::Warning {
                return "High GPU usage detected. May affect performance.".to_string();
            }
        }

        if let Some(ref check) = self.internet {
            if check.status == CheckStatus::Error {
                return "No internet connection. Check your network.".to_string();
            }
        }

        if let Some(ref check) = self.claude_api {
            match check.status {
                CheckStatus::Error => {
                    if check.details.contains("503") || check.details.contains("capacity") {
                        return "Claude API is overloaded. Try again later.".to_string();
                    } else if check.details.contains("529") {
                        return "Claude API overloaded (529). Try again in a few minutes.".to_string();
                    }
                    return format!("Claude API issue: {}", check.details);
                }
                CheckStatus::Warning => {
                    if check.details.contains("429") {
                        return "Claude API rate limited. Wait a few minutes.".to_string();
                    }
                    return "Claude API is slow. May experience delays.".to_string();
                }
                _ => {}
            }
        }

        if let Some(ref check) = self.openai_api {
            if check.status == CheckStatus::Error {
                return format!("OpenAI API issue: {}", check.details);
            }
        }

        if let Some(ref check) = self.opencode {
            if check.status == CheckStatus::Error {
                return "OpenCode process not running.".to_string();
            }
        }

        "All systems operational.".to_string()
    }

    /// Generate a text report for clipboard
    pub fn to_text_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str("=== OpenCode Diagnostics Report ===\n");
        if let Some(ref ts) = self.timestamp {
            report.push_str(&format!("Time: {}\n", ts));
        }
        report.push('\n');

        if let Some(ref check) = self.local_resources {
            report.push_str(&format_check_for_report(check));
        }
        
        if let Some(ref check) = self.gpu {
            report.push_str(&format_check_for_report(check));
        }
        
        if let Some(ref check) = self.internet {
            report.push_str(&format_check_for_report(check));
        }
        
        if let Some(ref check) = self.claude_api {
            report.push_str(&format_check_for_report(check));
        }
        
        if let Some(ref check) = self.openai_api {
            report.push_str(&format_check_for_report(check));
        }
        
        if let Some(ref check) = self.google_api {
            report.push_str(&format_check_for_report(check));
        }
        
        if let Some(ref check) = self.opencode {
            report.push_str(&format_check_for_report(check));
        }
        
        if let Some(ref check) = self.terminals {
            report.push_str(&format_check_for_report(check));
        }

        if let Some(ref diag) = self.diagnosis {
            report.push_str(&format!("\nDIAGNOSIS: {}\n", diag));
        }

        report
    }
}

/// Single error type with timestamps when it occurred
#[derive(Clone)]
pub struct ErrorEntry {
    pub name: String,           // "GPU", "CLAUDE API", etc.
    pub times: VecDeque<String>, // Up to 5 timestamps (HH:MM)
}

impl ErrorEntry {
    pub fn new(name: &str, time: &str) -> Self {
        let mut times = VecDeque::with_capacity(5);
        times.push_front(time.to_string());
        Self {
            name: name.to_string(),
            times,
        }
    }

    /// Add a new timestamp (keeps only last 5)
    pub fn add_time(&mut self, time: &str) {
        self.times.push_front(time.to_string());
        while self.times.len() > 5 {
            self.times.pop_back();
        }
    }

    /// Format times as comma-separated string
    pub fn format_times(&self) -> String {
        self.times.iter().cloned().collect::<Vec<_>>().join(", ")
    }
}

/// Log of errors grouped by type
pub struct ErrorLog {
    pub entries: Vec<ErrorEntry>,
}

impl ErrorLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Process a report and extract any errors/warnings
    pub fn process_report(&mut self, report: &DiagnosticReport) {
        // Extract HH:MM from timestamp
        let time = report.timestamp
            .as_ref()
            .map(|t| {
                if t.len() >= 16 {
                    t[11..16].to_string() // HH:MM
                } else {
                    t.clone()
                }
            })
            .unwrap_or_else(|| "--:--".to_string());

        // Check each result for errors/warnings
        let checks: Vec<Option<&CheckResult>> = vec![
            report.local_resources.as_ref(),
            report.gpu.as_ref(),
            report.internet.as_ref(),
            report.claude_api.as_ref(),
            report.openai_api.as_ref(),
            report.google_api.as_ref(),
            report.opencode.as_ref(),
            report.terminals.as_ref(),
        ];

        for check in checks.into_iter().flatten() {
            if check.status == CheckStatus::Error || check.status == CheckStatus::Warning {
                self.add_error(&check.name, &time);
            }
        }
    }

    /// Add an error occurrence
    fn add_error(&mut self, name: &str, time: &str) {
        // Find existing entry or create new
        if let Some(entry) = self.entries.iter_mut().find(|e| e.name == name) {
            entry.add_time(time);
        } else {
            self.entries.push(ErrorEntry::new(name, time));
        }
    }

    /// Get number of unique error types
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if no errors recorded
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

fn format_check_for_report(check: &CheckResult) -> String {
    let icon = match check.status {
        CheckStatus::Ok => "[OK]",
        CheckStatus::Warning => "[!!]",
        CheckStatus::Error => "[XX]",
        CheckStatus::Unknown => "[??]",
        CheckStatus::Inactive => "[--]",
    };
    
    let mut result = format!("{} {}\n", icon, check.name);
    result.push_str(&format!("     {}\n", check.details));
    if let Some(ref msg) = check.message {
        result.push_str(&format!("     Message: \"{}\"\n", msg));
    }
    result.push('\n');
    result
}

/// Check local system resources (CPU, RAM)
pub fn check_local_resources() -> CheckResult {
    let mut sys = System::new_all();
    sys.refresh_all();
    
    // Need to wait a bit and refresh again for accurate CPU reading
    std::thread::sleep(std::time::Duration::from_millis(200));
    sys.refresh_cpu_all();

    let cpu_usage = sys.global_cpu_usage();
    let total_mem = sys.total_memory();
    let used_mem = sys.used_memory();
    let mem_percent = if total_mem > 0 {
        (used_mem as f64 / total_mem as f64 * 100.0) as u32
    } else {
        0
    };

    let details = format!(
        "CPU: {}% :: RAM: {}%",
        cpu_usage as u32,
        mem_percent
    );

    let status = if cpu_usage > 90.0 || mem_percent > 95 {
        CheckStatus::Error
    } else if cpu_usage > 70.0 || mem_percent > 85 {
        CheckStatus::Warning
    } else {
        CheckStatus::Ok
    };

    CheckResult::new("LOCAL RESOURCES", status, &details)
}

/// Check internet connectivity by making HTTP requests
pub fn check_internet() -> CheckResult {
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => {
            return CheckResult::new("INTERNET", CheckStatus::Error, "Failed to create HTTP client");
        }
    };

    let start = Instant::now();
    
    // Try Google
    let google_ok = client.get("https://www.google.com")
        .send()
        .map(|r| r.status().is_success())
        .unwrap_or(false);

    let elapsed = start.elapsed().as_millis();

    if google_ok {
        let status = if elapsed > 2000 {
            CheckStatus::Warning
        } else {
            CheckStatus::Ok
        };
        
        CheckResult::new(
            "INTERNET",
            status,
            &format!("PING: {}ms :: google.com reachable", elapsed),
        )
    } else {
        // Try Cloudflare as backup
        let cf_ok = client.get("https://1.1.1.1")
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or(false);

        if cf_ok {
            CheckResult::new("INTERNET", CheckStatus::Warning, "google.com unreachable, cloudflare OK")
        } else {
            CheckResult::new("INTERNET", CheckStatus::Error, "No internet connection")
        }
    }
}
