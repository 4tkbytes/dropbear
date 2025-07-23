use std::{
    cell::OnceCell,
    sync::{LazyLock, OnceLock},
};

use dropbear_engine::{
    async_trait::async_trait,
    camera::Camera,
    graphics::{Graphics, Shader},
    input::{Controller, Keyboard, Mouse},
    scene::{Scene, SceneCommand},
    wgpu::{Color, Extent3d, RenderPipeline},
    winit::event_loop::ActiveEventLoop,
};
use egui_dock::{
    DockArea, DockState, NodeIndex, Style, TabViewer,
    egui::{self, CentralPanel, Image, TextureId, TopBottomPanel, Ui, WidgetText},
};

pub struct Editor {
    scene_command: SceneCommand,
    dock_state: DockState<EditorTab>,
    texture_id: Option<TextureId>,
    size: Extent3d,
    render_pipeline: Option<RenderPipeline>,
    camera: Camera,
    color: Color,

    is_viewport_focused: bool,
}

#[derive(Clone, Debug)]
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
        let [_old, bottom] = surface.split_below(right, 0.5, vec![EditorTab::AssetViewer]);
        let [_old, left] =
            surface.split_left(NodeIndex::root(), 0.20, vec![EditorTab::ResourceInspector]);

        Self {
            scene_command: SceneCommand::None,
            dock_state,
            texture_id: None,
            size: Extent3d::default(),
            render_pipeline: None,
            camera: Camera::default(),
            color: Color::default(),
            is_viewport_focused: false,
        }
    }

    pub fn show_ui(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    ui.label("New");
                    ui.label("Open");
                    ui.label("Save");
                });
                ui.menu_button("Edit", |ui| {
                    ui.label("Undo");
                    ui.label("Redo");
                });
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
                        // texture: self.viewport,
                        size: self.size,
                    },
                );

            // todo: render wgpu viewport
        });
    }
}

pub struct EditorTabViewer {
    pub view: TextureId,
    pub size: Extent3d,
}

impl TabViewer for EditorTabViewer {
    type Tab = EditorTab;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        match tab {
            EditorTab::Viewport => "Viewport".into(),
            EditorTab::ModelEntityList => "Model/Entity List".into(),
            EditorTab::AssetViewer => "Asset Viewer".into(),
            EditorTab::ResourceInspector => "Resource Inspector".into(),
        }
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
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
    }
    async fn update(&mut self, dt: f32, graphics: &mut Graphics) {}
    async fn render(&mut self, graphics: &mut Graphics) {
        let color = Color {
            r: if self.color.r < 1.0 {
                self.color.r + 0.1
            } else {
                0.1
            },
            g: if self.color.g < 1.0 {
                self.color.g + 0.1
            } else {
                0.2
            },
            b: if self.color.b < 1.0 {
                self.color.b + 0.1
            } else {
                0.3
            },
            a: 1.0,
        };
        self.color = color.clone();

        let mut pass = graphics.clear_colour(color);
        if let Some(pipeline) = &self.render_pipeline {
            pass.set_pipeline(pipeline);
        }

        self.texture_id = Some(graphics.state.texture_id);
        // self.viewport = Some(graphics.state.viewport_texture);
        self.size = graphics.state.viewport_texture.size;
        let ctx = graphics.get_egui_context();
        self.show_ui(ctx);
    }
    async fn exit(&mut self, event_loop: &ActiveEventLoop) {}

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
    }

    fn key_up(
        &mut self,
        key: dropbear_engine::winit::keyboard::KeyCode,
        event_loop: &dropbear_engine::winit::event_loop::ActiveEventLoop,
    ) {
    }
}

impl Mouse for Editor {
    fn mouse_move(&mut self, position: dropbear_engine::winit::dpi::PhysicalPosition<f64>) {}

    fn mouse_down(&mut self, button: dropbear_engine::winit::event::MouseButton) {}

    fn mouse_up(&mut self, button: dropbear_engine::winit::event::MouseButton) {}
}

impl Controller for Editor {
    fn button_down(
        &mut self,
        button: dropbear_engine::gilrs::Button,
        id: dropbear_engine::gilrs::GamepadId,
    ) {
    }

    fn button_up(
        &mut self,
        button: dropbear_engine::gilrs::Button,
        id: dropbear_engine::gilrs::GamepadId,
    ) {
    }

    fn left_stick_changed(&mut self, x: f32, y: f32, id: dropbear_engine::gilrs::GamepadId) {}

    fn right_stick_changed(&mut self, x: f32, y: f32, id: dropbear_engine::gilrs::GamepadId) {}

    fn on_connect(&mut self, id: dropbear_engine::gilrs::GamepadId) {}

    fn on_disconnect(&mut self, id: dropbear_engine::gilrs::GamepadId) {}
}
