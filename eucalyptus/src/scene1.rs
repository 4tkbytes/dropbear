use std::collections::HashMap;

use dropbear_engine::buffer::Vertex;
use dropbear_engine::graphics::{Graphics, Texture, Shader};
use dropbear_engine::wgpu::{Buffer, Color, IndexFormat, RenderPipeline};
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
        
        // using one of them for now
        let pipeline = graphics.create_render_pipline(&shader, vec![&texture1]);
        

        self.texture.insert("texture1".into(), texture1);
        self.texture.insert("texture2".into(), texture2);

        // ensure that this is the last line
        self.render_pipeline = Some(pipeline);
    }

    fn update(&mut self, _dt: f32) {}

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
            debug!("texture_toggle: {}", self.texture_toggle);
            if self.texture_toggle {
                debug!("Binding texture1");
                render_pass.set_bind_group(0, &self.texture.get("texture1").as_ref().unwrap().bind_group, &[]);
            } else {
                debug!("Binding texture2");
                render_pass.set_bind_group(0, &self.texture.get("texture2").as_ref().unwrap().bind_group, &[]);
            }
            render_pass.set_vertex_buffer(0, self.vertex_buffer.as_ref().unwrap().slice(..));
            render_pass.set_index_buffer(
                self.index_buffer.as_ref().unwrap().slice(..),
                IndexFormat::Uint16,
            );
            render_pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1);
        }
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
            KeyCode::Space => {
                self.texture_toggle = !self.texture_toggle;
                debug!("New: {}", self.texture_toggle);
            }
            _ => {}
        }
    }

    fn key_up(&mut self, _key: KeyCode, _event_loop: &ActiveEventLoop) {
        // debug!("Key released: {:?}", key);
    }
}

impl Mouse for TestingScene1 {
    fn mouse_down(&mut self, button: dropbear_engine::winit::event::MouseButton) {
        debug!("Mouse button pressed: {:?}", button)
    }

    fn mouse_up(&mut self, button: dropbear_engine::winit::event::MouseButton) {
        debug!("Mouse button released: {:?}", button);
    }

    fn mouse_move(&mut self, position: dropbear_engine::winit::dpi::PhysicalPosition<f64>) {
        debug!("Mouse position: {}, {}", position.x, position.y)
    }
}
