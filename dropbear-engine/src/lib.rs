pub mod graphics;
pub mod input;
pub mod scene;

pub use log;
use log::warn;
pub use wgpu;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
pub use winit;

use std::sync::Arc;
use std::thread::current;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::graphics::Graphics;
use crate::scene::SceneImpl;

pub struct State {
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    is_surface_configured: bool,
    window: Arc<Window>,
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        // create backend
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            window,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }
    }

    fn update(&mut self) {}

    fn render(&mut self, scene_manager: &mut scene::Manager) -> anyhow::Result<()> {
        self.window.request_redraw();

        if !self.is_surface_configured {
            return Ok(());
        }

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let mut graphics = Graphics::new(self, &view, &mut encoder);
        scene_manager.render(&mut graphics);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

pub struct App {
    config: WindowConfiguration,
    state: Option<State>,
    scene_manager: scene::Manager,
    input_manager: input::Manager,
}

impl App {
    pub fn new(config: WindowConfiguration) -> Self {
        Self {
            state: None,
            config,
            scene_manager: scene::Manager::new(),
            input_manager: input::Manager::new(),
        }
    }

    pub fn run<F>(config: WindowConfiguration, app_name: &str, setup: F) -> anyhow::Result<()>
    where
        F: FnOnce(&mut scene::Manager, &mut input::Manager),
    {
        if cfg!(debug_assertions) {
            // let package_name = std::env::var("CARGO_BIN_NAME").unwrap();
            let log_config = format!("dropbear_engine=debug,{}=debug,warn", app_name);
            unsafe { std::env::set_var("RUST_LOG", log_config) };
        }

        env_logger::init();

        let event_loop = EventLoop::with_user_event().build()?;
        let mut app = App::new(config);

        setup(&mut app.scene_manager, &mut app.input_manager);

        event_loop.run_app(&mut app)?;

        Ok(())
    }
}

#[macro_export]
macro_rules! run_app {
    ($config:expr, $setup:expr) => {
        $crate::App::run($config, env!("CARGO_PKG_NAME"), $setup)
    };
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes()
            .with_title(self.config.title)
            .with_inner_size(PhysicalSize::new(self.config.width, self.config.height));

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        self.state = Some(pollster::block_on(State::new(window)).unwrap());
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                self.scene_manager.update(0.016); // todo: get update to be calculated properly
                // self.scene_manager.render();

                self.input_manager.update();

                state.render(&mut self.scene_manager).unwrap();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => {
                self.input_manager
                    .handle_key_input(code, key_state.is_pressed(), event_loop);
            }
            WindowEvent::MouseInput {
                button,
                state: button_state,
                ..
            } => {
                self.input_manager
                    .handle_mouse_input(button, button_state.is_pressed());
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.input_manager.handle_mouse_movement(position);
            }
            _ => {}
        }
    }
}

pub struct WindowConfiguration {
    pub width: u32,
    pub height: u32,
    pub title: &'static str,
}
