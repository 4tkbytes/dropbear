pub mod buffer;
pub mod camera;
pub mod egui_renderer;
pub mod entity;
pub mod graphics;
pub mod input;
pub mod model;
pub mod resources;
pub mod scene;

use egui::TextureId;
use egui_wgpu::ScreenDescriptor;
use futures::FutureExt;
use gilrs::{Gilrs, GilrsBuilder};
use spin_sleep::SpinSleeper;
use std::{
    fmt::{self, Display, Formatter},
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    u32,
};
use wgpu::{
    BindGroupLayout, Device, Instance, Queue, Surface, SurfaceConfiguration, TextureFormat,
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::{
    egui_renderer::EguiRenderer,
    graphics::{Graphics, Texture},
};

/// The backend information, such as the device, queue, config, surface, renderer, window and more.
pub struct State {
    pub surface: Surface<'static>,
    pub device: Device,
    pub queue: Queue,
    pub config: SurfaceConfiguration,
    pub is_surface_configured: bool,
    pub depth_texture: Texture,
    pub texture_bind_layout: BindGroupLayout,
    pub egui_renderer: EguiRenderer,
    pub instance: Instance,
    pub viewport_texture: Texture,
    pub texture_id: TextureId,

    pub window: Arc<Window>,
}

impl State {
    /// Asynchronously initialised the state and sets up the backend and surface for wgpu to render to.
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
            .unwrap_or(TextureFormat::Rgba8Unorm);
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
        let viewport_texture =
            Texture::create_viewport_texture(&config, &device, Some("viewport texture"));

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let mut egui_renderer = EguiRenderer::new(&device, config.format, None, 1, &window);

        let texture_id = egui_renderer.renderer().register_native_texture(
            &device,
            &viewport_texture.view,
            wgpu::FilterMode::Linear,
        );

        let result = Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            depth_texture,
            texture_bind_layout: texture_bind_group_layout,
            window,
            instance,
            egui_renderer,
            viewport_texture,
            texture_id,
        };

        Ok(result)
    }

    /// A helper function that changes the surface config when resized (+ depth texture).
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }

        self.depth_texture =
            Texture::create_depth_texture(&self.config, &self.device, Some("depth texture"));
        self.viewport_texture =
            Texture::create_viewport_texture(&self.config, &self.device, Some("viewport texture"));
        self.texture_id = self.egui_renderer.renderer().register_native_texture(
            &self.device,
            &self.viewport_texture.view,
            wgpu::FilterMode::Linear,
        );
    }

    /// Asynchronously renders the scene and the egui renderer. I don't know what else to say.
    async fn render(
        &mut self,
        scene_manager: &mut scene::Manager,
        previous_dt: f32,
        event_loop: &ActiveEventLoop,
    ) -> anyhow::Result<()> {
        if !self.is_surface_configured {
            return Ok(());
        }

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let viewport_view = { &self.viewport_texture.view.clone() };

        self.egui_renderer.begin_frame(&self.window);

        let mut graphics = Graphics::new(self, viewport_view, &mut encoder);

        scene_manager
            .update(previous_dt, &mut graphics, event_loop)
            .await;
        scene_manager.render(&mut graphics).await;

        self.egui_renderer.end_frame_and_draw(
            &self.device,
            &self.queue,
            &mut encoder,
            &self.window,
            &view,
            screen_descriptor,
        );

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

/// Fetches the current time as nanoseconds. Purely just a helper function, but use if you wish.
pub fn get_current_time_as_ns() -> u128 {
    let now = SystemTime::now();
    let duration_since_epoch = now.duration_since(UNIX_EPOCH).unwrap();
    let timestamp_ns = duration_since_epoch.as_nanos();
    timestamp_ns
}

/// A struct storing the information about the application/game that is using the engine.
pub struct App {
    /// The configuration of the window.
    config: WindowConfiguration,
    /// The graphics backend
    state: Option<State>,
    /// The scene manager, manages and orchestrates the switching of scenes
    scene_manager: scene::Manager,
    /// The input manager, manages any inputs and their actions
    input_manager: input::Manager,
    /// The amount of time it took to render the last frame.
    /// To find the FPS: just do `1.0/delta_time`.
    delta_time: f32,
    /// Internal
    next_frame_time: Option<Instant>,
    /// The fps the app should aim to hit / the max fps.
    /// It is possible to aim it at 60 fps, 120 fps, or even no limit
    /// with the const variable [`App::NO_FPS_CAP`]
    target_fps: u32,
    /// The library used for polling controllers, specifically the instance of that.
    gilrs: Gilrs,
}

impl App {
    /// Creates a new instance of the application. It only sets the default for the struct + the
    /// window config.
    fn new(config: WindowConfiguration) -> Self {
        log::debug!("Created new instance of app");
        Self {
            state: None,
            config: config.clone(),
            scene_manager: scene::Manager::new(),
            input_manager: input::Manager::new(),
            delta_time: (1.0 / 60.0),
            next_frame_time: None,
            target_fps: config.max_fps,
            // default settings for now
            gilrs: GilrsBuilder::new().build().unwrap(),
        }
    }

    #[allow(dead_code)]
    /// A constant that lets you not have any fps count.
    /// It is just the max value of an unsigned 32 bit number lol.
    pub const NO_FPS_CAP: u32 = u32::MAX;

    /// Helper function that sets the target frames per second. Can be used mid game to increase FPS.
    pub fn set_target_fps(&mut self, fps: u32) {
        self.target_fps = fps.max(1);
    }

    /// The run function. This function runs the app into gear.
    ///
    /// ## Warning
    /// It is not recommended to use this function to start up the app due to the mandatory app_name
    /// parameter. Use the [`run_app!`] macro instead, which does not require
    /// for you to pass in the app name (it automatically does it for you).
    ///
    /// # Parameters:
    /// - config: The window configuration, such as the title, and window dimensions.
    /// - app_name: A string to the app name for debugging.
    /// - setup: A closure that can initialise the first scenes, such as a menu or the game itself.
    /// It takes an input of a scene manager and an input manager, and expects you to return back the changed
    /// managers.
    pub fn run<F>(config: WindowConfiguration, app_name: &str, setup: F) -> anyhow::Result<()>
    where
        F: FnOnce(scene::Manager, input::Manager) -> (scene::Manager, input::Manager),
    {
        if cfg!(debug_assertions) {
            log::info!("Running in dev mode");
            // let package_name = std::env::var("CARGO_BIN_NAME").unwrap();
            let log_config = format!("dropbear_engine=trace,{}=debug,warn", app_name);
            unsafe { std::env::set_var("RUST_LOG", log_config) };
        }

        env_logger::init();

        // log::debug!("OUT_DIR: {}", std::env!("OUT_DIR"));

        let event_loop = EventLoop::with_user_event().build()?;
        log::debug!("Created new event loop");
        let mut app = Box::new(App::new(config));
        log::debug!("Configured app with details: {}", app.config);

        log::debug!("Running through setup");

        let (new_scene, new_input) = setup(app.scene_manager, app.input_manager);
        app.scene_manager = new_scene;
        app.input_manager = new_input;
        log::debug!("Running app");
        event_loop.run_app(&mut app)?;

        Ok(())
    }
}

#[macro_export]
/// The macro to run the app/game. The difference between this and [`App::run()`] is that
/// this automatically fetches the package name during compilation.
///
/// It is crucial to run with this macro instead of the latter is for debugging purposes (and to make life
/// easier by not having to guess your package name if it changes).
///
/// See also the docs for a further run down on the parameters of how it is run: [`App::run()`]
macro_rules! run_app {
    ($config:expr, $setup:expr) => {
        $crate::App::run($config, env!("CARGO_PKG_NAME"), $setup)
    };
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let mut window_attributes = Window::default_attributes().with_title(self.config.title);

        if self.config.windowed_mode.is_windowed() {
            if let Some((width, height)) = self.config.windowed_mode.windowed_size() {
                window_attributes =
                    window_attributes.with_inner_size(PhysicalSize::new(width, height));
            }
        } else if self.config.windowed_mode.is_maximised() {
            window_attributes = window_attributes.with_maximized(true);
        } else if self.config.windowed_mode.is_fullscreen() {
            window_attributes = window_attributes
                .with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
        }

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

        state.egui_renderer.handle_input(&state.window, &event);

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                let frame_start = Instant::now();

                self.input_manager.update(&mut self.gilrs);
                if let Some(result) = state
                    .render(&mut self.scene_manager, self.delta_time, event_loop)
                    .now_or_never()
                {
                    result.unwrap();
                }

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
                if code == KeyCode::F11 && key_state.is_pressed() {
                    if let Some(state) = &self.state {
                        match self.config.windowed_mode {
                            WindowedModes::Windowed(_, _) => {
                                if state.window.fullscreen().is_some() {
                                    state.window.set_fullscreen(None);
                                    let _ = state
                                        .window
                                        .request_inner_size(PhysicalSize::new(1280, 720));
                                    state.window.set_maximized(false);
                                } else {
                                    state.window.set_fullscreen(Some(
                                        winit::window::Fullscreen::Borderless(None),
                                    ));
                                }
                            }
                            WindowedModes::Maximised => {
                                if state.window.fullscreen().is_some() {
                                    state.window.set_fullscreen(None);
                                    state.window.set_maximized(true);
                                } else {
                                    state.window.set_maximized(false);
                                    state.window.set_fullscreen(Some(
                                        winit::window::Fullscreen::Borderless(None),
                                    ));
                                }
                            }
                            WindowedModes::Fullscreen => {
                                state.window.set_fullscreen(None);
                                let _ = state
                                    .window
                                    .request_inner_size(PhysicalSize::new(1280, 720));
                                state.window.set_maximized(false);
                            }
                        }
                    }
                }
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

/// The window configuration of the app/game.
///
/// This struct is primitive but has purpose in the way that it sets the initial specs of the window.
/// Thats all it does. And it can also display. But thats about it.
#[derive(Debug, Clone)]
pub struct WindowConfiguration {
    pub windowed_mode: WindowedModes,
    pub title: &'static str,
    /// This reads from a config file.
    /// This will read from a client config file under {exe}/client.props, and on game exit will save the properties to the file.
    ///
    /// As of right now, it has not been implemented yet :(
    // TODO: Implement config reading.
    // pub read_from_config: Option<String>,
    pub max_fps: u32,
}

/// An enum displaying the different modes on initial startup
#[derive(PartialEq, Debug, Clone)]
pub enum WindowedModes {
    Windowed(u32, u32),
    Maximised,
    Fullscreen,
}

impl WindowedModes {
    /// Checks if the config is windowed and returns a bool. Use [`WindowedModes::windowed_size`]
    /// to fetch the values.
    pub fn is_windowed(&self) -> bool {
        matches!(self, WindowedModes::Windowed(_, _))
    }

    /// Checks if the config is maximised and returns a bool
    pub fn is_maximised(&self) -> bool {
        matches!(self, WindowedModes::Maximised)
    }

    /// Checks if the config is fullscreen and returns a bool.
    pub fn is_fullscreen(&self) -> bool {
        matches!(self, WindowedModes::Fullscreen)
    }

    /// Fetches the config windowed width and height in an option in the case
    /// that it is run on a mode like fullscreen or maximised.
    pub fn windowed_size(&self) -> Option<(u32, u32)> {
        if let WindowedModes::Windowed(w, h) = *self {
            Some((w, h))
        } else {
            None
        }
    }
}

impl Display for WindowConfiguration {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.windowed_mode.is_windowed() {
            if let Some((width, height)) = self.windowed_mode.windowed_size() {
                write!(
                    f,
                    "width: {}, height: {}, title: {}",
                    width, height, self.title
                )
            } else {
                write!(f, "yo how the fuck you get to here huh???")
            }
        } else if self.windowed_mode.is_maximised() {
            write!(f, "window is maximised: title: {}", self.title)
        } else if self.windowed_mode.is_fullscreen() {
            write!(f, "window is fullscreen: title: {}", self.title)
        } else {
            write!(
                f,
                "dude i think the code is broken can you lowk dm the dev about this thanks!"
            )
        }
    }
}

/// This enum represents the status of any asset, whether its IO, asset rendering,
/// scene loading and more.
///
/// # Representation
/// It's pretty simple really:
///- [`Status::Idle`]: Has not been loaded, and is the default value for anything
///- [`Status::Loading`]: In the process of loading.
///- [`Status::Completed`]: Loading has been completed.
pub enum Status {
    /// Has not been loaded, and is the default value for anything
    Idle,
    /// In the process of loading
    Loading,
    /// Loading has been completed
    Completed,
}
