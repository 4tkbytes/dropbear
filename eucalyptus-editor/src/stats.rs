use std::{collections::VecDeque, time::Instant};

use egui::{Context, RichText};

/// Statistics for checking performance of the editor. 
pub struct NerdStats {
    show_window: bool,

    fps_history: VecDeque<[f64; 2]>,
    frame_times: VecDeque<f64>,
    last_fps_update: Instant,
    current_fps: f32,
    
    frame_time_history: VecDeque<[f64; 2]>,
    
    memory_history: VecDeque<[f64; 2]>,
    
    start_time: Instant,
    total_frames: u64,
    
    min_fps: f32,
    max_fps: f32,
    avg_fps: f32,
}

impl Default for NerdStats {
    fn default() -> Self {
        Self {
            fps_history: VecDeque::with_capacity(300),
            frame_times: VecDeque::with_capacity(60),
            last_fps_update: Instant::now(),
            current_fps: 0.0,
            frame_time_history: VecDeque::with_capacity(300),
            memory_history: VecDeque::with_capacity(300),
            start_time: Instant::now(),
            total_frames: 0,
            min_fps: f32::MAX,
            max_fps: 0.0,
            avg_fps: 0.0,
            show_window: false,
        }
    }
}

impl NerdStats {
    /// Updates all information in [`NerdStats`] with the deltatime provided by the scene
    pub fn update(&mut self, dt: f32) {
        self.total_frames += 1;
        let elapsed = self.start_time.elapsed().as_secs_f64();
        
        let frame_time_ms = (dt * 1000.0) as f64;
        self.frame_times.push_back(dt as f64);
        if self.frame_times.len() > 60 {
            self.frame_times.pop_front();
        }
        
        if self.last_fps_update.elapsed().as_secs_f32() >= 0.1 {
            if dt > 0.0 {
                self.current_fps = 1.0 / dt;
                
                self.min_fps = self.min_fps.min(self.current_fps);
                self.max_fps = self.max_fps.max(self.current_fps);
                
                if !self.frame_times.is_empty() {
                    let avg_frame_time: f64 = self.frame_times.iter().sum::<f64>() / self.frame_times.len() as f64;
                    self.avg_fps = (1.0 / avg_frame_time) as f32;
                }
                
                self.fps_history.push_back([elapsed, self.current_fps as f64]);
                if self.fps_history.len() > 300 {
                    self.fps_history.pop_front();
                }
                
                self.frame_time_history.push_back([elapsed, frame_time_ms]);
                if self.frame_time_history.len() > 300 {
                    self.frame_time_history.pop_front();
                }
                
                self.last_fps_update = Instant::now();
            }
        }
        
        if self.total_frames % 30 == 0 {
            let memory_mb = if let Some(usage) = memory_stats::memory_stats() {
                (usage.physical_mem / 1024 / 1024) as f64
            } else {
                0.0
            };
            
            self.memory_history.push_back([elapsed, memory_mb]);
            if self.memory_history.len() > 300 {
                self.memory_history.pop_front();
            }
        }
    }

    /// Resets statistics to their defaults
    pub fn reset_stats(&mut self) {
        self.min_fps = self.current_fps;
        self.max_fps = self.current_fps;
        self.fps_history.clear();
        self.frame_time_history.clear();
        self.memory_history.clear();
        self.start_time = Instant::now();
        self.total_frames = 0;
    }

    /// Shows the egui window
    pub fn show(&mut self, ctx: &Context) {
        egui::Window::new("Nerdy Stuff 🤓")
            .resizable(true)
            .collapsible(false)
            .default_size([600.0, 500.0])
            .open(show_nerdy_stuff)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Performance Monitor");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Reset Stats").clicked() {
                                stats.reset_stats();
                            }
                        });
                    });
                    
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Current FPS").strong());
                            let fps_color = if stats.current_fps >= 60.0 {
                                Color32::GREEN
                            } else if stats.current_fps >= 30.0 {
                                Color32::YELLOW
                            } else {
                                Color32::RED
                            };
                            ui.label(
                                RichText::new(format!("{:.1}", stats.current_fps))
                                    .size(24.0)
                                    .color(fps_color)
                            );
                        });
                        
                        ui.separator();
                        
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Frame Time").strong());
                            ui.label(
                                RichText::new(format!("{:.2} ms", 1000.0 / stats.current_fps.max(1.0)))
                                    .size(24.0)
                            );
                        });
                        
                        ui.separator();
                        
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Avg FPS").strong());
                            ui.label(
                                RichText::new(format!("{:.1}", stats.avg_fps))
                                    .size(24.0)
                            );
                        });
                    });
                    
                    ui.add_space(5.0);
                    
                    ui.horizontal(|ui| {
                        ui.label(format!("Min: {:.1} fps", stats.min_fps));
                        ui.separator();
                        ui.label(format!("Max: {:.1} fps", stats.max_fps));
                        ui.separator();
                        ui.label(format!("Total Frames: {}", stats.total_frames));
                        ui.separator();
                        ui.label(format!("Uptime: {:.1}s", stats.start_time.elapsed().as_secs_f32()));
                    });
                    
                    ui.separator();
                    
                    ui.label(RichText::new("FPS Over Time").strong());
                    Plot::new("fps_plot")
                        .height(150.0)
                        .show_axes([false, true])
                        .show_grid([false, true])
                        .legend(Legend::default())
                        .show(ui, |plot_ui| {
                            if !stats.fps_history.is_empty() {
                                let points: Vec<[f64; 2]> = stats.fps_history.iter().cloned().collect();
                                plot_ui.line(
                                    Line::new(PlotPoints::from(points))
                                        .color(Color32::from_rgb(100, 200, 100))
                                        .name("FPS")
                                );
                                
                                if let Some(first) = stats.fps_history.front() {
                                    if let Some(last) = stats.fps_history.back() {
                                        plot_ui.line(
                                            Line::new(PlotPoints::from(vec![
                                                [first[0], 60.0],
                                                [last[0], 60.0]
                                            ]))
                                            .color(Color32::from_rgba_unmultiplied(255, 255, 0, 100))
                                            .style(egui_plot::LineStyle::Dashed { length: 5.0 })
                                            .name("60 FPS Target")
                                        );
                                    }
                                }
                            }
                        });
                    
                    ui.add_space(5.0);
                    
                    ui.label(RichText::new("Frame Time").strong());
                    Plot::new("frame_time_plot")
                        .height(150.0)
                        .show_axes([false, true])
                        .show_grid([false, true])
                        .legend(Legend::default())
                        .show(ui, |plot_ui| {
                            if !stats.frame_time_history.is_empty() {
                                let points: Vec<[f64; 2]> = stats.frame_time_history.iter().cloned().collect();
                                plot_ui.line(
                                    Line::new(PlotPoints::from(points))
                                        .color(Color32::from_rgb(100, 150, 255))
                                        .name("Frame Time (ms)")
                                );
                                
                                if let Some(first) = stats.frame_time_history.front() {
                                    if let Some(last) = stats.frame_time_history.back() {
                                        plot_ui.line(
                                            Line::new(PlotPoints::from(vec![
                                                [first[0], 16.67],
                                                [last[0], 16.67]
                                            ]))
                                            .color(Color32::from_rgba_unmultiplied(255, 255, 0, 100))
                                            .style(egui_plot::LineStyle::Dashed { length: 5.0 })
                                            .name("16.67ms (60 FPS)")
                                        );
                                    }
                                }
                            }
                        });
                    
                    ui.add_space(5.0);
                    
                    ui.label(RichText::new("Memory Usage").strong());
                    Plot::new("memory_plot")
                        .height(120.0)
                        .show_axes([false, true])
                        .show_grid([false, true])
                        .legend(Legend::default())
                        .show(ui, |plot_ui| {
                            if !stats.memory_history.is_empty() {
                                let points: Vec<[f64; 2]> = stats.memory_history.iter().cloned().collect();
                                plot_ui.line(
                                    Line::new(PlotPoints::from(points))
                                        .color(Color32::from_rgb(255, 150, 100))
                                        .name("Memory (MB)")
                                );
                            }
                        });
                    
                    ui.separator();
                    ui.collapsing("System Information", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("egui version:");
                            ui.label(egui::VERSION);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Backend:");
                            ui.label("wgpu");
                        });
                        ui.horizontal(|ui| {
                            ui.label("OS:");
                            ui.label(std::env::consts::OS);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Architecture:");
                            ui.label(std::env::consts::ARCH);
                        });
                    });
                });
            });
    }
}