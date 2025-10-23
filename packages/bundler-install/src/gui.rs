//! Native GUI for installation progress display
//!
//! Provides a professional branded window showing real-time installation
//! progress when launched from native installers (.app, .msi, .pkg).
//!
//! ## Architecture
//! - Main thread: Runs eframe GUI event loop (60 FPS)
//! - Background thread: Runs tokio installation task
//! - Communication: mpsc::UnboundedChannel for progress updates
//!
//! ## Integration
//! Receives InstallProgress from install_kodegen_daemon() via channel.
//! See: src/install/core.rs:152

use eframe::egui;
use tokio::sync::mpsc;
use std::sync::{Arc, Mutex};

use crate::install::core::InstallProgress;
use crate::wizard::InstallationResult;

/// Installation window state
pub struct InstallWindow {
    /// Progress receiver channel (Arc<Mutex<>> for thread safety)
    progress_rx: Arc<Mutex<mpsc::UnboundedReceiver<InstallProgress>>>,
    
    /// Current installation state (updated from channel)
    current_step: String,
    current_message: String,
    progress: f32,           // 0.0 to 1.0
    is_error: bool,
    is_complete: bool,
    
    /// Installation result (populated on completion)
    result: Option<InstallationResult>,
    
    /// Branding assets (loaded once at startup)
    banner: Option<egui::TextureHandle>,
}

impl InstallWindow {
    /// Create new installation window
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        progress_rx: mpsc::UnboundedReceiver<InstallProgress>,
    ) -> Self {
        // Configure dark theme (KODEGEN branding colors)
        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = egui::Color32::from_rgb(10, 25, 41);  // #0a1929 (dark blue)
        visuals.panel_fill = egui::Color32::from_rgb(5, 18, 38);    // #051226 (darker blue)
        cc.egui_ctx.set_visuals(visuals);
        
        // Load banner from embedded assets
        let banner = Self::load_banner(cc);
        
        Self {
            progress_rx: Arc::new(Mutex::new(progress_rx)),
            current_step: "Initializing...".to_string(),
            current_message: "Starting installation".to_string(),
            progress: 0.0,
            is_error: false,
            is_complete: false,
            result: None,
            banner,
        }
    }
    
    /// Load KODEGEN banner from embedded assets
    fn load_banner(cc: &eframe::CreationContext<'_>) -> Option<egui::TextureHandle> {
        // Embedded at compile time (see GUI_1 asset setup)
        let banner_bytes = include_bytes!("../assets/banner.png");
        
        // Decode PNG with image crate
        match image::load_from_memory(banner_bytes) {
            Ok(img) => {
                let img_rgba = img.to_rgba8();
                let size = [img_rgba.width() as usize, img_rgba.height() as usize];
                let pixels = img_rgba.into_raw();
                
                // Convert to egui color format
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                
                // Upload to GPU (one-time upload, reused every frame)
                Some(cc.egui_ctx.load_texture(
                    "banner",
                    color_image,
                    egui::TextureOptions::LINEAR,  // Linear filtering for smooth scaling
                ))
            }
            Err(e) => {
                eprintln!("Failed to load banner: {}", e);
                None  // Fallback to text title (handled in update())
            }
        }
    }
    
    /// Poll for progress updates (non-blocking)
    fn poll_progress(&mut self) {
        // try_lock() = non-blocking (won't stall GUI if contended)
        if let Ok(mut rx) = self.progress_rx.try_lock() {
            // try_recv() = non-blocking (returns immediately if empty)
            while let Ok(progress) = rx.try_recv() {
                self.current_step = progress.step;
                self.current_message = progress.message;
                self.progress = progress.progress;
                self.is_error = progress.is_error;
                
                // Check for completion
                if self.progress >= 1.0 && !self.is_error {
                    self.is_complete = true;
                }
            }
        }
        // If lock fails, skip this frame (will retry next frame at 60 FPS)
    }
}

impl eframe::App for InstallWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll for new progress updates (non-blocking)
        self.poll_progress();
        
        // Request repaint for smooth animation (60 FPS)
        ctx.request_repaint();
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                // Banner at top (or fallback to text title)
                if let Some(banner) = &self.banner {
                    // Calculate aspect ratio for responsive sizing
                    let banner_aspect = banner.size()[1] as f32 / banner.size()[0] as f32;
                    let banner_width = ui.available_width();
                    let banner_height = banner_aspect * banner_width;
                    let banner_size = egui::vec2(banner_width, banner_height);
                    
                    ui.add(egui::Image::new((banner.id(), banner_size)));
                } else {
                    // Fallback if banner load failed
                    ui.add_space(20.0);
                    ui.heading(egui::RichText::new("KODEGEN.ᴀɪ")
                        .size(32.0)
                        .color(egui::Color32::from_rgb(24, 202, 155)));  // Cyan
                }
                
                ui.add_space(30.0);
                
                // Progress section (state-based routing)
                if !self.is_complete && !self.is_error {
                    self.show_progress_panel(ui);
                } else if self.is_error {
                    self.show_error_panel(ui);
                } else {
                    self.show_completion_panel(ui);
                }
            });
        });
    }
}

impl InstallWindow {
    /// Show progress panel during installation
    fn show_progress_panel(&self, ui: &mut egui::Ui) {
        // Current step title (e.g., "Creating Directories", "Downloading Chromium")
        ui.label(egui::RichText::new(&self.current_step)
            .size(18.0)
            .strong()
            .color(egui::Color32::from_rgb(24, 202, 155)));  // Cyan accent
        
        ui.add_space(15.0);
        
        // Progress bar with percentage display
        let progress_bar = egui::ProgressBar::new(self.progress)
            .desired_width(500.0)
            .show_percentage()      // Shows "X%" inside bar
            .animate(true);         // Smooth fill animation
        
        ui.add(progress_bar);
        
        ui.add_space(10.0);
        
        // Status message (e.g., "Created installation directories", "Installing service...")
        ui.label(egui::RichText::new(&self.current_message)
            .size(14.0)
            .color(egui::Color32::from_rgb(204, 204, 204)));  // Light gray
        
        ui.add_space(20.0);
        
        // Special context for Chromium download (longest step, 65-85% progress)
        // Provides user reassurance during long download
        if self.progress >= 0.60 && self.progress < 0.85 {
            ui.label(egui::RichText::new("⏳ Downloading Chromium (~100MB)")
                .size(12.0)
                .color(egui::Color32::from_rgb(153, 153, 153)));  // Dim gray
            ui.label(egui::RichText::new("This may take 30-60 seconds")
                .size(11.0)
                .color(egui::Color32::from_rgb(153, 153, 153)));
        }
    }
    
    /// Show completion panel when installation succeeds
    fn show_completion_panel(&self, ui: &mut egui::Ui) {
        // Success icon (large, prominent)
        ui.label(egui::RichText::new("✓")
            .size(64.0)
            .color(egui::Color32::from_rgb(0, 255, 100)));  // Success green
        
        ui.add_space(10.0);
        
        // Success title
        ui.label(egui::RichText::new("Installation Complete!")
            .size(24.0)
            .strong()
            .color(egui::Color32::from_rgb(0, 255, 100)));
        
        ui.add_space(20.0);
        
        // Instructions (what user should do next)
        ui.label(egui::RichText::new("Kodegen daemon has been successfully installed.")
            .size(14.0)
            .color(egui::Color32::from_rgb(204, 204, 204)));
        
        ui.add_space(10.0);
        
        ui.label(egui::RichText::new("Please restart your MCP client to activate:")
            .size(14.0)
            .color(egui::Color32::from_rgb(204, 204, 204)));
        
        ui.add_space(10.0);
        
        // Client list (supported MCP clients)
        ui.horizontal(|ui| {
            ui.add_space(100.0);  // Center offset
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("• Claude Desktop")
                    .size(14.0)
                    .color(egui::Color32::WHITE));
                ui.label(egui::RichText::new("• Cursor")
                    .size(14.0)
                    .color(egui::Color32::WHITE));
                ui.label(egui::RichText::new("• Windsurf")
                    .size(14.0)
                    .color(egui::Color32::WHITE));
                ui.label(egui::RichText::new("• Zed")
                    .size(14.0)
                    .color(egui::Color32::WHITE));
            });
        });
        
        ui.add_space(30.0);
        
        // Close button (exits with success code)
        let close_button = egui::Button::new(
            egui::RichText::new("Close").size(16.0)
        ).fill(egui::Color32::from_rgb(24, 202, 155));  // Cyan button
        
        if ui.add(close_button).clicked() {
            std::process::exit(0);  // Success exit code
        }
    }
    
    /// Show error panel when installation fails
    fn show_error_panel(&self, ui: &mut egui::Ui) {
        // Error icon (large, prominent)
        ui.label(egui::RichText::new("❌")
            .size(64.0)
            .color(egui::Color32::from_rgb(255, 100, 100)));  // Error red
        
        ui.add_space(10.0);
        
        // Error title
        ui.label(egui::RichText::new("Installation Failed")
            .size(24.0)
            .strong()
            .color(egui::Color32::from_rgb(255, 100, 100)));
        
        ui.add_space(20.0);
        
        // Error details (from current_message set by InstallProgress::error())
        ui.label(egui::RichText::new(&self.current_message)
            .size(14.0)
            .color(egui::Color32::from_rgb(204, 204, 204)));
        
        ui.add_space(30.0);
        
        // Action buttons (horizontal layout)
        ui.horizontal(|ui| {
            // Report Issue button (opens GitHub in browser)
            let report_button = egui::Button::new(
                egui::RichText::new("Report Issue").size(14.0)
            ).fill(egui::Color32::from_rgb(24, 202, 155));  // Cyan (action button)
            
            if ui.add(report_button).clicked() {
                // Opens GitHub new issue page in default browser
                // opener crate handles cross-platform (macOS/Windows/Linux)
                let _ = opener::open("https://github.com/cyrup-ai/kodegen/issues/new");
            }
            
            ui.add_space(10.0);
            
            // Close button (exits with error code)
            let close_button = egui::Button::new(
                egui::RichText::new("Close").size(14.0)
            ).fill(egui::Color32::from_rgb(255, 100, 100));  // Red (destructive action)
            
            if ui.add(close_button).clicked() {
                std::process::exit(1);  // Error exit code
            }
        });
    }
}