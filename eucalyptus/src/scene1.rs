use std::collections::HashSet;
use std::sync::Arc;

use dropbear_engine::async_trait::async_trait;
use dropbear_engine::camera::Camera;
use dropbear_engine::entity::{AdoptedEntity, Transform};
use dropbear_engine::graphics::{Graphics, Shader};
use dropbear_engine::hecs::World;
use dropbear_engine::input::Controller;
use dropbear_engine::nalgebra::{Point3, Vector3};
use dropbear_engine::scene::SceneCommand;
use dropbear_engine::wgpu::{Color, RenderPipeline};
use dropbear_engine::winit::dpi::PhysicalPosition;
use dropbear_engine::winit::event::MouseButton;
use dropbear_engine::winit::window::Window;
use dropbear_engine::{gilrs, hecs};
use dropbear_engine::{
    input::{Keyboard, Mouse},
    log::debug,
    scene::Scene,
    winit::{event_loop::ActiveEventLoop, keyboard::KeyCode},
};

pub struct TestingScene1 {
    world: hecs::World,
    render_pipeline: Option<RenderPipeline>,
    camera: Camera,
    pressed_keys: HashSet<KeyCode>,
    is_cursor_locked: bool,
    window: Option<Arc<Window>>,
    scene_command: SceneCommand,
}

impl TestingScene1 {
    pub fn new() -> Self {
        debug!("TestingScene1 instance created");
        Self {
            world: World::new(),
            is_cursor_locked: true,
            render_pipeline: None,
            camera: Camera::default(),
            pressed_keys: Default::default(),
            window: Default::default(),
            scene_command: Default::default(),
        }
    }
}

#[async_trait]
impl Scene for TestingScene1 {
    async fn load(&mut self, graphics: &mut Graphics) {
        let shader = Shader::new(
            graphics,
            include_str!("../../dropbear-engine/resources/shaders/shader.wgsl"),
            Some("default"),
        );

        let horse_model =
            AdoptedEntity::new(graphics, "models/low_poly_horse.glb", Some("horse")).unwrap();

        self.world.spawn((horse_model, Transform::default()));

        let camera = Camera::new(
            graphics,
            Point3::new(0.0, 1.0, 2.0),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::y(),
            (graphics.state.config.width / graphics.state.config.height) as f32,
            45.0,
            0.1,
            100.0,
            0.125,
            0.002,
        );

        let pipeline = graphics.create_render_pipline(
            &shader,
            vec![&graphics.state.texture_bind_layout, camera.layout()],
        );

        self.camera = camera;
        self.window = Some(graphics.state.window.clone());

        // ensure that this is the last line
        self.render_pipeline = Some(pipeline);
    }

    async fn update(&mut self, _dt: f32, graphics: &mut Graphics) {
        // hold down movement
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

        graphics.state.surface.get_current_texture();

        if !self.is_cursor_locked {
            self.window.as_mut().unwrap().set_cursor_visible(true);
        }

        let query = self.world.query_mut::<(&mut AdoptedEntity, &Transform)>();
        for (_, (entity, transform)) in query {
            entity.update(&graphics, transform);
        }

        self.camera.update(graphics);
    }

    async fn render(&mut self, graphics: &mut Graphics) {
        let color = Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };

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

        self.window = Some(graphics.state.window.clone());
    }

    async fn exit(&mut self, _event_loop: &ActiveEventLoop) {}

    fn run_command(&mut self) -> SceneCommand {
        std::mem::replace(&mut self.scene_command, SceneCommand::None)
    }
}

impl Keyboard for TestingScene1 {
    fn key_down(&mut self, key: KeyCode, event_loop: &ActiveEventLoop) {
        // debug!("Key pressed: {:?}", key);
        match key {
            KeyCode::Escape => event_loop.exit(),
            KeyCode::F1 => self.is_cursor_locked = !self.is_cursor_locked,
            // KeyCode::F2 => self.switch_to = Some("testing_scene_2".into()),
            _ => {
                self.pressed_keys.insert(key);
            }
        }
    }

    fn key_up(&mut self, key: KeyCode, _event_loop: &ActiveEventLoop) {
        // debug!("Key released: {:?}", key);
        self.pressed_keys.remove(&key);
    }
}

impl Mouse for TestingScene1 {
    fn mouse_down(&mut self, _button: MouseButton) {
        // debug!("Mouse button pressed: {:?}", button)
    }

    fn mouse_up(&mut self, _button: MouseButton) {
        // debug!("Mouse button released: {:?}", button);
    }

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
}

impl Controller for TestingScene1 {
    fn button_down(&mut self, button: gilrs::Button, id: gilrs::GamepadId) {
        debug!("Controller button {:?} pressed! [{}]", button, id);
    }

    fn button_up(&mut self, button: gilrs::Button, id: gilrs::GamepadId) {
        debug!("Controller button {:?} released! [{}]", button, id);
    }

    fn left_stick_changed(&mut self, x: f32, y: f32, id: gilrs::GamepadId) {
        debug!("Left stick changed: x = {} | y = {} | id = {}", x, y, id);
    }

    fn right_stick_changed(&mut self, x: f32, y: f32, id: gilrs::GamepadId) {
        debug!("Right stick changed: x = {} | y = {} | id = {}", x, y, id);
    }

    fn on_connect(&mut self, id: gilrs::GamepadId) {
        debug!("Controller connected [{}]", id);
    }

    fn on_disconnect(&mut self, id: gilrs::GamepadId) {
        debug!("Controller disconnected [{}]", id);
    }
}
