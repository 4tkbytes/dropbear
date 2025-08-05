use std::path::PathBuf;

use dropbear_engine::{
    entity::{AdoptedEntity, Transform},
    graphics::{Graphics, Shader},
    scene::{Scene, SceneCommand},
};
use log;
// use nalgebra::{Point3, Vector3};
use wgpu::Color;
use winit::{event_loop::ActiveEventLoop, keyboard::KeyCode};

use super::*;
use crate::{
    states::{Node, RESOURCES},
    utils::PendingSpawn,
};

pub static PENDING_SPAWNS: LazyLock<Mutex<Vec<PendingSpawn>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

impl Scene for Editor {
    fn load(&mut self, graphics: &mut Graphics) {
        let camera = self.load_project_config(graphics).unwrap();

        let shader = Shader::new(
            graphics,
            include_str!("../shader.wgsl"),
            Some("viewport_shader"),
        );
        if self.world.len() == 0 {
            let cube_path = {
                #[allow(unused_assignments)]
                let mut path = PathBuf::new();
                let resources = RESOURCES.read().unwrap();
                let mut matches = Vec::new();
                crate::utils::search_nodes_recursively(
                    &resources.nodes,
                    &|node| match node {
                        Node::File(file) => file.name.contains("cube"),
                        Node::Folder(folder) => folder.name.contains("cube"),
                    },
                    &mut matches,
                );
                match matches.get(0) {
                    Some(thing) => match thing {
                        Node::File(file) => path = file.path.clone(),
                        Node::Folder(folder) => path = folder.path.clone(),
                    },
                    None => path = PathBuf::new(),
                }
                path
            };

            if cube_path != PathBuf::new() {
                let cube = AdoptedEntity::new(graphics, &cube_path, Some("default_cube")).unwrap();
                // let script = ScriptComponent {
                //     name: "DummyScript".to_string(),
                //     path: PathBuf::from("dummy/path/to/script.rs"),
                // };
                self.world.spawn((
                    cube,
                    Transform::default(),
                    // script
                ));
                log::info!("Added default cube since no entities were loaded from scene");
            } else {
                log::warn!("cube path is empty :(")
            }
        } else {
            log::info!(
                "Scene loaded with {} entities, skipping default cube",
                self.world.len()
            );
        }

        let texture_bind_group = &graphics.texture_bind_group().clone();

        let model_layout = graphics.create_model_uniform_bind_group_layout();
        let pipeline = graphics.create_render_pipline(
            &shader,
            vec![texture_bind_group, camera.layout(), &model_layout],
        );

        self.camera = camera;
        self.render_pipeline = Some(pipeline);
        self.window = Some(graphics.state.window.clone());
    }

    fn update(&mut self, _dt: f32, graphics: &mut Graphics) {
        if let Some((_, tab)) = self.dock_state.find_active_focused() {
            self.is_viewport_focused = matches!(tab, EditorTab::Viewport);
        } else {
            self.is_viewport_focused = false;
        }

        if let Ok(mut pending_spawns) = PENDING_SPAWNS.lock() {
            for spawn in pending_spawns.drain(..) {
                match AdoptedEntity::new(graphics, &spawn.asset_path, Some(&spawn.asset_name)) {
                    Ok(adopted) => {
                        let entity_id = self.world.spawn((adopted, spawn.transform));
                        self.selected_entity = Some(entity_id);
                        log::info!(
                            "Successfully spawned {} with ID {:?}",
                            spawn.asset_name,
                            entity_id
                        );
                    }
                    Err(e) => {
                        log::error!("Failed to spawn {}: {}", spawn.asset_name, e);
                    }
                }
            }
        }

        // if self.is_viewport_focused {
        //     self.is_cursor_locked = true;
        // }

        if self.is_viewport_focused
            && matches!(self.viewport_mode, crate::utils::ViewportMode::CameraMove)
        {
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

        let new_size = graphics.state.viewport_texture.size;
        let new_aspect = new_size.width as f64 / new_size.height as f64;
        self.camera.aspect = new_aspect;

        self.camera.update(graphics);

        let query = self.world.query_mut::<(&mut AdoptedEntity, &Transform)>();
        for (_, (entity, transform)) in query {
            entity.update(&graphics, transform);
        }
    }

    fn render(&mut self, graphics: &mut Graphics) {
        let color = Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };
        self.color = color.clone();
        self.size = graphics.state.viewport_texture.size.clone();
        self.texture_id = Some(graphics.state.texture_id.clone());
        self.show_ui(&graphics.get_egui_context());

        if self.resize_signal.0.clone() {
            // graphics.state.resize(self.resize_signal.1, self.resize_signal.2);
            // self.resize_signal.0 = false;
        }

        self.window = Some(graphics.state.window.clone());
        if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
            toasts.show(graphics.get_egui_context());
        }
        if let Some(pipeline) = &self.render_pipeline {
            {
                let mut query = self.world.query::<(&AdoptedEntity, &Transform)>();
                let mut render_pass = graphics.clear_colour(color);
                render_pass.set_pipeline(pipeline);

                for (_, (entity, _)) in query.iter() {
                    entity.render(&mut render_pass, &self.camera);
                }
            }
        }
    }

    fn exit(&mut self, _event_loop: &ActiveEventLoop) {}

    fn run_command(&mut self) -> SceneCommand {
        std::mem::replace(&mut self.scene_command, SceneCommand::None)
    }
}
