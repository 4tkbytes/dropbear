use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use dropbear_engine::buffer::Vertex;
use dropbear_engine::camera::Camera;
use dropbear_engine::graphics::{Graphics, Texture, Shader};
use dropbear_engine::nalgebra::{Point3, Vector3};
use dropbear_engine::wgpu::{Buffer, Color, IndexFormat, RenderPipeline};
use dropbear_engine::winit::dpi::PhysicalPosition;
use dropbear_engine::winit::event::MouseButton;
use dropbear_engine::winit::window::Window;
use dropbear_engine::{
    input::{Keyboard, Mouse},
    log::debug,
    scene::Scene,
    winit::{event_loop::ActiveEventLoop, keyboard::KeyCode},
};

pub struct TestingScene1 {
    render_pipeline: Option<RenderPipeline>,
    vertex_buffer: Option<Buffer>,
    index_buffer: Option<Buffer>,
    texture: HashMap<String, Texture>,
    texture_toggle: bool,
    camera: Camera,
    pressed_keys: HashSet<KeyCode>,
    is_cursor_locked: bool,
    window: Option<Arc<Window>>,
}

impl TestingScene1 {
    pub fn new() -> Self {
        debug!("TestingScene1 instance created");
        Self {
            render_pipeline: None,
            vertex_buffer: None,
            index_buffer: None,
            texture: HashMap::new(),
            texture_toggle: false,
            camera: Camera::default(),
            pressed_keys: HashSet::new(),
            is_cursor_locked: true,
            window: None,
        }
    }
}

const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.5,  0.5, 0.0], tex_coords: [0.0, 1.0] },
    Vertex { position: [-0.5, -0.5, 0.0], tex_coords: [0.0, 0.0] },
    Vertex { position: [ 0.5, -0.5, 0.0], tex_coords: [1.0, 0.0] },
    Vertex { position: [ 0.5,  0.5, 0.0], tex_coords: [1.0, 1.0] },
];

const INDICES: &[u16] = &[
    0, 1, 2,
    2, 3, 0,
];

impl Scene for TestingScene1 {
    fn load(&mut self, graphics: &mut Graphics) {
        let shader = Shader::new(
            graphics,
            include_str!("../../dropbear-engine/src/resources/shaders/shader.wgsl"),
            Some("default"),
        );

        self.vertex_buffer = Some(graphics.create_vertex(VERTICES));
        self.index_buffer = Some(graphics.create_index(INDICES));

        let texture1 = Texture::new(graphics, include_bytes!("../../dropbear-engine/src/resources/textures/no-texture.png"));
        let texture2 = Texture::new(graphics, include_bytes!("../../dropbear-engine/src/resources/textures/Autism.png"));

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

        let pipeline = graphics.create_render_pipline(&shader, &texture1.layout, camera.layout.as_ref().unwrap());
        
        // using one of them for now since they are the same
        self.texture.insert("texture1".into(), texture1);
        self.texture.insert("texture2".into(), texture2);
        self.camera = camera;
        
        self.window = Some(graphics.state.window.clone());
        // ensure that this is the last line
        self.render_pipeline = Some(pipeline);
    }

    fn update(&mut self, _dt: f32, graphics: &mut Graphics) {
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
        if !self.is_cursor_locked {self.window.as_mut().unwrap().set_cursor_visible(true);}
        self.camera.update(graphics);
    }

    fn render(&mut self, graphics: &mut Graphics) {
        let color = Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };
        let mut render_pass = graphics.clear_colour(color);

        if let Some(pipeline) = &self.render_pipeline {
            render_pass.set_pipeline(pipeline);

            if self.texture_toggle {
                render_pass.set_bind_group(0, &self.texture.get("texture1").as_ref().unwrap().bind_group, &[]);
            } else {
                render_pass.set_bind_group(0, &self.texture.get("texture2").as_ref().unwrap().bind_group, &[]);
            }

            render_pass.set_bind_group(1, &self.camera.bind_group, &[]);

            render_pass.set_vertex_buffer(0, self.vertex_buffer.as_ref().unwrap().slice(..));
            render_pass.set_index_buffer(
                self.index_buffer.as_ref().unwrap().slice(..),
                IndexFormat::Uint16,
            );
            render_pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1);
        }
        self.window = Some(graphics.state.window.clone());
    }

    fn exit(&mut self) {
        debug!("TestingScene1 exited!");
    }
}

impl Keyboard for TestingScene1 {
    fn key_down(&mut self, key: KeyCode, event_loop: &ActiveEventLoop) {
        // debug!("Key pressed: {:?}", key);
        match key {
            KeyCode::Escape => event_loop.exit(),
            KeyCode::Slash => {
                self.texture_toggle = !self.texture_toggle;
                debug!("New: {}", self.texture_toggle);
            },
            KeyCode::F1 => {
                self.is_cursor_locked = !self.is_cursor_locked
            }
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
                let center = PhysicalPosition::new(size.width as f64 / 2.0, size.height as f64 / 2.0);

                let dx = position.x - center.x;
                let dy = position.y - center.y;
                self.camera.track_mouse_delta(dx as f32, dy as f32);

                window.set_cursor_position(center).ok();
                window.set_cursor_visible(false);
            }
        }
    }
}
