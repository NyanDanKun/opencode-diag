//! Diagnostic settings with serialization support

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Preset intervals for auto-refresh (in seconds)
pub const REFRESH_PRESETS: &[(u32, &str)] = &[
    (30, "30s"),
    (60, "1m"),
    (120, "2m"),
    (300, "5m"),
];

/// Preset scale values
pub const SCALE_PRESETS: &[(f32, &str)] = &[
    (1.0, "100%"),
    (1.25, "125%"),
    (1.5, "150%"),
    (2.0, "200%"),
];

/// Settings for which checks to perform
#[derive(Clone, Serialize, Deserialize)]
pub struct DiagnosticSettings {
    // System
    pub check_cpu_ram: bool,
    pub check_gpu: bool,
    
    // Network
    pub check_internet: bool,
    
    // APIs
    pub check_claude: bool,
    pub check_openai: bool,
    pub check_google_ai: bool,
    
    // Processes
    pub check_opencode: bool,
    pub check_terminals: bool,
    
    // Auto-refresh
    pub auto_refresh: bool,
    pub refresh_interval_secs: u32,
    
    // UI Scale
    pub ui_scale: f32,
    
    // History (unused now, kept for compatibility)
    pub max_history_entries: usize,
}

impl Default for DiagnosticSettings {
    fn default() -> Self {
        Self {
            // System - CPU/RAM enabled, GPU disabled (experimental)
            check_cpu_ram: true,
            check_gpu: false,  // Disabled by default - WMI issues on some systems
            
            // Network - enabled by default
            check_internet: true,
            
            // APIs - only Claude by default
            check_claude: true,
            check_openai: false,
            check_google_ai: false,
            
            // Processes - opencode by default
            check_opencode: true,
            check_terminals: false,
            
            // Auto-refresh - disabled by default, 60s interval
            auto_refresh: false,
            refresh_interval_secs: 60,
            
            // UI Scale - 100%
            ui_scale: 1.0,
            
            // History - keep last 10 reports
            max_history_entries: 10,
        }
    }
}

impl DiagnosticSettings {
    /// Get the settings file path
    fn settings_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("opencode-diag").join("settings.json"))
    }

    /// Load settings from file or return defaults
    pub fn load() -> Self {
        if let Some(path) = Self::settings_path() {
            if path.exists() {
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    if let Ok(settings) = serde_json::from_str(&contents) {
                        return settings;
                    }
                }
            }
        }
        Self::default()
    }

    /// Save settings to file
    pub fn save(&self) -> Result<(), String> {
        let path = Self::settings_path().ok_or("Could not determine config directory")?;
        
        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;
        
        std::fs::write(&path, json)
            .map_err(|e| format!("Failed to write settings file: {}", e))?;

        Ok(())
    }

    /// Count how many checks are enabled
    pub fn enabled_count(&self) -> usize {
        let mut count = 0;
        if self.check_cpu_ram { count += 1; }
        if self.check_gpu { count += 1; }
        if self.check_internet { count += 1; }
        if self.check_claude { count += 1; }
        if self.check_openai { count += 1; }
        if self.check_google_ai { count += 1; }
        if self.check_opencode { count += 1; }
        if self.check_terminals { count += 1; }
        count
    }
    
    /// Get the current interval preset index
    pub fn current_preset_index(&self) -> usize {
        REFRESH_PRESETS.iter()
            .position(|(secs, _)| *secs == self.refresh_interval_secs)
            .unwrap_or(1) // Default to 1m if not found
    }
    
    /// Set interval from preset index
    pub fn set_preset(&mut self, index: usize) {
        if index < REFRESH_PRESETS.len() {
            self.refresh_interval_secs = REFRESH_PRESETS[index].0;
        }
    }
    
    /// Format the current interval for display
    pub fn format_interval(&self) -> String {
        if self.refresh_interval_secs >= 60 {
            format!("{}m", self.refresh_interval_secs / 60)
        } else {
            format!("{}s", self.refresh_interval_secs)
        }
    }
    
    /// Get the current scale preset index (or None if custom)
    pub fn current_scale_index(&self) -> Option<usize> {
        SCALE_PRESETS.iter()
            .position(|(scale, _)| (*scale - self.ui_scale).abs() < 0.01)
    }
    
    /// Set scale from preset index
    pub fn set_scale_preset(&mut self, index: usize) {
        if index < SCALE_PRESETS.len() {
            self.ui_scale = SCALE_PRESETS[index].0;
        }
    }
    
    /// Adjust scale by delta (for Ctrl+scroll)
    pub fn adjust_scale(&mut self, delta: f32) {
        self.ui_scale = (self.ui_scale + delta).clamp(0.75, 2.5);
    }
    
    /// Format scale as percentage
    pub fn format_scale(&self) -> String {
        format!("{}%", (self.ui_scale * 100.0) as u32)
    }
}
