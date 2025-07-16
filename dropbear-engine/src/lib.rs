pub mod model;
pub mod buffer;
pub mod camera;
pub mod entity;
pub mod graphics;
pub mod input;
pub mod scene;

pub use bytemuck;
pub use log;
pub use nalgebra;
pub use num_traits;
pub use wgpu;
pub use winit;

use spin_sleep::SpinSleeper;
use std::{
    fmt::{self, Display, Formatter}, sync::Arc, time::{Duration, Instant, SystemTime, UNIX_EPOCH}, u32
};
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::PhysicalKey,
    window::Window,
};

use crate::graphics::{Graphics, Texture};

pub struct State {
    pub surface: Surface<'static>,
    pub device: Device,
    pub queue: Queue,
    pub config: SurfaceConfiguration,
    pub is_surface_configured: bool,
    pub depth_texture: Texture,
    pub window: Arc<Window>,
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

        let info = adapter.get_info();
println!(
    "==================== BACKEND INFO ====================
Backend: {}

Hardware:
    Adapter Name: {}
    Vendor: {}
    Device: {}
    Type: {:?}
    Driver: {}
    Driver Info: {}

",
    info.backend.to_string(),
    info.name,
    info.vendor,
    info.device,
    info.device_type,
    info.driver,
    info.driver_info,
);
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

        let depth_texture = Texture::create_depth_texture(&config, &device, Some("depth texture"));

        let result = Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            depth_texture,
            window,
        };

        Ok(result)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }

        self.depth_texture = Texture::create_depth_texture(&self.config, &self.device, Some("depth texture"));
    }

    fn render(
        &mut self,
        scene_manager: &mut scene::Manager,
        previous_dt: f32,
    ) -> anyhow::Result<()> {
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

        scene_manager.update(previous_dt, &mut graphics);
        scene_manager.render(&mut graphics);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

pub fn get_current_time_as_ns() -> u128 {
    let now = SystemTime::now();
    let duration_since_epoch = now.duration_since(UNIX_EPOCH).unwrap();
    let timestamp_ns = duration_since_epoch.as_nanos();
    timestamp_ns
}

pub struct App {
    config: WindowConfiguration,
    state: Option<State>,
    scene_manager: scene::Manager,
    input_manager: input::Manager,
    delta_time: f32,
    next_frame_time: Option<Instant>,
    target_fps: u32,
}

impl App {
    pub fn new(config: WindowConfiguration) -> Self {
        log::debug!("Created new instance of app");
        Self {
            state: None,
            config,
            scene_manager: scene::Manager::new(),
            input_manager: input::Manager::new(),
            delta_time: (1.0 / 60.0),
            next_frame_time: None,
            target_fps: 60,
        }
    }

    #[allow(dead_code)]
    const NO_FPS_CAP: u32 = u32::MAX;

    pub fn set_target_fps(&mut self, fps: u32) {
        self.target_fps = fps.max(1);
    }

    pub fn run<F>(config: WindowConfiguration, app_name: &str, setup: F) -> anyhow::Result<()>
    where
        F: FnOnce(&mut scene::Manager, &mut input::Manager),
    {
        if cfg!(debug_assertions) {
            log::info!("Running in dev mode");
            // let package_name = std::env::var("CARGO_BIN_NAME").unwrap();
            let log_config = format!("dropbear_engine=trace,{}=debug,warn", app_name);
            unsafe { std::env::set_var("RUST_LOG", log_config) };
        }

        env_logger::init();

        let event_loop = EventLoop::with_user_event().build()?;
        log::debug!("Created new event loop");
        let mut app = App::new(config);
        log::debug!("Configured app with details: {}", app.config);
        
        log::debug!("Running through setup");
        setup(&mut app.scene_manager, &mut app.input_manager);

        log::debug!("Running app");
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

        self.next_frame_time = Some(Instant::now());
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
                let frame_start = Instant::now();

                self.input_manager.update();
                state
                    .render(&mut self.scene_manager, self.delta_time)
                    .unwrap();

                let frame_elapsed = frame_start.elapsed();
                let target_frame_time = Duration::from_secs_f32(1.0 / self.target_fps as f32);

                if frame_elapsed < target_frame_time {
                    SpinSleeper::default().sleep(target_frame_time - frame_elapsed);
                }

                let total_frame_time = frame_start.elapsed();
                self.delta_time = total_frame_time.as_secs_f32();

                if self.delta_time > 0.0 {
                    let fps = (1.0 / self.delta_time).round() as u32;
                    let new_title = format!("{} | FPS: {}", self.config.title, fps);
                    state.window.set_title(&new_title);
                }

                state.window.request_redraw();
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

#[derive(Debug)]
pub struct WindowConfiguration {
    pub width: u32,
    pub height: u32,
    pub title: &'static str,
}

impl Display for WindowConfiguration {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "width: {}, height: {}, title: {}", self.width, self.height, self.title)
    }
}


