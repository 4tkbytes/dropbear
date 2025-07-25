use std::{collections::HashSet, sync::Arc};

use dropbear_engine::{
    camera::Camera,
    egui, egui_extras,
    graphics::{Graphics, NO_TEXTURE, Shader},
    input::{Controller, Keyboard, Mouse},
    log,
    scene::{Scene, SceneCommand},
    wgpu::{Color, Extent3d, RenderPipeline},
    winit::{
        dpi::PhysicalPosition, event_loop::ActiveEventLoop, keyboard::KeyCode, window::Window,
    },
};
use egui_dock_fork::{DockArea, DockState, NodeIndex, Style, TabViewer};
use egui_toast_fork::{ToastOptions, Toasts};
use serde::{Deserialize, Serialize};

use crate::states::PROJECT;

pub struct Editor {
    scene_command: SceneCommand,
    dock_state: DockState<EditorTab>,
    texture_id: Option<egui::TextureId>,
    size: Extent3d,
    render_pipeline: Option<RenderPipeline>,
    camera: Camera,
    color: Color,
    toasts: Toasts,

    is_viewport_focused: bool,
    pressed_keys: HashSet<KeyCode>,
    is_cursor_locked: bool,

    window: Option<Arc<Window>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EditorTab {
    AssetViewer,       // bottom side,
    ResourceInspector, // left side,
    ModelEntityList,   // right side,
    Viewport,          // middle,
}

impl Editor {
    pub fn new() -> Self {
        let tabs = vec![EditorTab::Viewport];
        let mut dock_state = DockState::new(tabs);

        let surface = dock_state.main_surface_mut();
        let [_old, right] =
            surface.split_right(NodeIndex::root(), 0.25, vec![EditorTab::ModelEntityList]);
        let [_old, _] =
            surface.split_left(NodeIndex::root(), 0.20, vec![EditorTab::ResourceInspector]);
        let [_old, _] = surface.split_below(right, 0.5, vec![EditorTab::AssetViewer]);

        Self {
            scene_command: SceneCommand::None,
            dock_state,
            texture_id: None,
            size: Extent3d::default(),
            render_pipeline: None,
            camera: Camera::default(),
            color: Color::default(),
            is_viewport_focused: false,
            pressed_keys: HashSet::new(),
            is_cursor_locked: false,
            window: None,
            toasts: egui_toast_fork::Toasts::new()
                .anchor(egui::Align2::RIGHT_BOTTOM, (-10.0, -10.0))
                .direction(egui::Direction::BottomUp),
        }
    }

    pub fn save_project_config(&self) -> anyhow::Result<()> {
        let mut config = PROJECT.write().unwrap();
        config.dock_layout = Some(self.dock_state.clone());
        // let project_path = config.project_path.clone();
        // config.write_to(&PathBuf::from(project_path))
        config.write_to_all()
    }

    pub fn load_project_config(&mut self) -> anyhow::Result<()> {
        let config = PROJECT.read().unwrap();
        if let Some(layout) = &config.dock_layout {
            self.dock_state = layout.clone();
        }
        Ok(())
    }

    pub fn show_ui(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    ui.label("New");
                    ui.label("Open");
                    if ui.button("Save").clicked() {
                        match self.save_project_config() {
                            Ok(_) => {}
                            Err(e) => {
                                log::error!("Error saving project: {}", e);
                                self.toasts.add(egui_toast_fork::Toast {
                                    kind: egui_toast_fork::ToastKind::Error,
                                    text: format!("Error saving project: {}", e).into(),
                                    options: ToastOptions::default()
                                        .duration_in_seconds(5.0)
                                        .show_progress(true),
                                    ..Default::default()
                                });
                            }
                        }
                        log::info!("Successfully saved project");
                        self.toasts.add(egui_toast_fork::Toast {
                            kind: egui_toast_fork::ToastKind::Success,
                            text: format!("Successfully saved project").into(),
                            options: ToastOptions::default()
                                .duration_in_seconds(5.0)
                                .show_progress(true),
                            ..Default::default()
                        });
                    }
                    ui.menu_button("Settings", |ui| {
                        let project_name = {
                            let config = PROJECT.read().unwrap();
                            config.project_name.clone()
                        };
                        ui.label(format!("{} config", project_name));
                        ui.label("Eucalyptus Editor");
                    });
                    if ui.button("Quit").clicked() {
                        match self.save_project_config() {
                            Ok(_) => {}
                            Err(e) => {
                                log::error!("Error saving project: {}", e);
                                self.toasts.add(egui_toast_fork::Toast {
                                    kind: egui_toast_fork::ToastKind::Error,
                                    text: format!("Error saving project: {}", e).into(),
                                    options: ToastOptions::default()
                                        .duration_in_seconds(5.0)
                                        .show_progress(true),
                                    ..Default::default()
                                });
                            }
                        }
                        log::info!("Successfully saved project");
                        self.toasts.add(egui_toast_fork::Toast {
                            kind: egui_toast_fork::ToastKind::Success,
                            text: format!("Successfully saved project").into(),
                            options: ToastOptions::default()
                                .duration_in_seconds(5.0)
                                .show_progress(true),
                            ..Default::default()
                        });
                        self.scene_command = SceneCommand::Quit;
                    }
                });
                ui.menu_button("Edit", |ui| {
                    ui.label("Undo");
                    ui.label("Redo");
                });
                ui.menu_button("Window", |_ui| {
                    // ui.menu
                })
                // todo: add more stuff and give it purpose this is too bland :(
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            DockArea::new(&mut self.dock_state)
                .style(Style::from_egui(ui.style().as_ref()))
                .show_inside(
                    ui,
                    &mut EditorTabViewer {
                        view: self.texture_id.unwrap(),
                    },
                );
        });

        self.toasts.show(ctx);
    }
}

pub struct EditorTabViewer {
    pub view: egui::TextureId,
}

impl TabViewer for EditorTabViewer {
    type Tab = EditorTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            EditorTab::Viewport => "Viewport".into(),
            EditorTab::ModelEntityList => "Model/Entity List".into(),
            EditorTab::AssetViewer => "Asset Viewer".into(),
            EditorTab::ResourceInspector => "Resource Inspector".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            EditorTab::Viewport => {
                let size = ui.available_size();
                ui.image((self.view, size));
            }
            EditorTab::ModelEntityList => {
                ui.label("Model/Entity List");
            }
            EditorTab::AssetViewer => {
                egui_extras::install_image_loaders(ui.ctx());

                let assets: Vec<(egui::Image, String)> = (0..30)
                    .map(|i| {
                        let image =
                            egui::Image::from_bytes(format!("no texture {}", i), NO_TEXTURE);
                        (image, format!("no texture {}", i))
                    })
                    .collect::<Vec<_>>();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let columns = 6;
                    let available_width = ui.available_width();
                    let min_spacing = 8.0;
                    let max_spacing = 30.0;
                    let label_height = 20.0;
                    let padding = 8.0; // check it out

                    let card_width = ((available_width - max_spacing * (columns as f32 - 1.0))
                        / columns as f32)
                        .max(32.0);
                    let image_size = card_width - label_height;
                    let spacing = ((available_width - columns as f32 * card_width)
                        / (columns as f32 - 1.0))
                        .clamp(min_spacing, max_spacing);
                    let card_height = image_size + label_height + padding;

                    egui::Grid::new("asset_grid")
                        .num_columns(columns)
                        .min_col_width(card_width)
                        .max_col_width(card_width)
                        .spacing([spacing, spacing])
                        .show(ui, |ui| {
                            for (i, (image, asset_name)) in assets.iter().enumerate() {
                                // Allocate the card area and get a response for hover/click
                                let card_size = egui::vec2(card_width, card_height);
                                let (rect, card_response) =
                                    ui.allocate_exact_size(card_size, egui::Sense::click());

                                // Check if either the card or the image is hovered
                                let is_hovered = card_response.hovered();

                                // Paint highlight if hovered
                                if is_hovered {
                                    ui.painter().rect_filled(
                                        rect,
                                        6.0, // corner radius
                                        egui::Color32::from_rgb(60, 60, 80),
                                    );
                                }

                                let mut card_ui = ui.new_child(
                                    egui::UiBuilder::new()
                                        .max_rect(rect)
                                        .layout(egui::Layout::top_down(egui::Align::Center)),
                                );
                                let image_response = card_ui.add_sized(
                                    [image_size, image_size],
                                    egui::ImageButton::new(image.clone()).frame(false),
                                );
                                // Combine hover states
                                let is_hovered = is_hovered || image_response.hovered();

                                // Center the label below the image
                                card_ui.label(
                                    egui::RichText::new(asset_name)
                                        .strong()
                                        .color(egui::Color32::WHITE),
                                );

                                if (i + 1) % columns == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                });
            }
            EditorTab::ResourceInspector => {
                ui.label("Resource Inspector");
            }
        }
    }
}

impl Scene for Editor {
    fn load(&mut self, graphics: &mut Graphics) {
        let _ = self.load_project_config();

        let shader = Shader::new(
            graphics,
            include_str!("shader.wgsl"),
            Some("viewport_shader"),
        );

        let camera = Camera::predetermined(graphics);

        let pipeline = graphics.create_render_pipline(
            &shader,
            vec![&graphics.state.texture_bind_layout, camera.layout()],
        );

        self.camera = camera;
        self.render_pipeline = Some(pipeline);
        self.window = Some(graphics.state.window.clone());
    }

    fn update(&mut self, _dt: f32, _graphics: &mut Graphics) {
        if let Some((_, tab)) = self.dock_state.find_active_focused() {
            self.is_viewport_focused = matches!(tab, EditorTab::Viewport);
        } else {
            self.is_viewport_focused = false;
        }

        if self.is_viewport_focused {
            self.is_cursor_locked = true;
        }

        if self.is_cursor_locked {
            for key in &self.pressed_keys {
                match key {
                    KeyCode::KeyW => self.camera.move_forwards(),
                    KeyCode::KeyA => self.camera.move_left(),
                    KeyCode::KeyD => self.camera.move_right(),
                    KeyCode::KeyS => self.camera.move_back(),
                    KeyCode::ShiftLeft => self.camera.move_down(),
                    KeyCode::Space => self.camera.move_up(),
                    _ => {}
                }
            }
        }

        if !self.is_cursor_locked {
            self.window.as_mut().unwrap().set_cursor_visible(true);
        }

        self.toasts = egui_toast_fork::Toasts::new()
            .anchor(egui::Align2::RIGHT_BOTTOM, (-10.0, -10.0))
            .direction(egui::Direction::BottomUp);
    }

    fn render(&mut self, graphics: &mut Graphics) {
        let color = Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };
        self.color = color.clone();

        let mut pass = graphics.clear_colour(color);
        if let Some(pipeline) = &self.render_pipeline {
            pass.set_pipeline(pipeline);
        }

        self.texture_id = Some(graphics.state.texture_id);
        self.size = graphics.state.viewport_texture.size;
        let ctx = graphics.get_egui_context();
        self.show_ui(ctx);
        self.window = Some(graphics.state.window.clone());
        self.toasts.show(graphics.get_egui_context());
    }

    fn exit(&mut self, _event_loop: &ActiveEventLoop) {}

    fn run_command(&mut self) -> SceneCommand {
        std::mem::replace(&mut self.scene_command, SceneCommand::None)
    }
}

impl Keyboard for Editor {
    fn key_down(
        &mut self,
        key: dropbear_engine::winit::keyboard::KeyCode,
        _event_loop: &dropbear_engine::winit::event_loop::ActiveEventLoop,
    ) {
        match key {
            // KeyCode::Escape => event_loop.exit(),
            KeyCode::F1 => {
                self.is_cursor_locked = !self.is_cursor_locked;
                if !self.is_cursor_locked {
                    if let Some((surface_idx, node_idx, _)) =
                        self.dock_state.find_tab(&EditorTab::AssetViewer)
                    {
                        self.dock_state
                            .set_focused_node_and_surface((surface_idx, node_idx));
                    }
                }
            }
            KeyCode::KeyS => {
                #[cfg(not(target_os = "macos"))]
                let ctrl_pressed = self.pressed_keys.contains(&KeyCode::ControlLeft)
                    || self.pressed_keys.contains(&KeyCode::ControlRight);
                #[cfg(target_os = "macos")]
                let ctrl_pressed = self.pressed_keys.contains(&KeyCode::SuperLeft)
                    || self.pressed_keys.contains(&KeyCode::SuperRight);

                if ctrl_pressed {
                    match self.save_project_config() {
                        Ok(_) => {
                            log::info!("Successfully saved project");
                            self.toasts.add(egui_toast_fork::Toast {
                                kind: egui_toast_fork::ToastKind::Success,
                                text: format!("Successfully saved project").into(),
                                options: egui_toast_fork::ToastOptions::default()
                                    .duration_in_seconds(5.0)
                                    .show_progress(true),
                                ..Default::default()
                            });
                        }
                        Err(e) => {
                            log::error!("Error saving project: {}", e);
                            self.toasts.add(egui_toast_fork::Toast {
                                kind: egui_toast_fork::ToastKind::Error,
                                text: format!("Error saving project: {}", e).into(),
                                options: egui_toast_fork::ToastOptions::default()
                                    .duration_in_seconds(5.0)
                                    .show_progress(true),
                                ..Default::default()
                            });
                        }
                    }
                } else {
                    self.pressed_keys.insert(key);
                }
            }
            _ => {
                self.pressed_keys.insert(key);
            }
        }
    }

    fn key_up(
        &mut self,
        key: dropbear_engine::winit::keyboard::KeyCode,
        _event_loop: &dropbear_engine::winit::event_loop::ActiveEventLoop,
    ) {
        self.pressed_keys.remove(&key);
    }
}

impl Mouse for Editor {
    fn mouse_move(&mut self, position: PhysicalPosition<f64>) {
        if self.is_cursor_locked {
            if let Some(window) = &self.window {
                let size = window.inner_size();
                let center =
                    PhysicalPosition::new(size.width as f64 / 2.0, size.height as f64 / 2.0);

                let dx = position.x - center.x;
                let dy = position.y - center.y;
                self.camera.track_mouse_delta(dx as f32, dy as f32);

                window.set_cursor_position(center).ok();
                window.set_cursor_visible(false);
            }
        }
    }

    fn mouse_down(&mut self, _button: dropbear_engine::winit::event::MouseButton) {}

    fn mouse_up(&mut self, _button: dropbear_engine::winit::event::MouseButton) {}
}

impl Controller for Editor {
    fn button_down(
        &mut self,
        _button: dropbear_engine::gilrs::Button,
        _id: dropbear_engine::gilrs::GamepadId,
    ) {
    }

    fn button_up(
        &mut self,
        _button: dropbear_engine::gilrs::Button,
        _id: dropbear_engine::gilrs::GamepadId,
    ) {
    }

    fn left_stick_changed(&mut self, _x: f32, _y: f32, _id: dropbear_engine::gilrs::GamepadId) {
        // used for moving the camera
    }

    fn right_stick_changed(&mut self, _x: f32, _y: f32, _id: dropbear_engine::gilrs::GamepadId) {
        // used for moving the player
    }

    fn on_connect(&mut self, _id: dropbear_engine::gilrs::GamepadId) {}

    fn on_disconnect(&mut self, _id: dropbear_engine::gilrs::GamepadId) {}
}
