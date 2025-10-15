pub mod component;
pub mod dock;
pub mod input;
pub mod repl;
pub mod scene;

pub(crate) use crate::editor::dock::*;

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};
use crossbeam_channel::Receiver;
use crate::build::build;
use crate::camera::UndoableCameraAction;
use crate::debug;
use dropbear_engine::future::FutureHandle;
use dropbear_engine::graphics::{RenderContext, Shader};
use dropbear_engine::model::ModelId;
use dropbear_engine::{
    camera::Camera,
    entity::{AdoptedEntity, Transform},
    graphics::SharedGraphicsContext,
    lighting::{Light, LightManager},
    scene::SceneCommand,
};
use egui::{self, Context};
use egui_dock_fork::{DockArea, DockState, NodeIndex, Style};
use eucalyptus_core::input::InputState;
use eucalyptus_core::scripting::{BuildStatus, ScriptManager};
use eucalyptus_core::states::{
    CameraConfig, EditorTab, EntityNode, LightConfig, ModelProperties, PROJECT, SCENES,
    SceneEntity, ScriptComponent,
};
use eucalyptus_core::utils::ViewportMode;
use eucalyptus_core::{
    camera::{CameraComponent, CameraType, DebugCamera},
    states::WorldLoadingStatus,
};
use eucalyptus_core::{fatal, info, states, success, warn};
use hecs::World;
use parking_lot::Mutex;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use transform_gizmo_egui::{EnumSet, Gizmo, GizmoMode};
use wgpu::{Color, Extent3d, RenderPipeline};
use winit::{keyboard::KeyCode, window::Window};

pub struct Editor {
    scene_command: SceneCommand,
    pub world: Box<World>,
    dock_state: DockState<EditorTab>,
    texture_id: Option<egui::TextureId>,
    size: Extent3d,
    render_pipeline: Option<RenderPipeline>,
    light_manager: LightManager,
    color: Color,

    active_camera: Arc<Mutex<Option<hecs::Entity>>>,

    is_viewport_focused: bool,
    // is_cursor_locked: bool,
    window: Option<Arc<Window>>,

    show_new_project: bool,
    project_name: String,
    pub(crate) project_path: Arc<Mutex<Option<PathBuf>>>,
    pending_scene_switch: bool,

    gizmo: Gizmo,
    pub(crate) selected_entity: Option<hecs::Entity>,
    viewport_mode: ViewportMode,

    pub(crate) signal: Signal,
    pub(crate) undo_stack: Vec<UndoableAction>,
    // todo: add redo (later)
    // redo_stack: Vec<UndoableAction>,
    pub(crate) editor_state: EditorState,
    gizmo_mode: EnumSet<GizmoMode>,

    pub(crate) script_manager: ScriptManager,
    play_mode_backup: Option<PlayModeBackup>,

    /// State of the input
    pub(crate) input_state: InputState,

    // channels
    /// A threadsafe Unbounded Receiver, typically used for checking the status of the world loading
    progress_tx: Option<UnboundedReceiver<WorldLoadingStatus>>,
    /// Used to check if the world has been loaded in
    is_world_loaded: IsWorldLoadedYet,
    /// Used to fetch the current status of the loading, so it can be used for different
    /// egui loading windows or splash screens and such.
    current_state: WorldLoadingStatus,

    // handles for futures
    world_load_handle: Option<FutureHandle>,
    pub(crate) alt_pending_spawn_queue: Vec<FutureHandle>,
    world_receiver: Option<oneshot::Receiver<hecs::World>>,

    // building
    pub progress_rx: Option<Receiver<BuildStatus>>,
    pub handle_created: Option<FutureHandle>,
    pub build_logs: Vec<String>,
    pub build_progress: f32,
    pub show_build_window: bool,
    pub last_build_error: Option<String>,
    pub show_build_error_window: bool,

    dock_state_shared: Option<Arc<Mutex<DockState<EditorTab>>>>,
}

impl Editor {
    pub fn new() -> anyhow::Result<Self> {
        let tabs = vec![EditorTab::Viewport];
        let mut dock_state = DockState::new(tabs);

        let surface = dock_state.main_surface_mut();
        let [_old, right] =
            surface.split_right(NodeIndex::root(), 0.25, vec![EditorTab::ModelEntityList]);
        let [_old, _] =
            surface.split_left(NodeIndex::root(), 0.20, vec![EditorTab::ResourceInspector]);
        let [_old, _] = surface.split_below(
            right,
            0.5,
            vec![EditorTab::AssetViewer],
        );

        // this shit doesn't work :(
        // nvm it works
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
                panic!(
                    "Fatal: {} deadlocks detected, unable to continue on normal process",
                    deadlocks.len()
                );
            }
        });

        Ok(Self {
            scene_command: SceneCommand::None,
            dock_state,
            texture_id: None,
            size: Extent3d::default(),
            render_pipeline: None,
            color: Color::default(),
            is_viewport_focused: false,
            // is_cursor_locked: false,
            window: None,
            world: Box::new(World::new()),
            show_new_project: false,
            project_name: String::new(),
            project_path: Arc::new(Mutex::new(None)),
            pending_scene_switch: false,
            gizmo: Gizmo::default(),
            selected_entity: None,
            viewport_mode: ViewportMode::None,
            signal: Signal::None,
            undo_stack: Vec::new(),
            script_manager: ScriptManager::new()?,
            editor_state: EditorState::Editing,
            gizmo_mode: EnumSet::empty(),
            play_mode_backup: None,
            input_state: InputState::new(),
            light_manager: LightManager::new(),
            active_camera: Arc::new(Mutex::new(None)),
            progress_tx: None,
            is_world_loaded: IsWorldLoadedYet::new(),
            current_state: WorldLoadingStatus::Idle,
            world_load_handle: None,
            alt_pending_spawn_queue: vec![],
            world_receiver: None,
            progress_rx: None,
            handle_created: None,
            build_logs: Vec::new(),
            build_progress: 0.0,
            show_build_window: false,
            last_build_error: None,
            show_build_error_window: false,
            dock_state_shared: None,
        })
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
            let transform = *transform.unwrap_or(&Transform::default());

            let camera_config = if let Ok(mut camera_query) =
                self.world.query_one::<(&Camera, &CameraComponent)>(id)
            {
                if let Some((camera, component)) = camera_query.get() {
                    Some(CameraConfig::from_ecs_camera(camera, component))
                } else {
                    None
                }
            } else {
                None
            };

            let scene_entity = SceneEntity {
                model_path: adopted.model.path.clone(),
                label: adopted.model.label.clone(),
                transform,
                properties: properties.clone(),
                script: script.cloned(),
                camera: camera_config,
                entity_id: Some(id),
            };

            scene.entities.push(scene_entity);
            log::debug!("Pushed entity: {}", adopted.model.label);
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
                label: light.cube_model.label.to_string(),
                transform: *transform,
                light_component: light_component.clone(),
                enabled: light_component.enabled,
                entity_id: Some(id),
            };

            scene.lights.push(light_config);
            log::debug!("Pushed light into lights: {}", light.cube_model.label);
        }

        for (id, (camera, component)) in self.world.query::<(&Camera, &CameraComponent)>().iter() {
            if self.world.get::<&AdoptedEntity>(id).is_err() {
                let camera_config = CameraConfig::from_ecs_camera(camera, component);
                scene.cameras.push(camera_config);
                log::debug!("Pushed standalone camera into cameras: {}", camera.label);
            }
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
            let dock_state = self.dock_state.clone();
            config.dock_layout = Some(dock_state);
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

    /// The window when loading a project or a scene or anything that uses [`WorldLoadingStatus`]
    fn show_project_loading_window(&mut self, ctx: &egui::Context) {
        if let Some(ref mut rx) = self.progress_tx {
            match rx.try_recv() {
                Ok(status) => match status {
                    WorldLoadingStatus::LoadingEntity { index, name, total } => {
                        log::debug!("Loading entity: {} ({}/{})", name, index + 1, total);
                        self.current_state =
                            WorldLoadingStatus::LoadingEntity { index, name, total };
                    }
                    WorldLoadingStatus::LoadingLight { index, name, total } => {
                        log::debug!("Loading light: {} ({}/{})", name, index + 1, total);
                        self.current_state =
                            WorldLoadingStatus::LoadingLight { index, name, total };
                    }
                    WorldLoadingStatus::LoadingCamera { index, name, total } => {
                        log::debug!("Loading camera: {} ({}/{})", name, index + 1, total);
                        self.current_state =
                            WorldLoadingStatus::LoadingCamera { index, name, total };
                    }
                    WorldLoadingStatus::Completed => {
                        log::debug!(
                            "Received WorldLoadingStatus::Completed - project loading finished"
                        );
                        self.is_world_loaded.mark_project_loaded();
                        self.current_state = WorldLoadingStatus::Completed;
                        self.progress_tx = None;
                        log::debug!("Returning back");
                        return;
                    }
                    WorldLoadingStatus::Idle => {
                        log::debug!("Project loading is idle");
                    }
                },
                Err(_) => {
                    // log::debug!("Unable to receive the progress: {}", e);
                }
            }
        } else {
            log::debug!("No progress receiver available");
        }

        egui::Window::new("Loading Project")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size([300.0, 100.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Loading...");
                    });
                    // ui.add_space(5.0);
                    // ui.add(egui::ProgressBar::new(progress).text(format!("{:.0}%", progress * 100.0)));
                    match &self.current_state {
                        WorldLoadingStatus::Idle => {
                            ui.label("Initialising...");
                        }
                        WorldLoadingStatus::LoadingEntity { name, .. } => {
                            ui.label(format!("Loading entity: {}", name));
                        }
                        WorldLoadingStatus::LoadingLight { name, .. } => {
                            ui.label(format!("Loading light: {}", name));
                        }
                        WorldLoadingStatus::LoadingCamera { name, .. } => {
                            ui.label(format!("Loading camera: {}", name));
                        }
                        WorldLoadingStatus::Completed => {
                            ui.label("Done!");
                        }
                    }
                });
            });
    }

    /// Loads the project config.
    ///
    /// It uses an unbounded sender to send messages back to the receiver so it can
    /// be used within threads.
    pub async fn load_project_config(
        // &mut self,
        graphics: Arc<SharedGraphicsContext>,
        sender: Option<UnboundedSender<WorldLoadingStatus>>,
        world: &mut World,
        world_sender: Option<oneshot::Sender<hecs::World>>,
        active_camera: Arc<Mutex<Option<hecs::Entity>>>,
        project_path: Arc<Mutex<Option<PathBuf>>>,
        dock_state: Arc<Mutex<DockState<EditorTab>>>,
    ) -> anyhow::Result<()> {
        {
            let config = PROJECT.read();
            let mut path = project_path.lock();
            *path = Some(config.project_path.clone());

            if let Some(layout) = &config.dock_layout {
                let mut dock = dock_state.lock();
                let layout = layout.clone();
                *dock = layout.clone();
            }
        }

        let first_scene_opt = {
            let scenes = SCENES.read();
            scenes.first().cloned()
        };

        {
            if let Some(first_scene) = first_scene_opt {
                let cam = first_scene
                    .load_into_world(world, graphics, sender.clone())
                    .await?;
                let mut a_c = active_camera.lock();
                *a_c = Some(cam);

                log::info!(
                    "Successfully loaded scene with {} entities and {} camera configs",
                    first_scene.entities.len(),
                    first_scene.cameras.len(),
                );
            } else {
                let existing_debug_camera = {
                    world
                        .query::<(&Camera, &CameraComponent)>()
                        .iter()
                        .find_map(|(entity, (_, component))| {
                            if matches!(component.camera_type, CameraType::Debug) {
                                Some(entity)
                            } else {
                                None
                            }
                        })
                };

                if let Some(camera_entity) = existing_debug_camera {
                    log::info!("Using existing debug camera");
                    let mut a_c = active_camera.lock();
                    *a_c = Some(camera_entity);
                } else {
                    log::info!("No scenes found, creating default debug camera");

                    let debug_camera = Camera::predetermined(graphics, Some("Debug Camera"));
                    let component = DebugCamera::new();

                    {
                        let e = world.spawn((debug_camera, component));
                        let mut a_c = active_camera.lock();
                        *a_c = Some(e);
                    }
                }
            }
        }

        if let Some(ref s) = sender.clone() {
            let _ = s.send(WorldLoadingStatus::Completed);
        }

        if let Some(ws) = world_sender {
            let _ = ws.send(std::mem::take(world));
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
                    } else if ui.button("Play").clicked() {
                        self.signal = Signal::Play;
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
                                        model_path: e.model.path.clone(),
                                        label: e.model.label.clone(),
                                        transform: *t,
                                        properties: props.clone(),
                                        script: None,
                                        camera: None,
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
                    if ui_window.button("Open Kotlin REPL").clicked() {
                        self.dock_state.push_to_focused_leaf(EditorTab::KotlinREPL);
                    }
                });
                {
                    let cfg = PROJECT.read();
                    if cfg.editor_settings.is_debug_menu_shown {
                        debug::show_menu_bar(ui, &mut self.signal);
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
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

        let mut project_path = self.project_path.lock();
        crate::utils::show_new_project_window(
            ctx,
            &mut self.show_new_project,
            &mut self.project_name,
            &mut project_path,
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

    /// Restores transform components back to its original state before PlayMode.
    pub fn restore(&mut self) -> anyhow::Result<()> {
        if let Some(backup) = &self.play_mode_backup {
            for (entity_id, original_transform, original_properties, original_script) in
                &backup.entities
            {
                if let Ok(mut transform) = self.world.get::<&mut Transform>(*entity_id) {
                    *transform = *original_transform;
                }

                if let Ok(mut properties) = self.world.get::<&mut ModelProperties>(*entity_id) {
                    *properties = original_properties.clone();
                }

                let has_script = self.world.get::<&ScriptComponent>(*entity_id).is_ok();
                match (has_script, original_script) {
                    (true, Some(original)) => {
                        if let Ok(mut script) = self.world.get::<&mut ScriptComponent>(*entity_id) {
                            *script = original.clone();
                        }
                    }
                    (true, None) => {
                        let _ = self.world.remove_one::<ScriptComponent>(*entity_id);
                    }
                    (false, Some(original)) => {
                        let _ = self.world.insert_one(*entity_id, original.clone());
                    }
                    (false, None) => {}
                }
            }

            // for (entity_id, original_camera, original_component, original_follow_target) in
            //     &backup.camera_data
            // {
            //     if let Ok(mut camera) = self.world.get::<&mut Camera>(*entity_id) {
            //         *camera = original_camera.clone();
            //     }
            //
            //     if let Ok(mut component) = self.world.get::<&mut CameraComponent>(*entity_id) {
            //         *component = original_component.clone();
            //     }
            //
            //     let has_follow_target = self.world.get::<&CameraFollowTarget>(*entity_id).is_ok();
            //     match (has_follow_target, original_follow_target) {
            //         (true, Some(original)) => {
            //             // if let Ok(mut follow_target) =
            //             //     self.world.get::<&mut CameraFollowTarget>(*entity_id)
            //             // {
            //             //     *follow_target = original.clone();
            //             // }
            //         }
            //         (true, None) => {
            //             // {
            //             //     let _ = self.world
            //             //         .remove_one::<CameraFollowTarget>(*entity_id);
            //             // }
            //         }
            //         (false, Some(original)) => {
            //             {
            //                 let _ = self.world
            //                     .insert_one(*entity_id, original.clone());
            //             }
            //         }
            //         (false, None) => {
            //             // No change needed
            //         }
            //     }
            // }

            log::info!("Restored scene from play mode backup");

            self.play_mode_backup = None;
            Ok(())
        } else {
            Err(anyhow::anyhow!("No play mode backup found to restore"))
        }
    }

    pub fn create_backup(&mut self) -> anyhow::Result<()> {
        let mut entities = Vec::new();

        for (entity_id, (_, transform, properties)) in self
            .world
            .query::<(&AdoptedEntity, &Transform, &ModelProperties)>()
            .iter()
        {
            let script = self
                .world
                .query_one::<&ScriptComponent>(entity_id)
                .ok()
                .and_then(|mut s| s.get().cloned());
            entities.push((entity_id, *transform, properties.clone(), script));
        }

        let mut camera_data = Vec::new();

        for (entity_id, (camera, component)) in
            self.world.query::<(&Camera, &CameraComponent)>().iter()
        {
            camera_data.push((
                entity_id,
                camera.clone(),
                component.clone(),
                // follow_target.cloned(),
            ));
        }

        self.play_mode_backup = Some(PlayModeBackup {
            entities,
            camera_data,
        });

        log::info!(
            "Created play mode backup with {} entities and {} cameras",
            self.play_mode_backup.as_ref().unwrap().entities.len(),
            self.play_mode_backup.as_ref().unwrap().camera_data.len()
        );
        Ok(())
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
            let mut active_camera = self.active_camera.lock();
            *active_camera = Some(camera_entity);
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
            .find_map(
                |(e, (_cam, comp))| {
                    if comp.starting_camera { Some(e) } else { None }
                },
            );

        if let Some(camera_entity) = player_camera {
            let mut active_camera = self.active_camera.lock();
            *active_camera = Some(camera_entity);
            info!("Switched to player camera");
        } else {
            warn!("No player camera found in the world");
        }
    }

    pub fn is_using_debug_camera(&self) -> bool {
        let active_camera = self.active_camera.lock();
        if let Some(active_camera_entity) = *active_camera
            && let Ok(mut query) = self
                .world
                .query_one::<&CameraComponent>(active_camera_entity)
            && let Some(component) = query.get()
        {
            return matches!(component.camera_type, CameraType::Debug);
        }
        false
    }

    /// Loads all the wgpu resources such as renderer.
    ///
    /// **Note**: To be ran AFTER [`Editor::load_project_config`]
    pub fn load_wgpu_nerdy_stuff<'a>(&mut self, graphics: &mut RenderContext<'a>) {
        let shader = Shader::new(
            graphics.shared.clone(),
            include_str!("../../../resources/shaders/shader.wgsl"),
            Some("viewport_shader"),
        );

        self.light_manager
            .create_light_array_resources(graphics.shared.clone());

        if let Some(active_camera) = *self.active_camera.lock() {
            if let Ok(mut q) = self
                .world
                .query_one::<(&Camera, &CameraComponent)>(active_camera)
            {
                if let Some((camera, _component)) = q.get() {
                    let pipeline = graphics.create_render_pipline(
                        &shader,
                        vec![
                            &graphics.shared.texture_bind_layout.clone(),
                            camera.layout(),
                            self.light_manager.layout(),
                        ],
                        None,
                    );
                    self.render_pipeline = Some(pipeline);

                    self.light_manager.create_render_pipeline(
                        graphics.shared.clone(),
                        include_str!("../../../resources/shaders/light.wgsl"),
                        camera,
                        Some("Light Pipeline"),
                    );
                } else {
                    log_once::warn_once!(
                        "Unable to fetch the query result of camera: {:?}",
                        active_camera
                    )
                }
            } else {
                log_once::warn_once!(
                    "Unable to query camera, component and option<camerafollowtarget> for active camera: {:?}",
                    active_camera
                );
            }
        } else {
            log_once::warn_once!("No active camera found");
        }

        self.window = Some(graphics.shared.window.clone());
        self.is_world_loaded.mark_rendering_loaded();
    }
}

pub static LOGGED: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

fn show_entity_tree(
    ui: &mut egui::Ui,
    nodes: &mut [EntityNode],
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
            EntityNode::Script { .. } => {
                ui.horizontal(|ui| {
                    handle.ui(ui, |ui| {
                        ui.label("📜");
                    });
                    ui.label("Script");
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
                        *collapsed = header.body_returned.is_none();
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
    /// A change in transform. The entity + the old transform. Undoing will revert the transform
    Transform(hecs::Entity, Transform),
    #[allow(dead_code)] // don't know why its considered dead code, todo: check the cause
    /// A spawn of the entity. Undoing will delete the entity
    Spawn(hecs::Entity),
    /// A change of label of the entity. Undoing will revert its label
    Label(hecs::Entity, String, EntityType),
    /// Removing a component. Undoing will add back the component.
    RemoveComponent(hecs::Entity, Box<ComponentType>),
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

    pub fn undo(&self, world: &mut World) -> anyhow::Result<()> {
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
                            Arc::make_mut(&mut adopted.model).label = original_label.clone();
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
                match &**c_type {
                    ComponentType::Script(component) => {
                        world.insert_one(*entity, component.clone())?;
                    }
                    ComponentType::Camera(camera, component) => {
                        // if let Some(f) = follow {
                        //     {
                        //         world
                        //             .insert(*entity, (camera.clone(), component.clone(), f.clone()))?;
                        //     }
                        // } else {
                        world.insert(*entity, (camera.clone(), component.clone()))?;
                        // }
                    }
                }
                Ok(())
            }
            UndoableAction::CameraAction(action) => {
                match action {
                    UndoableCameraAction::Speed(entity, speed) => {
                        if let Ok(mut q) =
                            world.query_one::<(&mut Camera, &mut CameraComponent)>(*entity)
                            && let Some((cam, comp)) = q.get()
                        {
                            comp.speed = *speed;
                            comp.update(cam);
                        }
                    }
                    UndoableCameraAction::Sensitivity(entity, sensitivity) => {
                        if let Ok(mut q) =
                            world.query_one::<(&mut Camera, &mut CameraComponent)>(*entity)
                            && let Some((cam, comp)) = q.get()
                        {
                            comp.sensitivity = *sensitivity;
                            comp.update(cam);
                        }
                    }
                    UndoableCameraAction::Fov(entity, fov) => {
                        if let Ok(mut q) =
                            world.query_one::<(&mut Camera, &mut CameraComponent)>(*entity)
                            && let Some((cam, comp)) = q.get()
                        {
                            comp.fov_y = *fov;
                            comp.update(cam);
                        }
                    }
                    UndoableCameraAction::Type(entity, camera_type) => {
                        if let Ok(mut q) =
                            world.query_one::<(&mut Camera, &mut CameraComponent)>(*entity)
                            && let Some((cam, comp)) = q.get()
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
    // ScriptAction(ScriptAction),
    // not actions required because follow target is set through scripting.
    // CameraAction(CameraAction),
    Play,
    StopPlaying,
    AddComponent(hecs::Entity, EntityType),
    RemoveComponent(hecs::Entity, Box<ComponentType>),
    CreateEntity,
    LogEntities,
    Spawn(PendingSpawn2),
}

#[derive(Debug)]
#[allow(dead_code)]
// todo: deal with the Camera and create an implementation
pub enum ComponentType {
    Script(ScriptComponent),
    Camera(Box<Camera>, CameraComponent),
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
        // Option<CameraFollowTarget>,
    )>,
}

pub enum EditorState {
    Editing,
    Building,
    Playing,
}

pub enum PendingSpawn2 {
    Light,
    Plane,
    Cube,
    Camera,
}

pub(crate) struct IsWorldLoadedYet {
    /// Whether the project configuration and world data has been loaded
    pub project_loaded: bool,
    /// Whether the scene rendering and UI setup is complete
    pub scene_loaded: bool,
    /// Checks if the wgpu rendering contexts have been initialised for rendering
    pub rendering_loaded: bool,
}

impl IsWorldLoadedYet {
    pub fn new() -> Self {
        Self {
            project_loaded: false,
            scene_loaded: false,
            rendering_loaded: false,
        }
    }

    pub fn is_fully_loaded(&self) -> bool {
        self.project_loaded && self.scene_loaded
    }

    // I don't know whether this should be kept or removed, but
    // im adding dead code just in case.
    #[allow(dead_code)]
    pub fn is_project_ready(&self) -> bool {
        self.project_loaded
    }

    pub fn mark_project_loaded(&mut self) {
        self.project_loaded = true;
    }

    pub fn mark_scene_loaded(&mut self) {
        self.scene_loaded = true;
    }

    pub fn mark_rendering_loaded(&mut self) {
        self.rendering_loaded = true;
    }
}

impl Default for IsWorldLoadedYet {
    fn default() -> Self {
        Self::new()
    }
}
