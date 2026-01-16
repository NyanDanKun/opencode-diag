//! OpenCode Diagnostics Tool
//! 
//! Diagnoses "server at capacity" and other connection issues.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // Hide console on Windows

mod theme;
mod diagnostics;

use eframe::egui;
use theme::{Theme, ThemeMode, apply_theme};
use diagnostics::{DiagnosticReport, ErrorLog, CheckResult, CheckStatus, DiagnosticSettings};
use diagnostics::settings::{REFRESH_PRESETS, SCALE_PRESETS};
use arboard::Clipboard;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Detect system theme (Windows)
#[cfg(target_os = "windows")]
fn detect_system_theme() -> ThemeMode {
    use std::process::Command;
    
    // Query registry for AppsUseLightTheme
    // 0 = Dark, 1 = Light
    let output = Command::new("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
            "/v",
            "AppsUseLightTheme",
        ])
        .output();
    
    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Look for "0x0" (dark) or "0x1" (light)
        if stdout.contains("0x0") {
            return ThemeMode::Dark;
        } else if stdout.contains("0x1") {
            return ThemeMode::Light;
        }
    }
    
    // Default to dark
    ThemeMode::Dark
}

#[cfg(not(target_os = "windows"))]
fn detect_system_theme() -> ThemeMode {
    ThemeMode::Dark
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([550.0, 580.0])
            .with_min_inner_size([450.0, 450.0]),
        ..Default::default()
    };

    eframe::run_native(
        "OpenCode Diagnostics",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

struct App {
    theme_mode: ThemeMode,
    theme: Theme,
    status: String,
    report: Arc<Mutex<DiagnosticReport>>,
    is_running: Arc<Mutex<bool>>,
    just_completed: Arc<Mutex<bool>>, // Flag to know when run completed
    copied_feedback: Option<Instant>,
    // Settings
    settings: DiagnosticSettings,
    show_settings: bool,
    // Auto-refresh
    last_refresh: Option<Instant>,
    // Error log (grouped by error type)
    error_log: ErrorLog,
    show_history: bool,
}

impl App {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let settings = DiagnosticSettings::load();
        
        // Detect system theme
        let theme_mode = detect_system_theme();
        let theme = Theme::from_mode(theme_mode);
        
        Self {
            theme_mode,
            theme,
            status: "SYS.STATUS: READY".to_string(),
            report: Arc::new(Mutex::new(DiagnosticReport::new())),
            is_running: Arc::new(Mutex::new(false)),
            just_completed: Arc::new(Mutex::new(false)),
            copied_feedback: None,
            settings,
            show_settings: false,
            // Auto-refresh
            last_refresh: None,
            // Error log
            error_log: ErrorLog::new(),
            show_history: false,
        }
    }

    fn toggle_theme(&mut self) {
        self.theme_mode = match self.theme_mode {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Light,
        };
        self.theme = Theme::from_mode(self.theme_mode);
    }

    fn run_diagnostics(&mut self, ctx: &egui::Context) {
        // Check if already running
        {
            let mut is_running = self.is_running.lock().unwrap();
            if *is_running {
                return;
            }
            *is_running = true;
        }

        self.status = "SYS.STATUS: RUNNING DIAGNOSTICS...".to_string();

        let report = Arc::clone(&self.report);
        let is_running = Arc::clone(&self.is_running);
        let just_completed = Arc::clone(&self.just_completed);
        let ctx = ctx.clone();
        let settings = self.settings.clone();

        thread::spawn(move || {
            // Run checks based on settings
            let mut new_report = DiagnosticReport::new();
            new_report.run_with_settings(&settings);

            // Update report
            {
                let mut r = report.lock().unwrap();
                *r = new_report;
            }

            // Mark as complete
            {
                let mut running = is_running.lock().unwrap();
                *running = false;
            }
            
            // Signal completion for history
            {
                let mut completed = just_completed.lock().unwrap();
                *completed = true;
            }

            // Request repaint
            ctx.request_repaint();
        });
    }

    fn copy_report(&mut self) {
        if let Ok(report) = self.report.lock() {
            let text = report.to_text_report();
            if let Ok(mut clipboard) = Clipboard::new() {
                if clipboard.set_text(&text).is_ok() {
                    self.copied_feedback = Some(std::time::Instant::now());
                    self.status = "SYS.STATUS: REPORT COPIED".to_string();
                }
            }
        }
    }

    fn status_color(&self, status: CheckStatus) -> egui::Color32 {
        match status {
            CheckStatus::Ok => {
                if self.theme_mode == ThemeMode::Dark {
                    egui::Color32::from_rgb(0x4c, 0xaf, 0x50) // Green
                } else {
                    self.theme.text
                }
            }
            CheckStatus::Warning => egui::Color32::from_rgb(0xff, 0x98, 0x00), // Orange
            CheckStatus::Error => egui::Color32::from_rgb(0xf4, 0x43, 0x36),   // Red
            CheckStatus::Unknown => self.theme.text_dim,
            CheckStatus::Inactive => self.theme.text_dim,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply UI scale
        ctx.set_pixels_per_point(self.settings.ui_scale);
        
        // Handle Ctrl+scroll for zoom
        let scroll_delta = ctx.input(|i| i.raw_scroll_delta.y);
        let ctrl_held = ctx.input(|i| i.modifiers.ctrl);
        if ctrl_held && scroll_delta != 0.0 {
            let delta = if scroll_delta > 0.0 { 0.1 } else { -0.1 };
            self.settings.adjust_scale(delta);
        }
        
        apply_theme(ctx, &self.theme);

        // Handle completed diagnostics - process errors for log
        {
            let mut just_completed = self.just_completed.lock().unwrap();
            if *just_completed {
                *just_completed = false;
                self.last_refresh = Some(Instant::now());
                
                // Process report for error log
                if let Ok(report) = self.report.lock() {
                    self.error_log.process_report(&report);
                }
            }
        }

        // Auto-refresh logic
        if self.settings.auto_refresh && !*self.is_running.lock().unwrap() {
            if let Some(last) = self.last_refresh {
                let elapsed = last.elapsed().as_secs() as u32;
                if elapsed >= self.settings.refresh_interval_secs {
                    self.run_diagnostics(ctx);
                }
            }
            // Request repaint every second for timer updates
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }

        // Check if copied feedback should be cleared
        if let Some(instant) = self.copied_feedback {
            if instant.elapsed().as_secs() >= 2 {
                self.copied_feedback = None;
                self.status = "SYS.STATUS: READY".to_string();
            }
        }

        // Update status if running
        if *self.is_running.lock().unwrap() {
            self.status = "SYS.STATUS: RUNNING DIAGNOSTICS...".to_string();
        } else if let Ok(report) = self.report.lock() {
            if let Some(ref diag) = report.diagnosis {
                if !diag.contains("operational") {
                    self.status = "SYS.STATUS: ISSUE FOUND".to_string();
                }
            }
        }

        // Header
        egui::TopBottomPanel::top("header")
            .frame(egui::Frame::none().fill(self.theme.header))
            .show(ctx, |ui| {
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    
                    // Square indicator (glows in dark mode)
                    let indicator_color = if self.theme_mode == ThemeMode::Dark {
                        self.theme.accent_on
                    } else {
                        self.theme.text
                    };
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(10.0, 10.0),
                        egui::Sense::hover(),
                    );
                    ui.painter().rect_filled(rect, 0.0, indicator_color);
                    
                    ui.add_space(10.0);
                    
                    // Title
                    ui.label(
                        egui::RichText::new("OPENCODE DIAGNOSTICS")
                            .size(14.0)
                            .strong()
                            .color(self.theme.text),
                    );
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(20.0);
                        
                        // Theme toggle button
                        let btn_text = match self.theme_mode {
                            ThemeMode::Light => "DARK",
                            ThemeMode::Dark => "LIGHT",
                        };
                        
                        if ui.add(
                            egui::Button::new(
                                egui::RichText::new(btn_text)
                                    .size(9.0)
                                    .strong()
                                    .family(egui::FontFamily::Monospace)
                                    .color(self.theme.text_dim)
                            )
                            .fill(self.theme.panel)
                            .stroke(egui::Stroke::NONE)
                            .rounding(0.0)
                            .min_size(egui::vec2(50.0, 24.0))
                        ).clicked() {
                            self.toggle_theme();
                        }
                    });
                });
                
                ui.add_space(12.0);
            });

        // Footer
        egui::TopBottomPanel::bottom("footer")
            .frame(egui::Frame::none().fill(self.theme.window))
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.add_space(25.0);
                    ui.label(
                        egui::RichText::new(&self.status)
                            .size(9.0)
                            .family(egui::FontFamily::Monospace)
                            .color(self.theme.text_dim),
                    );
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(25.0);
                        ui.label(
                            egui::RichText::new(format!("v{}", VERSION))
                                .size(9.0)
                                .family(egui::FontFamily::Monospace)
                                .color(self.theme.text_dim),
                        );
                        
                        ui.add_space(15.0);
                        
                        // Show last check time and next refresh
                        if let Some(last) = self.last_refresh {
                            let elapsed = last.elapsed().as_secs();
                            let ago_str = if elapsed < 60 {
                                format!("{}s ago", elapsed)
                            } else {
                                format!("{}m ago", elapsed / 60)
                            };
                            
                            let time_info = if self.settings.auto_refresh {
                                let remaining = self.settings.refresh_interval_secs.saturating_sub(elapsed as u32);
                                format!("LAST: {} | NEXT: {}s", ago_str, remaining)
                            } else {
                                format!("LAST: {}", ago_str)
                            };
                            
                            ui.label(
                                egui::RichText::new(&time_info)
                                    .size(9.0)
                                    .family(egui::FontFamily::Monospace)
                                    .color(self.theme.text_dim),
                            );
                        }
                    });
                });
                ui.add_space(10.0);
            });

        // Settings popup - using Area instead of Window for better control
        if self.show_settings {
            // Check for click outside to close
            let popup_id = egui::Id::new("settings_popup");
            
            // Draw a transparent overlay to detect clicks outside
            let screen_rect = ctx.screen_rect();
            let response = egui::Area::new(egui::Id::new("settings_overlay"))
                .fixed_pos(screen_rect.min)
                .order(egui::Order::Background)
                .show(ctx, |ui| {
                    let response = ui.allocate_response(screen_rect.size(), egui::Sense::click());
                    response
                });
            
            if response.inner.clicked() {
                self.show_settings = false;
                // Save settings when closing
                let _ = self.settings.save();
            }
            
            // The actual popup
            egui::Area::new(popup_id)
                .anchor(egui::Align2::RIGHT_TOP, [-25.0, 85.0])
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(self.theme.panel)
                        .stroke(egui::Stroke::new(1.0, self.theme.border))
                        .rounding(0.0)  // Sharp corners
                        .shadow(egui::Shadow::NONE)  // No shadow
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.set_min_width(180.0);
                            
                            // System section
                            ui.label(
                                egui::RichText::new("// SYSTEM")
                                    .size(9.0)
                                    .family(egui::FontFamily::Monospace)
                                    .color(self.theme.text_dim),
                            );
                            ui.add_space(5.0);
                            let text_color = self.theme.text;
                            App::render_styled_checkbox(ui, &mut self.settings.check_cpu_ram, "CPU / RAM", text_color);
                            
                            // GPU with warning icon (experimental feature)
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut self.settings.check_gpu, "");
                                ui.add_space(-5.0);
                                if ui.add(
                                    egui::Label::new(
                                        egui::RichText::new("GPU")
                                            .size(10.0)
                                            .family(egui::FontFamily::Monospace)
                                            .color(text_color)
                                    ).sense(egui::Sense::click())
                                ).clicked() {
                                    self.settings.check_gpu = !self.settings.check_gpu;
                                }
                                ui.label(
                                    egui::RichText::new("!")
                                        .size(10.0)
                                        .strong()
                                        .color(egui::Color32::from_rgb(0xff, 0x98, 0x00)) // Orange warning
                                ).on_hover_text("Experimental: May not work on all systems");
                            });
                            
                            ui.add_space(8.0);
                            ui.add(egui::Separator::default().spacing(1.0));
                            ui.add_space(8.0);
                            
                            // Network section
                            ui.label(
                                egui::RichText::new("// NETWORK")
                                    .size(9.0)
                                    .family(egui::FontFamily::Monospace)
                                    .color(self.theme.text_dim),
                            );
                            ui.add_space(5.0);
                            App::render_styled_checkbox(ui, &mut self.settings.check_internet, "Internet", text_color);
                            
                            ui.add_space(8.0);
                            ui.add(egui::Separator::default().spacing(1.0));
                            ui.add_space(8.0);
                            
                            // APIs section
                            ui.label(
                                egui::RichText::new("// AI APIS")
                                    .size(9.0)
                                    .family(egui::FontFamily::Monospace)
                                    .color(self.theme.text_dim),
                            );
                            ui.add_space(5.0);
                            App::render_styled_checkbox(ui, &mut self.settings.check_claude, "Claude", text_color);
                            App::render_styled_checkbox(ui, &mut self.settings.check_openai, "OpenAI", text_color);
                            App::render_styled_checkbox(ui, &mut self.settings.check_google_ai, "Google AI", text_color);
                            
                            ui.add_space(8.0);
                            ui.add(egui::Separator::default().spacing(1.0));
                            ui.add_space(8.0);
                            
                            // Processes section
                            ui.label(
                                egui::RichText::new("// PROCESSES")
                                    .size(9.0)
                                    .family(egui::FontFamily::Monospace)
                                    .color(self.theme.text_dim),
                            );
                            ui.add_space(5.0);
                            App::render_styled_checkbox(ui, &mut self.settings.check_opencode, "OpenCode", text_color);
                            App::render_styled_checkbox(ui, &mut self.settings.check_terminals, "Terminals", text_color);
                            
                            ui.add_space(8.0);
                            ui.add(egui::Separator::default().spacing(1.0));
                            ui.add_space(8.0);
                            
                            // Auto-refresh section
                            ui.label(
                                egui::RichText::new("// AUTO-REFRESH")
                                    .size(9.0)
                                    .family(egui::FontFamily::Monospace)
                                    .color(self.theme.text_dim),
                            );
                            ui.add_space(5.0);
                            
                            // Enable/disable checkbox
                            App::render_styled_checkbox(ui, &mut self.settings.auto_refresh, "Enabled", text_color);
                            
                            // Interval selector (only show if enabled)
                            if self.settings.auto_refresh {
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    ui.add_space(22.0); // Align with checkboxes
                                    ui.label(
                                        egui::RichText::new("Interval:")
                                            .size(9.0)
                                            .family(egui::FontFamily::Monospace)
                                            .color(self.theme.text_dim),
                                    );
                                    ui.add_space(5.0);
                                    
                                    // Preset buttons
                                    for (i, (_, label)) in REFRESH_PRESETS.iter().enumerate() {
                                        let is_selected = self.settings.current_preset_index() == i;
                                        let btn = egui::Button::new(
                                            egui::RichText::new(*label)
                                                .size(9.0)
                                                .family(egui::FontFamily::Monospace)
                                                .color(if is_selected { 
                                                    egui::Color32::WHITE 
                                                } else { 
                                                    self.theme.text 
                                                })
                                        )
                                        .fill(if is_selected { 
                                            self.theme.accent_on 
                                        } else { 
                                            self.theme.panel 
                                        })
                                        .stroke(egui::Stroke::new(1.0, self.theme.border))
                                        .rounding(0.0)
                                        .min_size(egui::vec2(30.0, 18.0));
                                        
                                        if ui.add(btn).clicked() {
                                            self.settings.set_preset(i);
                                        }
                                    }
                                });
                            }
                            
                            ui.add_space(8.0);
                            ui.add(egui::Separator::default().spacing(1.0));
                            ui.add_space(8.0);
                            
                            // Scale section
                            ui.label(
                                egui::RichText::new("// SCALE")
                                    .size(9.0)
                                    .family(egui::FontFamily::Monospace)
                                    .color(self.theme.text_dim),
                            );
                            ui.add_space(5.0);
                            
                            ui.horizontal(|ui| {
                                // Current scale display
                                ui.label(
                                    egui::RichText::new(self.settings.format_scale())
                                        .size(9.0)
                                        .family(egui::FontFamily::Monospace)
                                        .color(self.theme.text_dim),
                                );
                                ui.add_space(10.0);
                                
                                // Preset buttons
                                for (i, (_, label)) in SCALE_PRESETS.iter().enumerate() {
                                    let is_selected = self.settings.current_scale_index() == Some(i);
                                    let btn = egui::Button::new(
                                        egui::RichText::new(*label)
                                            .size(9.0)
                                            .family(egui::FontFamily::Monospace)
                                            .color(if is_selected { 
                                                egui::Color32::WHITE 
                                            } else { 
                                                self.theme.text 
                                            })
                                    )
                                    .fill(if is_selected { 
                                        self.theme.accent_on 
                                    } else { 
                                        self.theme.panel 
                                    })
                                    .stroke(egui::Stroke::new(1.0, self.theme.border))
                                    .rounding(0.0)
                                    .min_size(egui::vec2(40.0, 18.0));
                                    
                                    if ui.add(btn).clicked() {
                                        self.settings.set_scale_preset(i);
                                    }
                                }
                            });
                            
                            ui.add_space(3.0);
                            ui.label(
                                egui::RichText::new("Ctrl+Scroll to adjust")
                                    .size(8.0)
                                    .family(egui::FontFamily::Monospace)
                                    .color(self.theme.text_dim),
                            );
                        });
                });
        }

        // Error Log popup
        if self.show_history {
            // Check for click outside to close
            let popup_id = egui::Id::new("history_popup");
            
            // Draw a transparent overlay to detect clicks outside
            let screen_rect = ctx.screen_rect();
            let response = egui::Area::new(egui::Id::new("history_overlay"))
                .fixed_pos(screen_rect.min)
                .order(egui::Order::Background)
                .show(ctx, |ui| {
                    let response = ui.allocate_response(screen_rect.size(), egui::Sense::click());
                    response
                });
            
            if response.inner.clicked() {
                self.show_history = false;
            }
            
            // The actual popup
            egui::Area::new(popup_id)
                .anchor(egui::Align2::RIGHT_TOP, [-25.0, 85.0])
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(self.theme.panel)
                        .stroke(egui::Stroke::new(1.0, self.theme.border))
                        .rounding(0.0)
                        .shadow(egui::Shadow::NONE)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.set_min_width(280.0);
                            
                            ui.label(
                                egui::RichText::new("// ERROR LOG")
                                    .size(9.0)
                                    .family(egui::FontFamily::Monospace)
                                    .color(self.theme.text_dim),
                            );
                            ui.add_space(8.0);
                            
                            if self.error_log.entries.is_empty() {
                                ui.label(
                                    egui::RichText::new("No issues recorded.")
                                        .size(9.0)
                                        .family(egui::FontFamily::Monospace)
                                        .color(self.theme.text_dim),
                                );
                            } else {
                                // Show grouped errors
                                for entry in &self.error_log.entries {
                                    ui.horizontal(|ui| {
                                        // Error type name (fixed width)
                                        ui.label(
                                            egui::RichText::new(&entry.name)
                                                .size(9.0)
                                                .family(egui::FontFamily::Monospace)
                                                .strong()
                                                .color(egui::Color32::from_rgb(0xf4, 0x43, 0x36)), // Red
                                        );
                                        
                                        ui.add_space(10.0);
                                        
                                        // Timestamps (comma-separated)
                                        ui.label(
                                            egui::RichText::new(entry.format_times())
                                                .size(9.0)
                                                .family(egui::FontFamily::Monospace)
                                                .color(self.theme.text_dim),
                                        );
                                    });
                                    ui.add_space(3.0);
                                }
                            }
                        });
                });
        }

        // Main content
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(self.theme.window).inner_margin(25.0))
            .show(ctx, |ui| {
                // Section header with settings button
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("// SYSTEM CHECK")
                            .size(10.0)
                            .family(egui::FontFamily::Monospace)
                            .color(self.theme.text_dim),
                    );
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // SETTINGS text button with border and hover effect (like COPY REPORT)
                        let settings_btn = egui::Button::new(
                            egui::RichText::new("SETTINGS")
                                .size(9.0)
                                .strong()
                                .family(egui::FontFamily::Monospace)
                                .color(if self.show_settings { 
                                    self.theme.accent_on 
                                } else { 
                                    self.theme.text 
                                })
                        )
                        .fill(self.theme.panel)
                        .stroke(egui::Stroke::new(1.0, self.theme.border))
                        .rounding(0.0)
                        .min_size(egui::vec2(70.0, 22.0));
                        
                        if ui.add(settings_btn).clicked() {
                            self.show_settings = !self.show_settings;
                            self.show_history = false; // Close history when opening settings
                        }
                        
                        ui.add_space(5.0);
                        
                        // LOG button for error log
                        let log_count = self.error_log.len();
                        let log_label = if log_count > 0 {
                            format!("LOG ({})", log_count)
                        } else {
                            "LOG".to_string()
                        };
                        let log_btn = egui::Button::new(
                            egui::RichText::new(&log_label)
                                .size(9.0)
                                .strong()
                                .family(egui::FontFamily::Monospace)
                                .color(if self.show_history { 
                                    self.theme.accent_on 
                                } else { 
                                    self.theme.text 
                                })
                        )
                        .fill(self.theme.panel)
                        .stroke(egui::Stroke::new(1.0, self.theme.border))
                        .rounding(0.0)
                        .min_size(egui::vec2(55.0, 22.0));
                        
                        if ui.add(log_btn).clicked() {
                            self.show_history = !self.show_history;
                            self.show_settings = false; // Close settings when opening log
                        }
                        
                        ui.add_space(10.0);
                        
                        // Show enabled checks count
                        ui.label(
                            egui::RichText::new(format!("{} checks", self.settings.enabled_count()))
                                .size(9.0)
                                .family(egui::FontFamily::Monospace)
                                .color(self.theme.text_dim),
                        );
                    });
                });
                
                ui.add_space(15.0);

                // Calculate available height for scroll area
                let available_height = ui.available_height() - 60.0; // Reserve space for buttons

                // Scrollable area for check cards - now fills available height
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])  // Don't shrink
                    .max_height(available_height)
                    .show(ui, |ui| {
                        // Get report data
                        let report = self.report.lock().unwrap().clone();

                        // Render cards based on settings
                        if self.settings.check_cpu_ram {
                            if let Some(ref check) = report.local_resources {
                                self.render_check_card(ui, check);
                            } else {
                                self.render_placeholder_card(ui, "LOCAL RESOURCES", "CPU :: RAM");
                            }
                        }

                        if self.settings.check_gpu {
                            if let Some(ref check) = report.gpu {
                                self.render_check_card(ui, check);
                            } else {
                                self.render_placeholder_card(ui, "GPU", "Video card status");
                            }
                        }

                        if self.settings.check_internet {
                            if let Some(ref check) = report.internet {
                                self.render_check_card(ui, check);
                            } else {
                                self.render_placeholder_card(ui, "INTERNET", "Connectivity check");
                            }
                        }

                        if self.settings.check_claude {
                            if let Some(ref check) = report.claude_api {
                                self.render_check_card(ui, check);
                            } else {
                                self.render_placeholder_card(ui, "CLAUDE API", "api.anthropic.com");
                            }
                        }

                        if self.settings.check_openai {
                            if let Some(ref check) = report.openai_api {
                                self.render_check_card(ui, check);
                            } else {
                                self.render_placeholder_card(ui, "OPENAI API", "api.openai.com");
                            }
                        }

                        if self.settings.check_google_ai {
                            if let Some(ref check) = report.google_api {
                                self.render_check_card(ui, check);
                            } else {
                                self.render_placeholder_card(ui, "GOOGLE AI", "googleapis.com");
                            }
                        }

                        if self.settings.check_opencode {
                            if let Some(ref check) = report.opencode {
                                self.render_check_card(ui, check);
                            } else {
                                self.render_placeholder_card(ui, "OPENCODE", "Process status");
                            }
                        }

                        if self.settings.check_terminals {
                            if let Some(ref check) = report.terminals {
                                self.render_check_card(ui, check);
                            } else {
                                self.render_placeholder_card(ui, "TERMINALS", "cmd, powershell, wt");
                            }
                        }

                        // Diagnosis
                        if let Some(ref diagnosis) = report.diagnosis {
                            ui.add_space(10.0);
                            egui::Frame::none()
                                .fill(self.theme.panel)
                                .inner_margin(egui::Margin::symmetric(15.0, 10.0))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new("DIAGNOSIS:")
                                                .size(10.0)
                                                .family(egui::FontFamily::Monospace)
                                                .strong()
                                                .color(self.theme.text),
                                        );
                                        ui.label(
                                            egui::RichText::new(diagnosis)
                                                .size(10.0)
                                                .family(egui::FontFamily::Monospace)
                                                .color(self.theme.text_dim),
                                        );
                                    });
                                });
                        }
                    });

                ui.add_space(15.0);

                // Action buttons
                ui.horizontal(|ui| {
                    let is_running = *self.is_running.lock().unwrap();
                    
                    // RUN DIAGNOSTICS button
                    let run_btn_text = if is_running { "RUNNING..." } else { "RUN DIAGNOSTICS" };
                    let run_btn = egui::Button::new(
                        egui::RichText::new(run_btn_text)
                            .size(11.0)
                            .strong()
                            .family(egui::FontFamily::Monospace)
                            .color(egui::Color32::WHITE)
                    )
                    .fill(self.theme.accent_on)
                    .stroke(egui::Stroke::NONE)
                    .rounding(0.0)
                    .min_size(egui::vec2(160.0, 32.0));

                    if ui.add_enabled(!is_running, run_btn).clicked() {
                        self.run_diagnostics(ctx);
                    }

                    ui.add_space(10.0);

                    // COPY REPORT button
                    let copy_text = if self.copied_feedback.is_some() { "COPIED!" } else { "COPY REPORT" };
                    if ui.add(
                        egui::Button::new(
                            egui::RichText::new(copy_text)
                                .size(11.0)
                                .strong()
                                .family(egui::FontFamily::Monospace)
                                .color(self.theme.text)
                        )
                        .fill(self.theme.panel)
                        .stroke(egui::Stroke::new(1.0, self.theme.border))
                        .rounding(0.0)
                        .min_size(egui::vec2(130.0, 32.0))
                    ).clicked() {
                        self.copy_report();
                    }
                });
            });
    }
}

impl App {
    /// Render a styled checkbox with monospace label (whole row is interactive)
    fn render_styled_checkbox(ui: &mut egui::Ui, value: &mut bool, label: &str, text_color: egui::Color32) {
        // Allocate space for the whole row
        let desired_size = egui::vec2(ui.available_width().min(200.0), 18.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
        
        if response.clicked() {
            *value = !*value;
        }
        
        // Determine colors based on hover
        let label_color = if response.hovered() {
            // Brighter on hover
            egui::Color32::from_rgba_unmultiplied(text_color.r(), text_color.g(), text_color.b(), 255)
        } else {
            text_color
        };
        
        // Draw checkbox manually
        let checkbox_size = 14.0;
        let checkbox_rect = egui::Rect::from_min_size(
            rect.min + egui::vec2(0.0, (rect.height() - checkbox_size) / 2.0),
            egui::vec2(checkbox_size, checkbox_size),
        );
        
        // Checkbox background
        let checkbox_fill = if *value {
            ui.visuals().widgets.active.bg_fill
        } else {
            ui.visuals().widgets.inactive.bg_fill
        };
        ui.painter().rect_filled(checkbox_rect, 2.0, checkbox_fill);
        ui.painter().rect_stroke(checkbox_rect, 2.0, egui::Stroke::new(1.0, ui.visuals().widgets.inactive.fg_stroke.color));
        
        // Checkmark
        if *value {
            let check_color = ui.visuals().widgets.active.fg_stroke.color;
            let center = checkbox_rect.center();
            let size = checkbox_size * 0.3;
            ui.painter().line_segment(
                [center + egui::vec2(-size, 0.0), center + egui::vec2(-size * 0.3, size * 0.7)],
                egui::Stroke::new(2.0, check_color),
            );
            ui.painter().line_segment(
                [center + egui::vec2(-size * 0.3, size * 0.7), center + egui::vec2(size, -size * 0.5)],
                egui::Stroke::new(2.0, check_color),
            );
        }
        
        // Label
        let label_pos = rect.min + egui::vec2(checkbox_size + 6.0, (rect.height() - 10.0) / 2.0);
        ui.painter().text(
            label_pos,
            egui::Align2::LEFT_TOP,
            label,
            egui::FontId::new(10.0, egui::FontFamily::Monospace),
            label_color,
        );
        
        // Hover highlight
        if response.hovered() {
            ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(1.0, label_color.gamma_multiply(0.3)));
        }
    }

    fn render_check_card(&mut self, ui: &mut egui::Ui, check: &CheckResult) {
        let status_color = self.status_color(check.status);
        
        egui::Frame::none()
            .fill(self.theme.panel)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Left accent bar
                    let accent_color = if ui.rect_contains_pointer(ui.max_rect()) {
                        self.theme.accent_on
                    } else {
                        status_color
                    };
                    
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(3.0, 50.0),
                        egui::Sense::hover(),
                    );
                    ui.painter().rect_filled(rect, 0.0, accent_color);
                    
                    ui.add_space(15.0);
                    
                    // Calculate available width for text (leave space for badge)
                    let badge_width = 70.0; // 55px button + 15px spacing
                    let available_width = ui.available_width() - badge_width - 20.0;
                    
                    // Content - constrained width
                    ui.vertical(|ui| {
                        ui.set_max_width(available_width.max(100.0));
                        ui.add_space(8.0);
                        
                        ui.label(
                            egui::RichText::new(&check.name)
                                .size(12.0)
                                .strong()
                                .color(self.theme.text),
                        );
                        
                        // Details with text wrapping
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&check.details)
                                    .size(9.0)
                                    .family(egui::FontFamily::Monospace)
                                    .color(self.theme.text_dim)
                            ).wrap()
                        );
                        
                        ui.add_space(8.0);
                    });
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(15.0);
                        
                        // Status badge
                        ui.add(
                            egui::Button::new(
                                egui::RichText::new(check.status.label())
                                    .size(10.0)
                                    .strong()
                                    .family(egui::FontFamily::Monospace)
                                    .color(if check.status == CheckStatus::Ok || check.status == CheckStatus::Inactive {
                                        egui::Color32::WHITE
                                    } else {
                                        egui::Color32::BLACK
                                    })
                            )
                            .fill(status_color)
                            .stroke(egui::Stroke::NONE)
                            .rounding(0.0)
                            .min_size(egui::vec2(55.0, 24.0))
                        );
                    });
                });
            });
        
        ui.add_space(5.0);
    }

    fn render_placeholder_card(&self, ui: &mut egui::Ui, name: &str, details: &str) {
        egui::Frame::none()
            .fill(self.theme.panel)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Left accent bar (dim)
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(3.0, 50.0),
                        egui::Sense::hover(),
                    );
                    ui.painter().rect_filled(rect, 0.0, self.theme.text_dim);
                    
                    ui.add_space(15.0);
                    
                    // Calculate available width for text
                    let badge_width = 70.0;
                    let available_width = ui.available_width() - badge_width - 20.0;
                    
                    // Content
                    ui.vertical(|ui| {
                        ui.set_max_width(available_width.max(100.0));
                        ui.add_space(8.0);
                        
                        ui.label(
                            egui::RichText::new(name)
                                .size(12.0)
                                .strong()
                                .color(self.theme.text_dim),
                        );
                        
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(details)
                                    .size(9.0)
                                    .family(egui::FontFamily::Monospace)
                                    .color(self.theme.text_dim)
                            ).wrap()
                        );
                        
                        ui.add_space(8.0);
                    });
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(15.0);
                        
                        // Status badge placeholder
                        ui.add(
                            egui::Button::new(
                                egui::RichText::new("...")
                                    .size(10.0)
                                    .family(egui::FontFamily::Monospace)
                                    .color(self.theme.text_dim)
                            )
                            .fill(self.theme.accent_off)
                            .stroke(egui::Stroke::NONE)
                            .rounding(0.0)
                            .min_size(egui::vec2(55.0, 24.0))
                        );
                    });
                });
            });
        
        ui.add_space(5.0);
    }
}
