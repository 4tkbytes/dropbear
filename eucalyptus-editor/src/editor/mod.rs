pub mod component;
pub mod dock;
pub mod input;
pub mod scene;

pub(crate) use crate::editor::dock::*;

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};

use crate::{build::build, debug::DependencyInstaller};
use crate::camera::UndoableCameraAction;
use crate::debug;
use dropbear_engine::{
    camera::Camera,
    entity::{AdoptedEntity, Transform},
    graphics::SharedGraphicsContext,
    lighting::{Light, LightManager},
    scene::SceneCommand,
};
use egui::{self, Context};
use egui_dock_fork::{DockArea, DockState, NodeIndex, Style};
use eucalyptus_core::{camera::{
    CameraAction, CameraComponent, CameraFollowTarget, CameraType, DebugCamera,
}};
use eucalyptus_core::input::InputState;
use eucalyptus_core::scripting::{ScriptAction, ScriptManager};
use eucalyptus_core::states::{
    CameraConfig, EditorTab, EntityNode, LightConfig, ModelProperties, PROJECT, SCENES,
    SceneEntity, ScriptComponent,
};
use eucalyptus_core::utils::ViewportMode;
use eucalyptus_core::{fatal, info, states, success, warn};
use hecs::World;
use log;
use parking_lot::Mutex;
use transform_gizmo_egui::{EnumSet, Gizmo, GizmoMode};
use wgpu::{Color, Extent3d, RenderPipeline};
use winit::{keyboard::KeyCode, window::Window};

pub struct Editor {
    scene_command: SceneCommand,
    world: Arc<World>,
    dock_state: DockState<EditorTab>,
    texture_id: Option<egui::TextureId>,
    size: Extent3d,
    render_pipeline: Option<RenderPipeline>,
    light_manager: LightManager,
    color: Color,

    active_camera: Option<hecs::Entity>,

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

    // channels
    dep_installer: DependencyInstaller,
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

                for (i, threads) in deadlocks.iter().enumerate() {
                    log::error!("Deadlock #{}", i);
                    for t in threads {
                        log::error!("Thread Id {:#?}", t.thread_id());
                        log::error!("{:#?}", t.backtrace());
                    }
                }
                panic!("Fatal: {} deadlocks detected, unable to continue on normal process", deadlocks.len());
            }
        });

        Self {
            scene_command: SceneCommand::None,
            dock_state,
            texture_id: None,
            size: Extent3d::default(),
            render_pipeline: None,
            color: Color::default(),
            is_viewport_focused: false,
            // is_cursor_locked: false,
            window: None,
            world: Arc::new(World::new()),
            show_new_project: false,
            project_name: String::new(),
            project_path: None,
            pending_scene_switch: false,
            gizmo: Gizmo::default(),
            selected_entity: None,
            viewport_mode: ViewportMode::None,
            signal: Signal::None,
            undo_stack: Vec::new(),
            script_manager: ScriptManager::new().unwrap(),
            editor_state: EditorState::Editing,
            gizmo_mode: EnumSet::empty(),
            play_mode_backup: None,
            input_state: InputState::new(),
            light_manager: LightManager::new(),
            active_camera: None,
            dep_installer: DependencyInstaller::default()
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
        let mut scenes = SCENES.write();

        let scene_index = if scenes.is_empty() {
            return Err(anyhow::anyhow!("No scenes loaded to save"));
        } else {
            0
        };

        let scene = &mut scenes[scene_index];
        scene.entities.clear();
        scene.lights.clear();
        scene.cameras.clear();

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
            log::debug!("Pushed light into lights: {}", light.label());
        }

        for (_id, (camera, component, follow_target)) in self
            .world
            .query::<(&Camera, &CameraComponent, Option<&CameraFollowTarget>)>()
            .iter()
        {
            let camera_config = CameraConfig::from_ecs_camera(camera, component, follow_target);
            scene.cameras.push(camera_config);
            log::debug!("Pushed camera into cameras: {}", camera.label);
        }

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
            let mut config = PROJECT.write();
            config.dock_layout = Some(self.dock_state.clone());
        }

        {
            let (scene_clone, project_path) = {
                let scenes = SCENES.read();
                let project = PROJECT.read();
                (scenes[0].clone(), project.project_path.clone())
            };

            scene_clone.write_to(&project_path)?;

            let mut config = PROJECT.write();
            config.write_to_all()?;
        }

        Ok(())
    }

    pub async fn load_project_config(&mut self, graphics: Arc<SharedGraphicsContext>) -> anyhow::Result<()> {
        {
            let config = PROJECT.read();

            self.project_path = Some(config.project_path.clone());

            if let Some(layout) = &config.dock_layout {
                self.dock_state = layout.clone();
            }
        }

        let first_scene_opt = {
            let scenes = SCENES.read();
            scenes.first().cloned()
        };

        {
            if let Some(first_scene) = first_scene_opt {
                let cam = first_scene.load_into_world(Arc::get_mut(&mut self.world).unwrap(), graphics).await?;
                self.active_camera = Some(cam);

                log::info!(
                    "Successfully loaded scene with {} entities and {} camera configs",
                    first_scene.entities.len(),
                    first_scene.cameras.len(),
                );
            } else {
                let existing_debug_camera = self
                    .world
                    .query::<(&Camera, &CameraComponent)>()
                    .iter()
                    .find_map(|(entity, (_, component))| {
                        if matches!(component.camera_type, CameraType::Debug) {
                            Some(entity)
                        } else {
                            None
                        }
                    });

                if let Some(camera_entity) = existing_debug_camera {
                    log::info!("Using existing debug camera");
                    self.active_camera = Some(camera_entity);
                } else {
                    log::info!("No scenes found, creating default debug camera");

                    let debug_camera = Camera::predetermined(graphics, Some("Debug Camera"));
                    let component = DebugCamera::new();

                    let e = Arc::get_mut(&mut self.world).unwrap().spawn((debug_camera, component));
                    self.active_camera = Some(e);
                }
            }
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
                                fatal!("Error saving project: {}", e);
                            }
                        }
                        success!("Successfully saved project");
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
                                let proj = PROJECT.read();
                                match build(proj.project_path.join(format!("{}.eucp", proj.project_name.clone())).clone()) {
                                    Ok(thingy) => success!("Project output at {}", thingy.display()),
                                    Err(e) => {
                                        fatal!("Unable to build project [{}]: {}", proj.project_path.clone().display(), e);
                                    },
                                }
                            }
                        }
                        ui.label("Package"); // todo: create a window for label
                    });
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        match self.save_project_config() {
                            Ok(_) => {
                                log::info!("Saved, quitting...");
                                std::process::exit(0);
                            }
                            Err(e) => {
                                fatal!("Error saving project: {}", e);
                            }
                        }
                        success!("Successfully saved project");
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if ui.button("Copy").clicked() {
                        if let Some(entity) = &self.selected_entity {
                            let query = self.world.query_one::<(&AdoptedEntity, &Transform, &ModelProperties)>(*entity);
                            if let Ok(mut q) = query {
                                if let Some((e, t, props)) = q.get() {
                                    let s_entity = states::SceneEntity {
                                        model_path: e.model().path.clone(),
                                        label: e.model().label.clone(),
                                        transform: *t,
                                        properties: props.clone(),
                                        script: None,
                                        entity_id: None,
                                    };
                                    self.signal = Signal::Copy(s_entity);

                                    info!("Copied selected entity!");
                                } else {
                                    warn!("Unable to copy entity: Unable to fetch world entity properties");
                                }
                            } else {
                                warn!("Unable to copy entity: Unable to obtain lock");
                            }
                        } else {
                            warn!("Unable to copy entity: None selected");
                        }
                    }

                    if ui.button("Paste").clicked() {
                        match &self.signal {
                            Signal::Copy(entity) => {
                                self.signal = Signal::Paste(entity.clone());
                            }
                            _ => {
                                warn!("Unable to paste: You haven't selected anything!");
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
                {
                    let cfg = PROJECT.read();
                    if cfg.editor_settings.is_debug_menu_shown {
                        debug::show_menu_bar(ui, &mut self.signal, &mut self.dep_installer);
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
                        active_camera: &mut self.active_camera,
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
        let debug_camera = self
            .world
            .query::<(&Camera, &CameraComponent)>()
            .iter()
            .find_map(|(e, (_cam, comp))| {
                if matches!(comp.camera_type, CameraType::Debug) {
                    Some(e)
                } else {
                    None
                }
            });

        if let Some(camera_entity) = debug_camera {
            self.active_camera = Some(camera_entity);
            info!("Switched to debug camera");
        } else {
            warn!("No debug camera found in the world");
        }
    }

    pub fn switch_to_player_camera(&mut self) {
        let player_camera = self
            .world
            .query::<(&Camera, &CameraComponent)>()
            .iter()
            .find_map(|(e, (_cam, comp))| {
                if matches!(comp.camera_type, CameraType::Player) {
                    Some(e)
                } else {
                    None
                }
            });

        if let Some(camera_entity) = player_camera {
            self.active_camera = Some(camera_entity);
            info!("Switched to player camera");
        } else {
            warn!("No player camera found in the world");
        }
    }

    pub fn is_using_debug_camera(&self) -> bool {
        if let Some(active_camera_entity) = self.active_camera {
            if let Ok(mut query) = self
                .world
                .query_one::<&CameraComponent>(active_camera_entity)
            {
                if let Some(component) = query.get() {
                    return matches!(component.camera_type, CameraType::Debug);
                }
            }
        }
        false
    }
}

pub static LOGGED: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

fn show_entity_tree(
    ui: &mut egui::Ui,
    nodes: &mut Vec<EntityNode>,
    selected: &mut Option<hecs::Entity>,
    id_source: &str,
) {
    egui_dnd::Dnd::new(ui, id_source).show_vec(nodes, |ui, item, handle, _dragging| {
        match item.clone() {
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
            EntityNode::Light { id, name } => {
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
            EntityNode::Camera {
                id,
                name,
                camera_type,
            } => {
                ui.horizontal(|ui| {
                    handle.ui(ui, |ui| {
                        let icon = match camera_type {
                            CameraType::Debug => "🎥",  // Debug camera
                            CameraType::Player => "📹", // Player camera
                            CameraType::Normal => "📷", // Normal camera
                        };
                        ui.label(icon);
                    });
                    let display_name = format!(
                        "{} ({})",
                        name,
                        match camera_type {
                            CameraType::Debug => "Debug",
                            CameraType::Player => "Player",
                            CameraType::Normal => "Normal",
                        }
                    );
                    let resp = ui.selectable_label(selected.as_ref().eq(&Some(&id)), display_name);
                    if resp.clicked() {
                        *selected = Some(id);
                    }
                });
            }
        }
    });
}

/// Describes an action that is undoable
#[derive(Debug)]
pub enum UndoableAction {
    Transform(hecs::Entity, Transform),
    Spawn(hecs::Entity),
    Label(hecs::Entity, String, EntityType),
    RemoveComponent(hecs::Entity, ComponentType),
    #[allow(dead_code)]
    CameraAction(UndoableCameraAction),
}
#[derive(Debug)]
#[allow(dead_code)]
// todo: deal with why there is no Camera
pub enum EntityType {
    Entity,
    Light,
    Camera,
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
            UndoableAction::Label(entity, original_label, entity_type) => match entity_type {
                EntityType::Entity => {
                    if let Ok(mut q) = world.query_one::<&mut AdoptedEntity>(*entity) {
                        if let Some(adopted) = q.get() {
                            adopted.model_mut().label = original_label.clone();
                            log::debug!(
                                "Reverted label for entity {:?} to '{}'",
                                entity,
                                original_label
                            );
                            Ok(())
                        } else {
                            Err(anyhow::anyhow!(
                                "Unable to query the entity for label revert"
                            ))
                        }
                    } else {
                        Err(anyhow::anyhow!(
                            "Could not find an entity to query for label revert"
                        ))
                    }
                }
                EntityType::Light => {
                    if let Ok(mut q) = world.query_one::<&mut Light>(*entity) {
                        if let Some(adopted) = q.get() {
                            adopted.label = original_label.clone();
                            log::debug!(
                                "Reverted label for light {:?} to '{}'",
                                entity,
                                original_label
                            );
                            Ok(())
                        } else {
                            Err(anyhow::anyhow!(
                                "Unable to query the light for label revert"
                            ))
                        }
                    } else {
                        Err(anyhow::anyhow!(
                            "Could not find a light to query for label revert"
                        ))
                    }
                }
                EntityType::Camera => {
                    if let Ok(mut q) = world.query_one::<&mut Camera>(*entity) {
                        if let Some(adopted) = q.get() {
                            adopted.label = original_label.clone();
                            log::debug!(
                                "Reverted label for camera {:?} to '{}'",
                                entity,
                                original_label
                            );
                            Ok(())
                        } else {
                            Err(anyhow::anyhow!(
                                "Unable to query the camera for label revert"
                            ))
                        }
                    } else {
                        Err(anyhow::anyhow!(
                            "Could not find a camera to query for label revert"
                        ))
                    }
                }
            },
            UndoableAction::RemoveComponent(entity, c_type) => {
                match c_type {
                    ComponentType::Script(component) => {
                        world.insert_one(*entity, component.clone())?;
                    }
                    ComponentType::Camera(camera, component, follow) => {
                        if let Some(f) = follow {
                            world
                                .insert(*entity, (camera.clone(), component.clone(), f.clone()))?;
                        } else {
                            world.insert(*entity, (camera.clone(), component.clone()))?;
                        }
                    }
                }
                Ok(())
            }
            UndoableAction::CameraAction(action) => {
                match action {
                    UndoableCameraAction::Speed(entity, speed) => {
                        if let Ok((cam, comp)) =
                            world.query_one_mut::<(&mut Camera, &mut CameraComponent)>(*entity)
                        {
                            comp.speed = *speed;
                            comp.update(cam);
                        }
                    }
                    UndoableCameraAction::Sensitivity(entity, sensitivity) => {
                        if let Ok((cam, comp)) =
                            world.query_one_mut::<(&mut Camera, &mut CameraComponent)>(*entity)
                        {
                            comp.sensitivity = *sensitivity;
                            comp.update(cam);
                        }
                    }
                    UndoableCameraAction::FOV(entity, fov) => {
                        if let Ok((cam, comp)) =
                            world.query_one_mut::<(&mut Camera, &mut CameraComponent)>(*entity)
                        {
                            comp.fov_y = *fov;
                            comp.update(cam);
                        }
                    }
                    UndoableCameraAction::Type(entity, camera_type) => {
                        if let Ok((cam, comp)) =
                            world.query_one_mut::<(&mut Camera, &mut CameraComponent)>(*entity)
                        {
                            comp.camera_type = *camera_type;
                            comp.update(cam);
                        }
                    }
                };
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
    Spawn(PendingSpawn2),
}

#[derive(Debug)]
#[allow(dead_code)]
// todo: deal with the Camera and create an implementation
pub enum ComponentType {
    Script(ScriptComponent),
    Camera(Camera, CameraComponent, Option<CameraFollowTarget>),
}

#[derive(Clone)]
pub struct PlayModeBackup {
    entities: Vec<(
        hecs::Entity,
        Transform,
        ModelProperties,
        Option<ScriptComponent>,
    )>,
    camera_data: Vec<(
        hecs::Entity,
        Camera,
        CameraComponent,
        Option<CameraFollowTarget>,
    )>,
}

impl PlayModeBackup {
    pub fn create_backup(editor: &mut Editor) -> anyhow::Result<()> {
        let mut entities = Vec::new();

        for (entity_id, (_, transform, properties)) in editor
            .world
            .query::<(&AdoptedEntity, &Transform, &ModelProperties)>()
            .iter()
        {
            let script = editor
                .world
                .query_one::<&ScriptComponent>(entity_id)
                .ok()
                .and_then(|mut s| s.get().map(|script| script.clone()));
            entities.push((entity_id, *transform, properties.clone(), script));
        }

        let mut camera_data = Vec::new();

        for (entity_id, (camera, component, follow_target)) in editor
            .world
            .query::<(&Camera, &CameraComponent, Option<&CameraFollowTarget>)>()
            .iter()
        {
            camera_data.push((
                entity_id,
                camera.clone(),
                component.clone(),
                follow_target.cloned(),
            ));
        }

        editor.play_mode_backup = Some(PlayModeBackup {
            entities,
            camera_data,
        });

        log::info!(
            "Created play mode backup with {} entities and {} cameras",
            editor.play_mode_backup.as_ref().unwrap().entities.len(),
            editor.play_mode_backup.as_ref().unwrap().camera_data.len()
        );
        Ok(())
    }

    pub fn restore(editor: &mut Editor) -> anyhow::Result<()> {
        if let Some(backup) = &editor.play_mode_backup {
            // Restore entity states
            for (entity_id, original_transform, original_properties, original_script) in
                &backup.entities
            {
                if let Ok(mut transform) = editor.world.get::<&mut Transform>(*entity_id) {
                    *transform = *original_transform;
                }

                if let Ok(mut properties) = editor.world.get::<&mut ModelProperties>(*entity_id) {
                    *properties = original_properties.clone();
                }

                let has_script = editor.world.get::<&ScriptComponent>(*entity_id).is_ok();
                match (has_script, original_script) {
                    (true, Some(original)) => {
                        if let Ok(mut script) = editor.world.get::<&mut ScriptComponent>(*entity_id)
                        {
                            *script = original.clone();
                        }
                    }
                    (true, None) => {
                        let _ = Arc::get_mut(&mut editor.world).unwrap().remove_one::<ScriptComponent>(*entity_id);
                    }
                    (false, Some(original)) => {
                        let _ = Arc::get_mut(&mut editor.world).unwrap().insert_one(*entity_id, original.clone());
                    }
                    (false, None) => {
                        // No change needed
                    }
                }
            }

            // Restore camera states
            for (entity_id, original_camera, original_component, original_follow_target) in
                &backup.camera_data
            {
                if let Ok(mut camera) = editor.world.get::<&mut Camera>(*entity_id) {
                    *camera = original_camera.clone();
                }

                if let Ok(mut component) = editor.world.get::<&mut CameraComponent>(*entity_id) {
                    *component = original_component.clone();
                }

                let has_follow_target = editor.world.get::<&CameraFollowTarget>(*entity_id).is_ok();
                match (has_follow_target, original_follow_target) {
                    (true, Some(original)) => {
                        if let Ok(mut follow_target) =
                            editor.world.get::<&mut CameraFollowTarget>(*entity_id)
                        {
                            *follow_target = original.clone();
                        }
                    }
                    (true, None) => {
                        let _ = Arc::get_mut(&mut editor.world).unwrap().remove_one::<CameraFollowTarget>(*entity_id);
                    }
                    (false, Some(original)) => {
                        let _ = Arc::get_mut(&mut editor.world).unwrap().insert_one(*entity_id, original.clone());
                    }
                    (false, None) => {
                        // No change needed
                    }
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

pub enum PendingSpawn2 {
    CreateLight,
    CreatePlane,
    CreateCube,
    CreateCamera,
}
