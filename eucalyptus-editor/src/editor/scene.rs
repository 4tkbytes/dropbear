
use super::*;
use dropbear_engine::graphics::{InstanceRaw, RenderContext};
use dropbear_engine::model::{Model, MODEL_CACHE};
use dropbear_engine::procedural::plane::PlaneBuilder;
use dropbear_engine::{
    entity::{AdoptedEntity, Transform},
    lighting::{Light, LightComponent},
    model::{DrawLight, DrawModel},
    scene::{Scene, SceneCommand},
};
use egui::{Align2, Image};
use eucalyptus_core::camera::PlayerCamera;
use eucalyptus_core::states::{WorldLoadingStatus};
use eucalyptus_core::{logging};
use hecs::Entity;
use log;
use parking_lot::Mutex;
use tokio::sync::mpsc::unbounded_channel;
use wgpu::Color;
use wgpu::util::DeviceExt;
use winit::{event_loop::ActiveEventLoop, keyboard::KeyCode};
use crate::signal::SignalController;
use crate::spawn::PendingSpawnController;

impl Scene for Editor {
    fn load(&mut self, graphics: &mut RenderContext) {
        let (tx, rx) = unbounded_channel::<WorldLoadingStatus>();
        let (tx2, rx2) = oneshot::channel::<World>();
        self.progress_tx = Some(rx);
        self.world_receiver = Some(rx2);

        let graphics_shared = graphics.shared.clone();
        let active_camera_clone = self.active_camera.clone();
        let project_path_clone = self.project_path.clone();
        let dock_state_clone = Arc::new(Mutex::new(self.dock_state.clone()));

        let handle = graphics.shared.future_queue.push(async move {
            let mut temp_world = World::new();
            if let Err(e) = Self::load_project_config(graphics_shared, Some(tx), &mut temp_world, Some(tx2), active_camera_clone, project_path_clone, dock_state_clone).await {
                log::error!("Failed to load project config: {}", e);
            }
        });

        self.world_load_handle = Some(handle);

        self.window = Some(graphics.shared.window.clone());
        self.is_world_loaded.mark_scene_loaded();
    }

    fn update(&mut self, dt: f32, graphics: &mut RenderContext) {
        if let Some(mut receiver) = self.world_receiver.take() {
            self.show_project_loading_window(&graphics.shared.get_egui_context());
            if let Ok(loaded_world) = receiver.try_recv() {
                self.world = loaded_world;
                self.is_world_loaded.mark_project_loaded();
                log::info!("World received");
            } else {
                self.world_receiver = Some(receiver);
                return;
            }
        }

        if !self.is_world_loaded.is_fully_loaded() {
            log::debug!("Scene is not fully loaded, initializing...");
            // self.load(graphics).await;
            return;
        } else {
            log_once::debug_once!("Scene has fully loaded");
        }

        if !self.is_world_loaded.rendering_loaded && self.is_world_loaded.is_fully_loaded() {
            self.load_wgpu_nerdy_stuff(graphics);
            return;
        }

        match self.check_up(graphics.shared.clone(), graphics.shared.future_queue.clone()) {
            Ok(_) => {}
            Err(e) => {
                fatal!("{}", e);
            }
        }

        if let Some((_, tab)) = self.dock_state.find_active_focused() {
            self.is_viewport_focused = matches!(tab, EditorTab::Viewport);
        } else {
            self.is_viewport_focused = false;
        }

        if matches!(self.editor_state, EditorState::Playing) {
            if self.input_state.pressed_keys.contains(&KeyCode::Escape) {
                self.signal = Signal::StopPlaying;
            }

            let mut script_entities = Vec::new();
            {
                for (entity_id, script) in self.world
                    .query::<&mut ScriptComponent>()
                    .iter()
                {
                    log_once::debug_once!(
                        "Script Entity -> id: {:?}, component: {:?}",
                        entity_id,
                        script
                    );
                    script.name = script
                        .path
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string();
                    script_entities.push((entity_id, script.name.clone()));
                }
            }

            if script_entities.is_empty() {
                log_once::warn_once!("Script entities is empty");
            }

            for (entity_id, script_name) in script_entities {
                if let Err(e) = self.script_manager.update_entity_script(
                    entity_id,
                    &script_name,
                    &mut self.world,
                    &self.input_state,
                    dt,
                ) {
                    log_once::warn_once!(
                        "Failed to update script '{}' for entity {:?}: {}",
                        script_name,
                        entity_id,
                        e
                    );
                }
            }
        }

        if self.is_viewport_focused && matches!(self.viewport_mode, ViewportMode::CameraMove)
        // && self.is_using_debug_camera()
        {
            let movement_keys: std::collections::HashSet<KeyCode> = self
                .input_state
                .pressed_keys
                .iter()
                .filter(|&&key| {
                    matches!(
                        key,
                        KeyCode::KeyW
                            | KeyCode::KeyA
                            | KeyCode::KeyS
                            | KeyCode::KeyD
                            | KeyCode::Space
                            | KeyCode::ShiftLeft
                    )
                })
                .copied()
                .collect();

            let active_cam = self.active_camera.lock();
            if let Some(active_camera) = *active_cam {
                if let Ok(mut query) = self.world
                    .query_one::<(&mut Camera, &CameraComponent)>(active_camera)
                    && let Some((camera, component)) = query.get() {
                        match component.camera_type {
                            CameraType::Debug => {
                                DebugCamera::handle_keyboard_input(camera, &movement_keys);
                                DebugCamera::handle_mouse_input(
                                    camera,
                                    component,
                                    self.input_state.mouse_delta,
                                );
                            }
                            CameraType::Player => {
                                PlayerCamera::handle_keyboard_input(camera, &movement_keys);
                                PlayerCamera::handle_mouse_input(
                                    camera,
                                    component,
                                    self.input_state.mouse_delta,
                                );
                            }
                            CameraType::Normal => {
                                DebugCamera::handle_keyboard_input(camera, &movement_keys);
                                DebugCamera::handle_mouse_input(
                                    camera,
                                    component,
                                    self.input_state.mouse_delta,
                                );
                            }
                        }
                    }
            }

        }

        match self.run_signal(graphics.shared.clone()) {
            Ok(_) => {}
            Err(e) => {
                fatal!("{}", e);
            }
        }

        let current_size = graphics.shared.viewport_texture.size;
        self.size = current_size;

        let new_aspect = current_size.width as f64 / current_size.height as f64;

        {
            let active_cam = self.active_camera.lock();
            if let Some(active_camera) = *active_cam {
                if let Ok(mut query) = self.world.query_one::<&mut Camera>(active_camera)
                    && let Some(camera) = query.get() {
                        camera.aspect = new_aspect;
                    }
            }

        }

        let camera_follow_data: Vec<(Entity, String, glam::Vec3)> = {
            self.world
                .query::<(&Camera, &CameraComponent, Option<&CameraFollowTarget>)>()
                .iter()
                .filter_map(|(entity_id, (_, _, follow_target))| {
                    follow_target.map(|target| {
                        (
                            entity_id,
                            target.follow_target.clone(),
                            target.offset.as_vec3()
                        )
                    })
                })
                .collect()
        };


        for (camera_entity, target_label, offset) in camera_follow_data {
            let target_position = {
                self.world
                    .query::<(&AdoptedEntity, &Transform)>()
                    .iter()
                    .find_map(|(_, (adopted, transform))| {
                        if adopted.model.label == target_label {
                            Some(transform.position)
                        } else {
                            None
                        }
                    })
            };


            if let Some(pos) = target_position {
                if let Ok(mut query) = self.world.query_one::<&mut Camera>(camera_entity)
                    && let Some(camera) = query.get() {
                        camera.eye = pos + offset.as_dvec3();
                        camera.target = pos;
                    }
            }

        }

        {
            for (_entity_id, (camera, component)) in self.world
                .query::<(&mut Camera, &mut CameraComponent)>()
                .iter()
            {
                component.update(camera);
                camera.update(graphics.shared.clone());
            }
        }

        {
            {
                let query = self.world
                    .query_mut::<(&mut AdoptedEntity, &Transform)>();
                for (_, (entity, transform)) in query {
                    entity.update(graphics.shared.clone(), transform);
                }
            }


            {
                let light_query =
                    self.world
                        .query_mut::<(&mut LightComponent, &Transform, &mut Light)>();
                for (_, (light_component, transform, light)) in light_query {
                    light.update(light_component, transform);
                }
            }
            
        }

        {
            self.light_manager.update(
                graphics.shared.clone(),
                &self.world,
            );
        }

        if self.dep_installer.is_installing {
            self.dep_installer
                .show_installation_window(&graphics.shared.get_egui_context());
        }
        self.dep_installer.update_progress();
    }

    fn render(&mut self, graphics: &mut RenderContext) {
        // cornflower blue
        let color = Color {
            r: 100.0 / 255.0,
            g: 149.0 / 255.0,
            b: 237.0 / 255.0,
            a: 1.0,
        };

        self.color = color;
        self.size = graphics.shared.viewport_texture.size;
        self.texture_id = Some(*graphics.shared.texture_id.clone());
        { self.show_ui(&graphics.shared.get_egui_context()); }

        self.window = Some(graphics.shared.window.clone());
        logging::render(&graphics.shared.get_egui_context());
        if let Some(pipeline) = &self.render_pipeline {
            log_once::debug_once!("Found render pipeline");
            if let Some(active_camera) = *self.active_camera.lock() {
                let cam = {
                    if let Ok(mut query) = self.world.query_one::<&Camera>(active_camera) {
                        query.get().cloned()
                    } else {
                        None
                    }
                };

                if let Some(camera) = cam {
                    let lights = {
                        let mut lights = Vec::new();
                        let mut light_query = self.world.query::<(&Light, &LightComponent)>();
                        for (_, (light, comp)) in light_query.iter() {
                            lights.push((light.clone(), comp.clone()));
                        }
                        lights
                    };


                    let entities = {
                        let mut entities = Vec::new();
                        let mut entity_query = self.world.query::<&AdoptedEntity>();
                        for (_, entity) in entity_query.iter() {
                            entities.push(entity.clone());
                        }
                        entities
                    };


                    {
                        let mut render_pass = graphics.clear_colour(color);
                        if let Some(light_pipeline) = &self.light_manager.pipeline {
                            render_pass.set_pipeline(light_pipeline);
                            for (light, _component) in &lights {
                                render_pass.set_vertex_buffer(
                                    1,
                                    light.instance_buffer.as_ref().unwrap().slice(..),
                                );
                                render_pass.draw_light_model(
                                    &light.cube_model,
                                    camera.bind_group(),
                                    light.bind_group(),
                                );
                            }
                        }
                    }

                    let mut model_batches: HashMap<ModelId, Vec<InstanceRaw>> =
                        HashMap::new();
                    for entity in &entities {
                        let model_ptr = entity.model.id;
                        let instance_raw = entity.instance.to_raw();
                        model_batches
                            .entry(model_ptr)
                            .or_default()
                            .push(instance_raw);
                    }

                    for (model_ptr, instances) in model_batches {
                        {
                            let model_opt = {
                                let cache = MODEL_CACHE.lock();
                                cache.values().find(|m| m.id == model_ptr).cloned()
                            };

                            if let Some(model) = model_opt {
                                {
                                    let mut render_pass = graphics.continue_pass();
                                    render_pass.set_pipeline(pipeline);

                                    let instance_buffer = graphics.shared.device.create_buffer_init(
                                        &wgpu::util::BufferInitDescriptor {
                                            label: Some("Batched Instance Buffer"),
                                            contents: bytemuck::cast_slice(&instances),
                                            usage: wgpu::BufferUsages::VERTEX,
                                        },
                                    );
                                    render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
                                    render_pass.draw_model_instanced(
                                        &model,
                                        0..instances.len() as u32,
                                        camera.bind_group(),
                                        self.light_manager.bind_group(),
                                    );
                                }
                                log_once::debug_once!("Rendered {:?}", model_ptr);
                            } else {
                                log_once::error_once!("No such MODEL as {:?}", model_ptr);
                            }
                        }
                    }
                } else {
                    log_once::error_once!("Camera returned None");
                }
            } else {
                log_once::error_once!("No active camera found");
            }
        } else {
            log_once::warn_once!("No render pipeline exists");
        }
    }

    fn exit(&mut self, _event_loop: &ActiveEventLoop) {}

    fn run_command(&mut self) -> SceneCommand {
        std::mem::replace(&mut self.scene_command, SceneCommand::None)
    }
}
