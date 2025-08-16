use std::{any::Any, collections::HashSet};

use dropbear_engine::{camera::Camera, entity::Transform};
use glam::DVec3;
use serde::{Deserialize, Serialize};
use winit::keyboard::KeyCode;

pub trait CameraController: std::fmt::Debug {
    fn update(&mut self, camera: &mut Camera, dt: f32);
    fn handle_keyboard_input(&mut self, camera: &mut Camera, pressed_keys: &HashSet<KeyCode>);
    fn handle_mouse_input(&mut self, camera: &mut Camera, mouse_delta: Option<(f64, f64)>);

    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[derive(Debug)]
pub struct DebugCameraController {
    #[allow(dead_code)]
    pub speed: f64,
    pub sensitivity: f64,
}

impl DebugCameraController {
    pub fn new() -> Self {
        Self {
            speed: 0.125,
            sensitivity: 0.002,
        }
    }
}

impl CameraController for DebugCameraController {
    fn update(&mut self, _camera: &mut Camera, _dt: f32) {
        // Debug camera doesn't need frame-based updates
    }

    fn handle_keyboard_input(
        &mut self,
        camera: &mut Camera,
        pressed_keys: &std::collections::HashSet<KeyCode>,
    ) {
        for key in pressed_keys {
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

    fn handle_mouse_input(&mut self, camera: &mut Camera, mouse_delta: Option<(f64, f64)>) {
        if let Some((dx, dy)) = mouse_delta {
            camera.track_mouse_delta(dx * self.sensitivity, dy * self.sensitivity);
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct PlayerCameraController {
    pub follow_target: Option<hecs::Entity>,
    pub offset: DVec3,
    pub follow_speed: f64,
    pub look_sensitivity: f64,
}

#[allow(dead_code)]
impl PlayerCameraController {
    pub fn new() -> Self {
        Self {
            follow_target: None,
            offset: DVec3::new(0.0, 2.0, -5.0),
            follow_speed: 5.0,
            look_sensitivity: 0.002,
        }
    }

    pub fn set_follow_target(&mut self, entity: hecs::Entity) {
        self.follow_target = Some(entity);
    }

    pub fn clear_follow_target(&mut self) {
        self.follow_target = None;
    }

    pub fn set_offset(&mut self, offset: DVec3) {
        self.offset = offset;
    }

    pub fn get_follow_target(&self) -> Option<hecs::Entity> {
        self.follow_target
    }
}

impl CameraController for PlayerCameraController {
    fn update(&mut self, _camera: &mut Camera, _dt: f32) {
        // todo: implement following the entity
    }

    fn handle_keyboard_input(
        &mut self,
        camera: &mut Camera,
        pressed_keys: &std::collections::HashSet<KeyCode>,
    ) {
        // todo: handle keyboard input, make it custom according to user
        for key in pressed_keys {
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

    fn handle_mouse_input(&mut self, camera: &mut Camera, mouse_delta: Option<(f64, f64)>) {
        if let Some((dx, dy)) = mouse_delta {
            camera.track_mouse_delta(dx * self.look_sensitivity, dy * self.look_sensitivity);
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CameraType {
    Debug,
    Player,
}

#[derive(Debug)]
pub struct CameraManager {
    cameras: HashMap<CameraType, Camera>,
    controllers: HashMap<CameraType, Box<dyn CameraController>>,
    active_camera: CameraType,
}

impl CameraManager {
    pub fn new() -> Self {
        Self {
            cameras: HashMap::new(),
            controllers: HashMap::new(),
            active_camera: CameraType::Debug,
        }
    }

    pub fn add_camera(
        &mut self,
        camera_type: CameraType,
        camera: Camera,
        controller: Box<dyn CameraController>,
    ) {
        self.cameras.insert(camera_type, camera);
        self.controllers.insert(camera_type, controller);
    }

    pub fn set_active(&mut self, camera_type: CameraType) {
        if self.cameras.contains_key(&camera_type) {
            self.active_camera = camera_type;
        }
    }

    pub fn get_active(&self) -> Option<&Camera> {
        self.cameras.get(&self.active_camera)
    }

    pub fn get_active_mut(&mut self) -> Option<&mut Camera> {
        self.cameras.get_mut(&self.active_camera)
    }

    pub fn get_active_type(&self) -> CameraType {
        self.active_camera
    }

    pub fn update_all(&mut self, dt: f32, graphics: &dropbear_engine::graphics::Graphics) {
        for (camera_type, camera) in self.cameras.iter_mut() {
            if let Some(controller) = self.controllers.get_mut(camera_type) {
                controller.update(camera, dt);
            }
            camera.update(graphics);
        }
    }

    pub fn handle_input(
        &mut self,
        pressed_keys: &std::collections::HashSet<KeyCode>,
        mouse_delta: Option<(f64, f64)>,
    ) {
        if let Some(camera) = self.cameras.get_mut(&self.active_camera) {
            if let Some(controller) = self.controllers.get_mut(&self.active_camera) {
                controller.handle_keyboard_input(camera, pressed_keys);
                controller.handle_mouse_input(camera, mouse_delta);
            }
        }
    }

    pub fn update_camera_following(&mut self, world: &hecs::World, _dt: f32) {
        if let Some(player_camera) = self.cameras.get_mut(&CameraType::Player) {
            if let Some(player_controller) = self.controllers.get_mut(&CameraType::Player) {
                if let Some(controller) = player_controller
                    .as_any_mut()
                    .downcast_mut::<PlayerCameraController>()
                {
                    if let Some(target_entity) = controller.follow_target {
                        match world.get::<&Transform>(target_entity) {
                            Ok(transform) => {
                                let target_pos = transform.position + controller.offset;

                                let raw_dir = player_camera.target - player_camera.eye;
                                let look_direction = if raw_dir.length() > 1e-6 {
                                    raw_dir.normalize()
                                } else {
                                    let fwd = player_camera.forward();
                                    if fwd.length().is_finite() && fwd.length() > 1e-6 {
                                        fwd
                                    } else {
                                        glam::DVec3::new(0.0, 0.0, 1.0)
                                    }
                                };

                                player_camera.eye = target_pos;
                                player_camera.target = target_pos + look_direction;
                            }
                            Err(_) => {
                                log_once::error_once!("Follow target entity {:?} not present in world", target_entity);
                            }
                        }
                    } else {
                        log_once::error_once!("Unable to follow camera: No follow target exists in controller");
                    }
                } else {
                    log_once::error_once!("Unable to follow camera: Failed to downcast mut");
                }
            } else {
                log_once::error_once!("Unable to follow camera: No player controller exists");
            }
        } else {
            log_once::error_once!("Unable to follow camera: No camera exists for player");
        }
    }

    #[allow(dead_code)]
    pub fn set_player_camera_target(&mut self, entity: hecs::Entity, offset: DVec3) {
        if let Some(controller) = self.controllers.get_mut(&CameraType::Player) {
            if let Some(player_controller) = controller
                .as_any_mut()
                .downcast_mut::<PlayerCameraController>()
            {
                player_controller.set_follow_target(entity);
                player_controller.set_offset(offset);
            }
        }
    }

    #[allow(dead_code)]
    pub fn clear_player_camera_target(&mut self) {
        if let Some(controller) = self.controllers.get_mut(&CameraType::Player) {
            if let Some(player_controller) = controller
                .as_any_mut()
                .downcast_mut::<PlayerCameraController>()
            {
                player_controller.clear_follow_target();
            }
        }
    }

    #[allow(dead_code)]
    pub fn get_player_camera_target(&self) -> Option<hecs::Entity> {
        if let Some(controller) = self.controllers.get(&CameraType::Player) {
            if let Some(player_controller) =
                controller.as_any().downcast_ref::<PlayerCameraController>()
            {
                return player_controller.get_follow_target();
            }
        }
        None
    }

    pub fn get_camera(&self, camera_type: &CameraType) -> Option<&Camera> {
        self.cameras.get(camera_type)
    }

    #[allow(dead_code)]
    pub fn get_camera_mut(&mut self, camera_type: &CameraType) -> Option<&mut Camera> {
        self.cameras.get_mut(camera_type)
    }

    #[allow(dead_code)]
    pub fn has_camera(&self, camera_type: &CameraType) -> bool {
        self.cameras.contains_key(camera_type)
    }

    #[allow(dead_code)]
    pub fn remove_camera(&mut self, camera_type: &CameraType) -> Option<Camera> {
        self.controllers.remove(camera_type);
        self.cameras.remove(camera_type)
    }

    pub fn clear_cameras(&mut self) {
        self.cameras.clear();
        self.controllers.clear();
    }

    pub fn get_player_camera_offset(&self) -> Option<DVec3> {
        if let Some(controller) = self.controllers.get(&CameraType::Player) {
            if let Some(player_controller) =
                controller.as_any().downcast_ref::<PlayerCameraController>()
            {
                return Some(player_controller.offset);
            }
        }
        None
    }
}

#[derive(Debug)]
pub enum CameraAction {
    SetPlayerTarget { entity: hecs::Entity, offset: DVec3 },
    ClearPlayerTarget,
}
