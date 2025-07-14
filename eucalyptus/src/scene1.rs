use dropbear_engine::buffer::Vertex;
use dropbear_engine::graphics::Graphics;
use dropbear_engine::{
    input::{Keyboard, Mouse},
    log::debug,
    scene::Scene,
    winit::{
        event_loop::ActiveEventLoop,
        keyboard::KeyCode,
    },
};
use dropbear_engine::wgpu::{Buffer, Color, RenderPipeline};

pub struct TestingScene1 {
    render_pipeline: Option<RenderPipeline>,
    vertex_buffer: Option<Buffer>,
}

impl TestingScene1 {
    pub fn new() -> Self {
        Self {
            render_pipeline: None,
            vertex_buffer: None,
        }
    }
}

const VERTICES: &[Vertex] = &[
    Vertex { position: [0.0, 0.5, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [-0.5, -0.5, 0.0], color: [0.0, 1.0, 0.0] },
    Vertex { position: [0.5, -0.5, 0.0], color: [0.0, 0.0, 1.0] },
];

impl Scene for TestingScene1 {
    fn load(&mut self, graphics: &mut Graphics) {
        let shader = graphics.new_shader(
            include_str!("../../dropbear-engine/src/resources/shaders/shader.wgsl"),
            Some("default"),
        );
        let pipeline = graphics.start_rendering(&shader);
        self.render_pipeline = Some(pipeline);

        self.vertex_buffer = Some(graphics.create_buffer(VERTICES));
    }

    fn update(&mut self, _dt: f32) {
        // Scene update logic here
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
            render_pass.draw(0..3, 0..1);
        }
    }

    fn exit(&mut self) {
        debug!("TestingScene1 exited!");
    }
}

impl Keyboard for TestingScene1 {
    fn key_down(&mut self, key: KeyCode, event_loop: &ActiveEventLoop) {
        debug!("Key pressed: {:?}", key);
        match key {
            KeyCode::Escape => event_loop.exit(),
            _ => {}
        }
    }

    fn key_up(&mut self, key: KeyCode, _event_loop: &ActiveEventLoop) {
        debug!("Key released: {:?}", key);
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
