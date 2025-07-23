use std::{collections::HashSet, path::PathBuf, str::FromStr, sync::Arc};

use dropbear_engine::{
    async_trait::async_trait,
    camera::Camera,
    egui,
    graphics::{Graphics, Shader},
    input::{Controller, Keyboard, Mouse},
    log,
    scene::{Scene, SceneCommand},
    wgpu::{Color, Extent3d, RenderPipeline},
    winit::{
        dpi::PhysicalPosition, event_loop::ActiveEventLoop, keyboard::KeyCode, window::Window,
    },
};
use egui_dock::{DockArea, DockState, NodeIndex, Style, TabViewer};
use egui_toast::{ToastOptions, Toasts};
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
            toasts: egui_toast::Toasts::new()
                .anchor(egui::Align2::RIGHT_BOTTOM, (-10.0, -10.0))
                .direction(egui::Direction::BottomUp),
        }
    }

    pub fn show_ui(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    ui.label("New");
                    ui.label("Open");
                    if ui.button("Save").clicked() {
                        let project_path = {
                            let config = PROJECT.read().unwrap();
                            config.project_path.clone()
                        };
                        let mut config = PROJECT.write().unwrap();
                        match config.write_to(&PathBuf::from_str(&project_path).unwrap()) {
                            Ok(_) => {}
                            Err(e) => {
                                log::error!("Error saving project: {}", e);
                                self.toasts.add(egui_toast::Toast {
                                    kind: egui_toast::ToastKind::Error,
                                    text: format!("Error saving project: {}", e).into(),
                                    options: ToastOptions::default()
                                        .duration_in_seconds(5.0)
                                        .show_progress(true),
                                    ..Default::default()
                                });
                            }
                        }
                        log::info!("Successfully saved project");
                        self.toasts.add(egui_toast::Toast {
                            kind: egui_toast::ToastKind::Success,
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
                        let project_path = {
                            let config = PROJECT.read().unwrap();
                            config.project_path.clone()
                        };
                        let mut config = PROJECT.write().unwrap();
                        match config.write_to(&PathBuf::from_str(&project_path).unwrap()) {
                            Ok(_) => {}
                            Err(e) => {
                                log::error!("Error saving project: {}", e);
                                self.toasts.add(egui_toast::Toast {
                                    kind: egui_toast::ToastKind::Error,
                                    text: format!("Error saving project: {}", e).into(),
                                    options: ToastOptions::default()
                                        .duration_in_seconds(5.0)
                                        .show_progress(true),
                                    ..Default::default()
                                });
                            }
                        }
                        log::info!("Successfully saved project");
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
                ui.label("Asset Viewer");
            }
            EditorTab::ResourceInspector => {
                ui.label("Resource Inspector");
            }
        }
    }
}

#[async_trait]
impl Scene for Editor {
    async fn load(&mut self, graphics: &mut Graphics) {
        let shader = Shader::new(
            graphics,
            include_str!("../../dropbear-engine/resources/shaders/shader.wgsl"),
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

    async fn update(&mut self, _dt: f32, _graphics: &mut Graphics) {
        if let Some((_, tab)) = self.dock_state.find_active_focused() {
            self.is_viewport_focused = matches!(tab, EditorTab::Viewport);
        } else {
            self.is_viewport_focused = false;
        }

        if self.is_viewport_focused {
            self.is_cursor_locked = true;
        }

        if !self.is_cursor_locked {
            self.window.as_mut().unwrap().set_cursor_visible(true);
        }

        self.toasts = egui_toast::Toasts::new()
            .anchor(egui::Align2::RIGHT_BOTTOM, (-10.0, -10.0))
            .direction(egui::Direction::BottomUp);
    }

    async fn render(&mut self, graphics: &mut Graphics) {
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

    async fn exit(&mut self, _event_loop: &ActiveEventLoop) {}

    fn run_command(&mut self) -> SceneCommand {
        std::mem::replace(&mut self.scene_command, SceneCommand::None)
    }
}

impl Keyboard for Editor {
    fn key_down(
        &mut self,
        key: dropbear_engine::winit::keyboard::KeyCode,
        event_loop: &dropbear_engine::winit::event_loop::ActiveEventLoop,
    ) {
        match key {
            KeyCode::Escape => event_loop.exit(),
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

    fn left_stick_changed(&mut self, _x: f32, _y: f32, _id: dropbear_engine::gilrs::GamepadId) {}

    fn right_stick_changed(&mut self, _x: f32, _y: f32, _id: dropbear_engine::gilrs::GamepadId) {}

    fn on_connect(&mut self, _id: dropbear_engine::gilrs::GamepadId) {}

    fn on_disconnect(&mut self, _id: dropbear_engine::gilrs::GamepadId) {}
}
