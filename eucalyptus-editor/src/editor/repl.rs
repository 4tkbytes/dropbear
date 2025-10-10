use egui::{Color32, FontId, RichText, ScrollArea, TextEdit};

/// A simple Kotlin REPL interface for testing scripts
pub struct KotlinREPL {
    input: String,
    output: Vec<ReplOutputLine>,
    history: Vec<String>,
    history_index: Option<usize>,
}

#[derive(Clone)]
pub struct ReplOutputLine {
    pub text: String,
    pub is_error: bool,
    pub is_input: bool,
}

impl Default for KotlinREPL {
    fn default() -> Self {
        Self::new()
    }
}

impl KotlinREPL {
    pub fn new() -> Self {
        let mut repl = Self {
            input: String::new(),
            output: Vec::new(),
            history: Vec::new(),
            history_index: None,
        };

        repl.add_output("Kotlin REPL - Ready", false, false);
        repl.add_output(
            "Type Kotlin expressions to test script functionality",
            false,
            false,
        );
        repl.add_output("Example: Input.isKeyPressed(KeyCode.W)", false, false);
        repl.add_output("", false, false);

        repl
    }

    fn add_output(&mut self, text: &str, is_error: bool, is_input: bool) {
        self.output.push(ReplOutputLine {
            text: text.to_string(),
            is_error,
            is_input,
        });
    }

    fn execute(&mut self, code: &str) {
        // Add input to output
        self.add_output(&format!("> {}", code), false, true);

        // Add to history
        if !code.trim().is_empty() {
            self.history.push(code.to_string());
            self.history_index = None;
        }

        // Execute the code
        match self.execute_kotlin_code(code) {
            Ok(result) => {
                if !result.is_empty() {
                    self.add_output(&result, false, false);
                }
            }
            Err(e) => {
                self.add_output(&format!("Error: {}", e), true, false);
            }
        }

        self.add_output("", false, false); // blank line
    }

    fn execute_kotlin_code(&self, code: &str) -> anyhow::Result<String> {
        // For now, we'll provide a simplified evaluation
        // In the future, this could compile and run actual Kotlin code via JNI

        // Check for common test commands
        if code.trim().starts_with("Input.isKeyPressed") {
            Ok("boolean (check console for actual value)".to_string())
        } else if code.trim().starts_with("Input.getMouseX")
            || code.trim().starts_with("Input.getMouseY")
        {
            Ok("double (check console for actual value)".to_string())
        } else if code.contains("Transform") {
            Ok("Transform manipulation (check entity in viewport)".to_string())
        } else if code.trim() == "help" {
            Ok(r#"Available APIs:
- Input.isKeyPressed(KeyCode.W) - Check if key is pressed
- Input.getMouseX() - Get mouse X position
- Input.getMouseY() - Get mouse Y position
- engine.getTransform() - Get current entity transform
- transform.position - Get/set position (Vector3D)
- transform.rotation - Get/set rotation (Quaternion)
- transform.scale - Get/set scale (Vector3D)

Note: This is a simplified REPL. Full script execution requires 
attaching a script to an entity and running the game."#
                .to_string())
        } else if code.trim() == "clear" {
            Err(anyhow::anyhow!("CLEAR_SCREEN"))
        } else {
            Ok(format!(
                "Command received: {}\n(Full Kotlin evaluation not yet implemented)",
                code
            ))
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // Title bar
            ui.horizontal(|ui| {
                ui.heading("Kotlin REPL");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Clear").clicked() {
                        self.output.clear();
                        self.add_output("Kotlin REPL - Ready", false, false);
                    }
                    if ui.button("Help").clicked() {
                        self.execute("help");
                    }
                });
            });
            
            ui.separator();
            
            // Output area
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.set_min_height(ui.available_height() - 80.0);
                    
                    for line in &self.output {
                        let color = if line.is_error {
                            Color32::from_rgb(255, 100, 100)
                        } else if line.is_input {
                            Color32::from_rgb(100, 200, 255)
                        } else {
                            Color32::from_rgb(200, 200, 200)
                        };
                        
                        ui.label(
                            RichText::new(&line.text)
                                .color(color)
                                .font(FontId::monospace(14.0))
                        );
                    }
                });
            
            ui.separator();
            
            // Input area
            ui.horizontal(|ui| {
                ui.label(">>>");
                
                let response = ui.add(
                    TextEdit::singleline(&mut self.input)
                        .desired_width(f32::INFINITY)
                        .font(FontId::monospace(14.0))
                );
                
                // Handle keyboard input
                if response.has_focus() {
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let code = self.input.clone();
                        self.input.clear();
                        
                        if code.trim() == "clear" {
                            self.output.clear();
                            self.add_output("Kotlin REPL - Ready", false, false);
                        } else {
                            self.execute(&code);
                        }
                    }
                    
                    // History navigation
                    if ui.input(|i| i.key_pressed(egui::Key::ArrowUp))
                        && !self.history.is_empty() {
                            if let Some(idx) = self.history_index {
                                if idx > 0 {
                                    self.history_index = Some(idx - 1);
                                    self.input = self.history[idx - 1].clone();
                                }
                            } else {
                                self.history_index = Some(self.history.len() - 1);
                                self.input = self.history[self.history.len() - 1].clone();
                            }
                        }
                    
                    if ui.input(|i| i.key_pressed(egui::Key::ArrowDown))
                        && let Some(idx) = self.history_index {
                            if idx < self.history.len() - 1 {
                                self.history_index = Some(idx + 1);
                                self.input = self.history[idx + 1].clone();
                            } else {
                                self.history_index = None;
                                self.input.clear();
                            }
                        }
                }
                
                if ui.button("Execute").clicked() && !self.input.is_empty() {
                    let code = self.input.clone();
                    self.input.clear();
                    self.execute(&code);
                }
            });
            
            // Tips
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Tip: Type 'help' for available commands, 'clear' to clear output. Use ↑↓ for history.")
                        .size(10.0)
                        .color(Color32::from_rgb(150, 150, 150))
                );
            });
        });
    }
}
