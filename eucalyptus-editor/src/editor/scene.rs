
use super::*;
use dropbear_engine::graphics::{InstanceRaw, RenderContext};
use dropbear_engine::model::MODEL_CACHE;
use dropbear_engine::{
    entity::{AdoptedEntity, Transform},
    lighting::{Light, LightComponent},
    model::{DrawLight, DrawModel},
    scene::{Scene, SceneCommand},
};
use eucalyptus_core::states::{WorldLoadingStatus};
use eucalyptus_core::{logging};
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
        
        let dock_state_shared = Arc::new(Mutex::new(self.dock_state.clone()));
        let dock_state_for_loading = dock_state_shared.clone();

        let handle = graphics.shared.future_queue.push(async move {
            let mut temp_world = World::new();
            if let Err(e) = Self::load_project_config(graphics_shared, Some(tx), &mut temp_world, Some(tx2), active_camera_clone, project_path_clone, dock_state_for_loading).await {
                log::error!("Failed to load project config: {}", e);
            }
        });

        self.world_load_handle = Some(handle);
        
        self.dock_state_shared = Some(dock_state_shared);

        self.window = Some(graphics.shared.window.clone());
        self.is_world_loaded.mark_scene_loaded();
    }

    fn update(&mut self, dt: f32, graphics: &mut RenderContext) {
        if let Some(mut receiver) = self.world_receiver.take() {
            self.show_project_loading_window(&graphics.shared.get_egui_context());
            if let Ok(loaded_world) = receiver.try_recv() {
                self.world = Box::new(loaded_world);
                self.is_world_loaded.mark_project_loaded();
                
                if let Some(dock_state_shared) = &self.dock_state_shared &&
                let Some(loaded_dock_state) = dock_state_shared.try_lock() {
                    self.dock_state = loaded_dock_state.clone();
                    log::info!("Dock state updated from loaded config");
                }
                
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
                        "Script Entity -> id: {:?}, tags: {:?}",
                        entity_id,
                        script.tags
                    );
                    script_entities.push((entity_id, script.tags.clone()));
                }
            }

            if script_entities.is_empty() {
                log_once::warn_once!("Script entities is empty");
            }

            for (entity_id, script_name) in script_entities {
                if let Err(e) = self.script_manager.update_entity_script(
                    entity_id,
                    script_name.clone(),
                    &mut self.world,
                    &self.input_state,
                    dt,
                ) {
                    log_once::warn_once!(
                        "Failed to update script for entity {:?}: {}",
                        entity_id,
                        e
                    );
                }
            }
        }

        if self.is_viewport_focused && matches!(self.viewport_mode, ViewportMode::CameraMove)
        // && self.is_using_debug_camera()
        {
            let active_cam = self.active_camera.lock();
            if let Some(active_camera) = *active_cam &&
                let Ok(mut query) = self.world
                    .query_one::<(&mut Camera, &CameraComponent)>(active_camera)
                    &&
                let Some((camera, _)) = query.get()
            {
                for key in &self.input_state.pressed_keys {
                    match key {
                        KeyCode::KeyW => camera.move_forwards(),
                        KeyCode::KeyA => camera.move_left(),
                        KeyCode::KeyD => camera.move_right(),
                        KeyCode::KeyS => camera.move_back(),
                        KeyCode::ShiftLeft => camera.move_down(),
                        KeyCode::Space => camera.move_up(),
                        _ => {}
                    }
                }
            }
        }

        let _ = self.run_signal(graphics.shared.clone());

        let current_size = graphics.shared.viewport_texture.size;
        self.size = current_size;

        let new_aspect = current_size.width as f64 / current_size.height as f64;

        {
            let active_cam = self.active_camera.lock();
            if let Some(active_camera) = *active_cam 
            && let Ok(mut query) = self.world.query_one::<&mut Camera>(active_camera)
            && let Some(camera) = query.get() {
                camera.aspect = new_aspect;
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
