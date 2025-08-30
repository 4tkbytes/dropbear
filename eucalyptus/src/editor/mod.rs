pub mod dock;
pub mod input;
pub mod scene;
mod component;

pub(crate) use crate::editor::dock::*;

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};

use dropbear_engine::{
    camera::Camera, entity::{AdoptedEntity, Transform}, graphics::Graphics, lighting::{Light, LightManager}, scene::SceneCommand
};
use egui::{self, Context};
use egui_dock_fork::{DockArea, DockState, NodeIndex, Style};
use glam::DVec3;
use hecs::{World};
use log;
use parking_lot::Mutex;
use transform_gizmo_egui::{EnumSet, Gizmo, GizmoMode};
use wgpu::{Color, Extent3d, RenderPipeline};
use winit::{keyboard::KeyCode, window::Window};

use crate::{build::build, camera::{
    CameraAction, CameraManager, CameraType, DebugCameraController, PlayerCameraController,
}, debug, scripting::{input::InputState, ScriptAction, ScriptManager}, states::{EntityNode, LightConfig, ModelProperties, SceneEntity, ScriptComponent, PROJECT, SCENES}, utils::ViewportMode};

pub struct Editor {
    scene_command: SceneCommand,
    world: World,
    dock_state: DockState<EditorTab>,
    texture_id: Option<egui::TextureId>,
    size: Extent3d,
    render_pipeline: Option<RenderPipeline>,
    light_manager: LightManager,
    color: Color,

    camera_manager: CameraManager,

    is_viewport_focused: bool,
    // is_cursor_locked: bool,
    window: Option<Arc<Window>>,

    show_new_project: bool,
    project_name: String,
    project_path: Option<PathBuf>,
    pending_scene_switch: bool,

    gizmo: Gizmo,
    selected_entity: Option<hecs::Entity>,
    viewport_mode: ViewportMode,

    signal: Signal,
    undo_stack: Vec<UndoableAction>,
    // todo: add redo (later)
    // redo_stack: Vec<UndoableAction>,

    editor_state: EditorState,
    gizmo_mode: EnumSet<GizmoMode>,

    script_manager: ScriptManager,
    play_mode_backup: Option<PlayModeBackup>,

    input_state: InputState,
}

#[derive(Clone)]
pub struct PlayModeBackup {
    entities: Vec<(hecs::Entity, Transform, ModelProperties, Option<ScriptComponent>)>,
    camera_positions: HashMap<CameraType, (DVec3, DVec3)>, // pos, target
}

impl PlayModeBackup {
    pub fn create_backup(editor: &mut Editor) -> anyhow::Result<()> {
        let mut entities = Vec::new();
        
        for (entity_id, (_, transform, properties)) in editor
            .world
            .query::<(&AdoptedEntity, &Transform, &ModelProperties)>()
            .iter()
        {
            let script = editor.world.query_one::<&ScriptComponent>(entity_id).ok().map(|mut s| {
                if let Some(script) = s.get() {
                    Some(script.clone())
                } else {
                    None
                }
            }).unwrap();
            entities.push((entity_id, *transform, properties.clone(), script));
        }

        let mut camera_positions = HashMap::new();
        
        if let Some(debug_camera) = editor.camera_manager.get_camera(&CameraType::Debug) {
            camera_positions.insert(CameraType::Debug, (debug_camera.eye, debug_camera.target));
        }
        
        if let Some(player_camera) = editor.camera_manager.get_camera(&CameraType::Player) {
            camera_positions.insert(CameraType::Player, (player_camera.eye, player_camera.target));
        }

        editor.play_mode_backup = Some(PlayModeBackup {
            entities,
            camera_positions,
        });

        log::info!("Created play mode backup with {} entities", editor.play_mode_backup.as_ref().unwrap().entities.len());
        Ok(())
    }

    pub fn restore(editor: &mut Editor) -> anyhow::Result<()> {
        if let Some(backup) = &editor.play_mode_backup {
            for (entity_id, original_transform, original_properties, original_script) in &backup.entities {
                if let Ok(mut transform) = editor.world.get::<&mut Transform>(*entity_id) {
                    *transform = *original_transform;
                }
                
                if let Ok(mut properties) = editor.world.get::<&mut ModelProperties>(*entity_id) {
                    *properties = original_properties.clone();
                }
                
                let has_script = editor.world.get::<&ScriptComponent>(*entity_id).is_ok();
                match (has_script, original_script) {
                    (true, Some(original)) => {
                        if let Ok(mut script) = editor.world.get::<&mut ScriptComponent>(*entity_id) {
                            *script = original.clone();
                        }
                    }
                    (true, None) => {
                        let _ = editor.world.remove_one::<ScriptComponent>(*entity_id);
                    }
                    (false, Some(original)) => {
                        let _ = editor.world.insert_one(*entity_id, original.clone());
                    }
                    (false, None) => {
                    }
                }
            }

            for (camera_type, (position, target)) in &backup.camera_positions {
                if let Some(camera) = editor.camera_manager.get_camera_mut(camera_type) {
                    camera.eye = *position;
                    camera.target = *target;
                }
            }

            log::info!("Restored scene from play mode backup");
            
            editor.play_mode_backup = None;
            Ok(())
        } else {
            Err(anyhow::anyhow!("No play mode backup found to restore"))
        }
    }
}

pub enum EditorState {
    Editing,
    Playing,
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

        // this shit doesnt work :(
        // nvm it works (sorta)
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(1));
                let deadlocks = parking_lot::deadlock::check_deadlock();
                if deadlocks.is_empty() {
                    continue;
                }

                log::error!("{} deadlocks detected", deadlocks.len());
                for (i, threads) in deadlocks.iter().enumerate() {
                    log::error!("Deadlock #{}", i);
                    for t in threads {
                        log::error!("Thread Id {:#?}", t.thread_id());
                        log::error!("{:#?}", t.backtrace());
                    }
                }
            }
        });

        Self {
            scene_command: SceneCommand::None,
            dock_state,
            texture_id: None,
            size: Extent3d::default(),
            render_pipeline: None,
            camera_manager: CameraManager::new(),
            color: Color::default(),
            is_viewport_focused: false,
            // is_cursor_locked: false,
            window: None,
            world: World::new(),
            show_new_project: false,
            project_name: String::new(),
            project_path: None,
            pending_scene_switch: false,
            gizmo: Gizmo::default(),
            selected_entity: None,
            viewport_mode: ViewportMode::None,
            signal: Signal::None,
            undo_stack: Vec::new(),
            script_manager: ScriptManager::new(),
            editor_state: EditorState::Editing,
            gizmo_mode: EnumSet::empty(),
            play_mode_backup: None,
            input_state: InputState::new(),
            light_manager: LightManager::new(),
            // ..Default::default()
            // note to self: DO NOT USE ..DEFAULT::DEFAULT(), IT WILL CAUSE OVERFLOW
        }
    }

    fn double_key_pressed(&mut self, key: KeyCode) -> bool {
        let now = Instant::now();

        if let Some(last_time) = self.input_state.last_key_press_times.get(&key) {
            let time_diff = now.duration_since(*last_time);

            if time_diff <= self.input_state.double_press_threshold {
                self.input_state.last_key_press_times.remove(&key);
                return true;
            }
        }

        self.input_state.last_key_press_times.insert(key, now);
        false
    }

    /// Save the current world state to the active scene
    pub fn save_current_scene(&mut self) -> anyhow::Result<()> {
        let mut scenes = SCENES.write().unwrap();

        let scene_index = if scenes.is_empty() {
            return Err(anyhow::anyhow!("No scenes loaded to save"));
        } else {
            0
        };

        let scene = &mut scenes[scene_index];
        scene.entities.clear();
        scene.lights.clear();

        for (id, (adopted, transform, properties, script)) in self
            .world
            .query::<(
                &AdoptedEntity,
                Option<&Transform>,
                &ModelProperties,
                Option<&ScriptComponent>,
            )>()
            .iter()
        {
            let transform = transform.unwrap_or(&Transform::default()).clone();

            let scene_entity = SceneEntity {
                model_path: adopted.model().path.clone(),
                label: adopted.model().label.clone(),
                transform,
                properties: properties.clone(),
                script: script.cloned(),
                entity_id: Some(id),
            };

            scene.entities.push(scene_entity);
            log::debug!("Pushed entity: {}", adopted.label());
        }

        for (id, (light_component, transform, light)) in self
            .world
            .query::<(
                &dropbear_engine::lighting::LightComponent,
                &Transform,
                &Light,
            )>()
            .iter()
        {
            let light_config = LightConfig {
                label: light.label().to_string(),
                transform: *transform,
                light_component: light_component.clone(),
                enabled: light_component.enabled,
                entity_id: Some(id),
            };

            scene.lights.push(light_config);
            log::debug!("Pushed light into lights: {}", light.label())
        }

        scene.save_cameras_from_manager(&self.camera_manager, &mut self.world);

        log::info!(
            "Saved {} entities and camera configs to scene '{}'",
            scene.entities.len(),
            scene.scene_name
        );

        Ok(())
    }

    pub fn save_project_config(&mut self) -> anyhow::Result<()> {
        self.save_current_scene()?;

        {
            let mut config = PROJECT.write().unwrap();
            config.dock_layout = Some(self.dock_state.clone());
        }

        {
            let (scene_clone, project_path) = {
                let scenes = SCENES.read().unwrap();
                let project = PROJECT.read().unwrap();
                (scenes[0].clone(), project.project_path.clone())
            };

            scene_clone.write_to(&project_path)?;

            let mut config = PROJECT.write().unwrap();
            config.write_to_all()?;
        }

        Ok(())
    }

    pub fn load_project_config(&mut self, graphics: &Graphics) -> anyhow::Result<()> {
        let config = PROJECT.read().unwrap();

        self.project_path = Some(config.project_path.clone());

        if let Some(layout) = &config.dock_layout {
            self.dock_state = layout.clone();
        }

        self.camera_manager.clear_cameras();

        {
            let scenes = SCENES.read().unwrap();
            if let Some(first_scene) = scenes.first() {
                first_scene.load_into_world(&mut self.world, graphics)?;

                first_scene.load_cameras_into_manager(
                    &mut self.camera_manager,
                    graphics,
                    &self.world,
                )?;

                log::info!(
                    "Successfully loaded scene with {} entities and camera configs",
                    first_scene.entities.len()
                );
            } else {
                log::info!("No scenes found, creating default cameras and scene");

                let debug_camera = Camera::predetermined(graphics);
                let debug_controller = Box::new(DebugCameraController::new());
                self.camera_manager
                    .add_camera(CameraType::Debug, debug_camera, debug_controller);

                let player_camera = Camera::new(
                    graphics,
                    DVec3::new(0.0, 2.0, 5.0),
                    DVec3::new(0.0, 0.0, 0.0),
                    DVec3::Y,
                    graphics.state.config.width as f64 / graphics.state.config.height as f64,
                    45.0,
                    0.1,
                    100.0,
                    0.1,
                    0.001,
                );
                let player_controller = Box::new(PlayerCameraController::new());
                self.camera_manager.add_camera(
                    CameraType::Player,
                    player_camera,
                    player_controller,
                );
            }
        }

        self.camera_manager.set_active(CameraType::Debug);

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
                                crate::fatal!("Error saving project: {}", e);
                            }
                        }
                        crate::success!("Successfully saved project");
                    }
                    if ui.button("Project Settings").clicked() {};
                    if matches!(self.editor_state, EditorState::Playing) {
                        if ui.button("Stop").clicked() {
                            self.signal = Signal::StopPlaying;
                        }
                    } else {
                        if ui.button("Play").clicked() {
                            self.signal = Signal::Play;
                        }
                    }
                    ui.menu_button("Export", |ui| {
                        // todo: create a window for better build menu
                        if ui.button("Build").clicked() {
                            {
                                if let Ok(proj) = PROJECT.read() {
                                    match build(proj.project_path.join(format!("{}.eucp", proj.project_name.clone())).clone()) {
                                        Ok(thingy) => crate::success!("Project output at {}", thingy.display()),
                                        Err(e) => {
                                            crate::fatal!("Unable to build project [{}]: {}", proj.project_path.clone().display(), e);
                                        },
                                    }
                                }
                            }
                        }
                        ui.label("Package"); // todo: create a window for label
                    });
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        match self.save_project_config() {
                            Ok(_) => {}
                            Err(e) => {
                                crate::fatal!("Error saving project: {}", e);
                            }
                        }
                        crate::success!("Successfully saved project");
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if ui.button("Copy").clicked() {
                        if let Some(entity) = &self.selected_entity {
                            let query = self.world.query_one::<(&AdoptedEntity, &Transform, &ModelProperties)>(*entity);
                            if let Ok(mut q) = query {
                                if let Some((e, t, props)) = q.get() {
                                    let s_entity = crate::states::SceneEntity {
                                        model_path: e.model().path.clone(),
                                        label: e.model().label.clone(),
                                        transform: *t,
                                        properties: props.clone(),
                                        script: None,
                                        entity_id: None,
                                    };
                                    self.signal = Signal::Copy(s_entity);

                                    crate::info!("Copied selected entity!");
                                } else {
                                    crate::warn!("Unable to copy entity: Unable to fetch world entity properties");
                                }
                            } else {
                                crate::warn!("Unable to copy entity: Unable to obtain lock");
                            }
                        } else {
                            crate::warn!("Unable to copy entity: None selected");
                        }
                    }

                    if ui.button("Paste").clicked() {
                        match &self.signal {
                            Signal::Copy(entity) => {
                                self.signal = Signal::Paste(entity.clone());
                            }
                            _ => {
                                crate::warn!("Unable to paste: You haven't selected anything!");
                            }
                        }
                    }

                    if ui.button("Undo").clicked() {
                        self.signal = Signal::Undo;
                    }
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
                if let Ok(cfg) = PROJECT.read() {
                    if cfg.editor_settings.is_debug_menu_shown {
                        debug::show_menu_bar(ui, &mut self.signal);
                    }
                }
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
                        world: &mut self.world,
                        selected_entity: &mut self.selected_entity,
                        viewport_mode: &mut self.viewport_mode,
                        undo_stack: &mut self.undo_stack,
                        signal: &mut self.signal,
                        // engine: &mut self.rhai_engine,
                        camera_manager: &mut self.camera_manager,
                        gizmo_mode: &mut self.gizmo_mode,
                        editor_mode: &mut self.editor_state,
                    },
                );
        });

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

    pub fn switch_to_debug_camera(&mut self) {
        self.camera_manager.set_active(CameraType::Debug);
        crate::info!("Switched to debug camera");
    }

    pub fn switch_to_player_camera(&mut self) {
        self.camera_manager.set_active(CameraType::Player);
        crate::info!("Switched to player camera");
    }

    pub fn is_using_debug_camera(&self) -> bool {
        matches!(self.camera_manager.get_active_type(), CameraType::Debug)
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
                    ui.label("⏹️");
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
                    ui.label("📜");
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
                    let header = egui::CollapsingHeader::new(name)
                        .default_open(!*collapsed)
                        .show(ui, |ui| {
                            show_entity_tree(ui, children, selected, name);
                        });
                    *collapsed = !header.body_returned.is_some();
                });
            });
        }
        EntityNode::Light {
            id,
            name,
        } => {
            ui.horizontal(|ui| {
                handle.ui(ui, |ui| {
                    ui.label("💡");
                });
                let resp = ui.selectable_label(selected.as_ref().eq(&Some(&id)), name);
                if resp.clicked() {
                    *selected = Some(id);
                }
            });
        }
    });
}

/// Describes an action that is undoable
#[derive(Debug)]
pub enum UndoableAction {
    Transform(hecs::Entity, Transform),
    Spawn(hecs::Entity),
    Label(hecs::Entity, String, EntityType),
    RemoveComponent(hecs::Entity, ComponentType)
}
#[derive(Debug)]
pub enum EntityType {
    Entity,
    Light,
}

impl UndoableAction {
    pub fn push_to_undo(undo_stack: &mut Vec<UndoableAction>, action: Self) {
        undo_stack.push(action);
        // log::debug!("Undo Stack contents: {:#?}", undo_stack);
    }

    pub fn undo(&self, world: &mut hecs::World) -> anyhow::Result<()> {
        match self {
            UndoableAction::Transform(entity, transform) => {
                        if let Ok(mut q) = world.query_one::<&mut Transform>(*entity) {
                            if let Some(e_t) = q.get() {
                                *e_t = *transform;
                                log::debug!("Reverted transform");
                                Ok(())
                            } else {
                                Err(anyhow::anyhow!("Unable to query the entity"))
                            }
                        } else {
                            Err(anyhow::anyhow!("Could not find an entity to query"))
                        }
                    }
            UndoableAction::Spawn(entity) => {
                        if world.despawn(*entity).is_ok() {
                            log::debug!("Undid spawn by despawning entity {:?}", entity);
                            Ok(())
                        } else {
                            Err(anyhow::anyhow!("Failed to despawn entity {:?}", entity))
                        }
                    }
            UndoableAction::Label(entity, original_label, entity_type) => {
                match entity_type {
                    EntityType::Entity => {
                        if let Ok(mut q) = world.query_one::<&mut AdoptedEntity>(*entity) {
                            if let Some(adopted) = q.get() {
                                adopted.model_mut().label = original_label.clone();
                                log::debug!("Reverted label for entity {:?} to '{}'", entity, original_label);
                                Ok(())
                            } else {
                                Err(anyhow::anyhow!("Unable to query the entity for label revert"))
                            }
                        } else {
                            Err(anyhow::anyhow!("Could not find an entity to query for label revert"))
                        }
                    },
                    EntityType::Light => {
                        if let Ok(mut q) = world.query_one::<&mut Light>(*entity) {
                            if let Some(adopted) = q.get() {
                                adopted.label = original_label.clone();
                                log::debug!("Reverted label for entity {:?} to '{}'", entity, original_label);
                                Ok(())
                            } else {
                                Err(anyhow::anyhow!("Unable to query the light for label revert"))
                            }
                        } else {
                            Err(anyhow::anyhow!("Could not find a light to query for label revert"))
                        }
                    },
                }
            },
            UndoableAction::RemoveComponent(entity, c_type) => {
                match c_type {
                    ComponentType::Script(component) => {
                        world.insert_one(*entity, component.clone())?;
                    }
                }
                Ok(())
            }
        }
    }
}

/// This enum will be used to describe the type of command/signal. This is only between
/// the editor and unlike SceneCommand, this will ping a signal everywhere in that scene
pub enum Signal {
    None,
    Copy(SceneEntity),
    Paste(SceneEntity),
    Delete,
    Undo,
    ScriptAction(ScriptAction),
    CameraAction(CameraAction),
    Play,
    StopPlaying,
    AddComponent(hecs::Entity, EntityType),
    RemoveComponent(hecs::Entity, ComponentType),
    CreateEntity,
    LogEntities,
}

impl Default for Editor {
    fn default() -> Self {
        Editor::new()
    }
}

#[derive(Debug)]
pub enum ComponentType {
    Script(ScriptComponent),
    // Camera,
}