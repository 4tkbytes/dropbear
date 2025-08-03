pub mod dock;
pub mod input;
pub mod scene;

pub(crate) use crate::editor::dock::*;

use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{Arc, LazyLock, Mutex},
};

use dropbear_engine::{camera::Camera, scene::SceneCommand};
use egui::{self, Context};
use egui_dock_fork::{DockArea, DockState, NodeIndex, Style};
use egui_toast_fork::{ToastOptions, Toasts};
use hecs::World;
use log;
use once_cell::sync::Lazy;
use transform_gizmo_egui::Gizmo;
use wgpu::{Color, Extent3d, RenderPipeline};
use winit::{keyboard::KeyCode, window::Window};

use crate::states::{EntityNode, PROJECT};

pub static GLOBAL_TOASTS: Lazy<Mutex<Toasts>> = Lazy::new(|| {
    Mutex::new(
        Toasts::new()
            .anchor(egui::Align2::RIGHT_BOTTOM, (-10.0, -10.0))
            .direction(egui::Direction::BottomUp),
    )
});

pub struct Editor {
    scene_command: SceneCommand,
    world: hecs::World,
    dock_state: DockState<EditorTab>,
    texture_id: Option<egui::TextureId>,
    size: Extent3d,
    render_pipeline: Option<RenderPipeline>,
    camera: Camera,
    color: Color,

    is_viewport_focused: bool,
    pressed_keys: HashSet<KeyCode>,
    // is_cursor_locked: bool,
    window: Option<Arc<Window>>,

    show_new_project: bool,
    project_name: String,
    project_path: Option<PathBuf>,
    pending_scene_switch: bool,

    gizmo: Gizmo,
    selected_entity: Option<hecs::Entity>,

    resize_signal: (bool, u32, u32),
}

impl Default for Editor {
    fn default() -> Self {
        Editor::new()
    }
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
            // is_cursor_locked: false,
            window: None,
            world: World::new(),
            show_new_project: false,
            project_name: String::new(),
            project_path: None,
            pending_scene_switch: false,
            gizmo: Gizmo::default(),
            resize_signal: (false, 1, 1),
            selected_entity: None,
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

    pub fn show_ui(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui
                        .button("Main Menu (New + Open + Editor Settings)")
                        .clicked()
                    {
                        self.scene_command = SceneCommand::SwitchScene("main_menu".into());
                    }

                    if ui.button("Save").clicked() {
                        match self.save_project_config() {
                            Ok(_) => {}
                            Err(e) => {
                                log::error!("Error saving project: {}", e);
                                if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                                    toasts.add(egui_toast_fork::Toast {
                                        kind: egui_toast_fork::ToastKind::Error,
                                        text: format!("Error saving project: {}", e).into(),
                                        options: ToastOptions::default()
                                            .duration_in_seconds(5.0)
                                            .show_progress(true),
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                        log::info!("Successfully saved project");
                        if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                            toasts.add(egui_toast_fork::Toast {
                                kind: egui_toast_fork::ToastKind::Success,
                                text: format!("Successfully saved project").into(),
                                options: ToastOptions::default()
                                    .duration_in_seconds(5.0)
                                    .show_progress(true),
                                ..Default::default()
                            });
                        }
                    }
                    if ui.button("Project Settings").clicked() {};
                    if ui.button("Quit").clicked() {
                        match self.save_project_config() {
                            Ok(_) => {}
                            Err(e) => {
                                log::error!("Error saving project: {}", e);
                                if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                                    toasts.add(egui_toast_fork::Toast {
                                        kind: egui_toast_fork::ToastKind::Error,
                                        text: format!("Error saving project: {}", e).into(),
                                        options: ToastOptions::default()
                                            .duration_in_seconds(5.0)
                                            .show_progress(true),
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                        log::info!("Successfully saved project");
                        if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                            toasts.add(egui_toast_fork::Toast {
                                kind: egui_toast_fork::ToastKind::Success,
                                text: format!("Successfully saved project").into(),
                                options: ToastOptions::default()
                                    .duration_in_seconds(5.0)
                                    .show_progress(true),
                                ..Default::default()
                            });
                            self.scene_command = SceneCommand::Quit;
                        }
                    }
                });
                ui.menu_button("Edit", |ui| {
                    ui.label("Undo");
                    ui.label("Redo");
                });

                ui.menu_button("Window", |ui_window| {
                    if ui_window.button("Open Asset Viewer").clicked() {
                        self.dock_state.push_to_focused_leaf(EditorTab::AssetViewer);
                    }
                    if ui_window.button("Open Resource Inspector").clicked() {
                        self.dock_state
                            .push_to_focused_leaf(EditorTab::ResourceInspector);
                    }
                    if ui_window.button("Open Entity List").clicked() {
                        self.dock_state
                            .push_to_focused_leaf(EditorTab::ModelEntityList);
                    }
                    if ui_window.button("Open Viewport").clicked() {
                        self.dock_state.push_to_focused_leaf(EditorTab::Viewport);
                    }
                });
                // todo: add more stuff and give it purpose this is too bland :(
            });
        });

        egui::CentralPanel::default().show(&ctx, |ui| {
            DockArea::new(&mut self.dock_state)
                .style(Style::from_egui(ui.style().as_ref()))
                .show_inside(
                    ui,
                    &mut EditorTabViewer {
                        view: self.texture_id.unwrap(),
                        nodes: EntityNode::from_world(&self.world),
                        gizmo: &mut self.gizmo,
                        tex_size: self.size,
                        camera: &mut self.camera,
                        resize_signal: &mut self.resize_signal,
                        world: &mut self.world,
                        selected_entity: &mut self.selected_entity,
                    },
                );
        });

        if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
            toasts.show(ctx);
        }

        crate::utils::show_new_project_window(
            ctx,
            &mut self.show_new_project,
            &mut self.project_name,
            &mut self.project_path,
            |name, path| {
                crate::utils::start_project_creation(name.to_string(), Some(path.clone()));
                self.pending_scene_switch = true;
            },
        );

        if self.pending_scene_switch {
            self.scene_command = SceneCommand::SwitchScene("editor".to_string());
            self.pending_scene_switch = false;
        }
    }
}

pub static LOGGED: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

fn show_entity_tree(
    ui: &mut egui::Ui,
    nodes: &mut Vec<EntityNode>,
    selected: &mut Option<hecs::Entity>,
    id_source: &str,
) {
    egui_dnd::Dnd::new(ui, id_source).show(nodes.iter(), |ui, item, handle, _dragging| match item
        .clone()
    {
        EntityNode::Entity { id, name } => {
            ui.horizontal(|ui| {
                handle.ui(ui, |ui| {
                    ui.label("|||");
                });
                let resp = ui.selectable_label(selected.as_ref().eq(&Some(&id)), name);
                if resp.clicked() {
                    *selected = Some(id);
                }
            });
        }
        EntityNode::Script { name, path: _ } => {
            ui.horizontal(|ui| {
                handle.ui(ui, |ui| {
                    ui.label("|||");
                });
                ui.label(format!("{name}"));
            });
        }
        EntityNode::Group {
            ref name,
            ref mut children,
            ref mut collapsed,
        } => {
            ui.horizontal(|ui| {
                handle.ui(ui, |ui| {
                    ui.label("|||");
                });
                let header = egui::CollapsingHeader::new(name)
                    .default_open(!*collapsed)
                    .show(ui, |ui| {
                        show_entity_tree(ui, children, selected, name);
                    });
                *collapsed = !header.body_returned.is_some();
            });
        }
    });
}
