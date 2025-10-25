pub mod attenuation;
pub mod buffer;
pub mod camera;
pub mod egui_renderer;
pub mod entity;
pub mod graphics;
pub mod input;
pub mod lighting;
pub mod model;
pub mod panic;
pub mod procedural;
pub mod resources;
pub mod scene;
pub mod utils;

use app_dirs2::{AppDataType, AppInfo};
use bytemuck::Contiguous;
use chrono::Local;
use colored::Colorize;
use egui::TextureId;
use egui_wgpu::ScreenDescriptor;
use env_logger::Builder;
use futures::executor::block_on;
use gilrs::{Gilrs, GilrsBuilder};
use parking_lot::Mutex;
use spin_sleep::SpinSleeper;
use std::fs::OpenOptions;
use std::io::Write;
use std::{
    fmt::{self, Display, Formatter},
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use wgpu::{
    BindGroupLayout, Device, Instance, Queue, Surface, SurfaceConfiguration, SurfaceError,
    TextureFormat,
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::{egui_renderer::EguiRenderer, graphics::Texture};

pub use dropbear_future_queue as future;
use dropbear_future_queue::FutureQueue;
pub use gilrs;
use log::LevelFilter;
pub use wgpu;
pub use winit;
use winit::event::{DeviceEvent, DeviceId};

/// The backend information, such as the device, queue, config, surface, renderer, window and more.
pub struct State {
    pub surface: Surface<'static>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub config: SurfaceConfiguration,
    pub is_surface_configured: bool,
    pub depth_texture: Texture,
    pub texture_bind_layout: BindGroupLayout,
    pub egui_renderer: Arc<Mutex<EguiRenderer>>,
    pub instance: Instance,
    pub viewport_texture: Texture,
    pub texture_id: Arc<TextureId>,
    pub future_queue: Arc<FutureQueue>,

    pub window: Arc<Window>, // note to self: functions can only be called in the main thread
}

impl State {
    /// Asynchronously initialised the state and sets up the backend and surface for wgpu to render to.
    pub async fn new(window: Arc<Window>, future_queue: Arc<FutureQueue>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        // create backend
        let instance = Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            // flags: wgpu::InstanceFlags::empty(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone())?;

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
        let os_info = os_info::get();
        log::info!(
            "\n==================== BACKEND INFO ====================
Backend: {}

Software:
    Architecture: {:?}
    Bitness: {:?}
    Codename: {:?}
    Edition: {:?}
    Os Type: {:?}
    Version: {:?}
    TLDR: {}


Hardware:
    Adapter Name: {}
    Vendor: {}
    Device: {}
    Type: {:?}
    Driver: {}
    Driver Info: {}
=======================================================
",
            info.backend,
            os_info.architecture(),
            os_info.bitness(),
            os_info.codename(),
            os_info.edition(),
            os_info.os_type(),
            os_info.version(),
            os_info,
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
        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

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

        let mut egui_renderer = Arc::new(Mutex::new(EguiRenderer::new(
            &device,
            config.format,
            None,
            1,
            &window,
        )));

        let texture_id = Arc::get_mut(&mut egui_renderer)
            .unwrap()
            .lock()
            .renderer()
            .register_native_texture(&device, &viewport_texture.view, wgpu::FilterMode::Linear);

        let result = Self {
            surface,
            device: Arc::new(device),
            queue: Arc::new(queue),
            config,
            is_surface_configured: true,
            depth_texture,
            texture_bind_layout: texture_bind_group_layout,
            window,
            instance,
            egui_renderer,
            viewport_texture,
            texture_id: Arc::new(texture_id),
            future_queue,
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
        self.egui_renderer
            .lock()
            .renderer()
            .update_egui_texture_from_wgpu_texture(
                &self.device,
                &self.viewport_texture.view,
                wgpu::FilterMode::Linear,
                *self.texture_id,
            );
    }

    /// Renders the scene and the egui renderer. I don't know what else to say.
    fn render(
        &mut self,
        scene_manager: &mut scene::Manager,
        previous_dt: f32,
        event_loop: &ActiveEventLoop,
    ) -> anyhow::Result<()> {
        if !self.is_surface_configured {
            return Ok(());
        }

        let output = match self.surface.get_current_texture() {
            Ok(val) => val,
            Err(e) => {
                return match e {
                    SurfaceError::Lost => {
                        log_once::warn_once!("Surface lost, reconfiguring...");
                        self.surface.configure(&self.device, &self.config);
                        Ok(())
                    }
                    SurfaceError::Outdated => {
                        log_once::warn_once!("Surface outdated, reconfiguring...");
                        self.surface.configure(&self.device, &self.config);
                        Ok(())
                    }
                    SurfaceError::Timeout => {
                        log_once::warn_once!("Surface timeout, skipping frame");
                        Ok(())
                    }
                    SurfaceError::OutOfMemory => {
                        Err(anyhow::anyhow!("Surface out of memory: {:?}", e))
                    }
                    SurfaceError::Other => {
                        log_once::warn_once!("Surface error (Other): {:?}, skipping frame", e);
                        Ok(())
                    }
                };
            }
        };

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let viewport_view = { &self.viewport_texture.view.clone() };

        self.egui_renderer.lock().begin_frame(&self.window);

        let mut graphics = graphics::RenderContext::from_state(self, viewport_view, &mut encoder);

        scene_manager.update(previous_dt, &mut graphics, event_loop);
        scene_manager.render(&mut graphics);

        self.egui_renderer.lock().end_frame_and_draw(
            &self.device,
            &self.queue,
            &mut encoder,
            &self.window,
            &view,
            screen_descriptor,
        );

        let command_buffer = encoder.finish();

        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.queue.submit(std::iter::once(command_buffer));
        })) {
            Ok(_) => {}
            Err(_) => {
                log::error!("Failed to submit command buffer, device may be lost");
                return Err(anyhow::anyhow!("Command buffer submission failed"));
            }
        }

        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            output.present();
        })) {
            Ok(_) => {}
            Err(_) => {
                log::error!("Failed to present frame, surface may be lost");
                return Err(anyhow::anyhow!("Frame presentation failed"));
            }
        }

        Ok(())
    }
}

/// Fetches the current time as nanoseconds. Purely just a helper function, but use if you wish.
pub fn get_current_time_as_ns() -> u128 {
    let now = SystemTime::now();
    let duration_since_epoch = now.duration_since(UNIX_EPOCH).unwrap();
    duration_since_epoch.as_nanos()
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
    /// A queue that polls through futures for asynchronous functions
    ///
    /// Winit doesn't use async, so this is the next best alternative.
    future_queue: Arc<FutureQueue>,
    delta_position: Option<(f64, f64)>,
}

impl App {
    /// Creates a new instance of the application. It only sets the default for the struct + the
    /// window config.
    fn new(config: WindowConfiguration, future_queue: Option<Arc<FutureQueue>>) -> Self {
        let result = Self {
            state: None,
            config: config.clone(),
            scene_manager: scene::Manager::new(),
            input_manager: input::Manager::new(),
            delta_time: 1.0 / 60.0,
            next_frame_time: None,
            target_fps: config.max_fps,
            // default settings for now
            gilrs: GilrsBuilder::new().build().unwrap(),
            future_queue: future_queue.unwrap_or_else(|| Arc::new(FutureQueue::new())),
            delta_position: None,
        };
        log::debug!("Created new instance of app");
        result
    }

    /// A constant that lets you not have any fps count.
    /// It is just the max value of an unsigned 32 bit number lol.
    pub const NO_FPS_CAP: u32 = u32::MAX_VALUE;

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
    ///
    /// It takes an input of a scene manager and an input manager, and expects you to return back the changed
    /// managers.
    pub async fn run<F>(
        config: WindowConfiguration,
        app_name: &str,
        future_queue: Option<Arc<FutureQueue>>,
        setup: F,
    ) -> anyhow::Result<()>
    where
        F: FnOnce(scene::Manager, input::Manager) -> (scene::Manager, input::Manager),
    {
        let log_dir = app_dirs2::app_root(AppDataType::UserData, &config.app_info)
            .expect("Failed to get app data directory")
            .join("logs");
        std::fs::create_dir_all(&log_dir).expect("Failed to create log dir");

        let datetime_str = Local::now().format("%Y-%m-%d_%H-%M-%S");
        let log_filename = format!("{}.{}.log", app_name, datetime_str);
        let log_path = log_dir.join(log_filename);

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .expect("Failed to open log file");
        let file = Mutex::new(file);

        let app_target = app_name.replace('-', "_");
        let log_config = format!("dropbear_engine=trace,{}=debug,warn", app_target);
        unsafe { std::env::set_var("RUST_LOG", log_config) };

        #[cfg(not(target_os = "android"))]
        {
            Builder::new()
                .format(move |buf, record| {
                    let ts = Local::now().format("%Y-%m-%dT%H:%M:%S");

                    let colored_level = match record.level() {
                        log::Level::Error => record.level().to_string().red().bold(),
                        log::Level::Warn => record.level().to_string().yellow().bold(),
                        log::Level::Info => record.level().to_string().green().bold(),
                        log::Level::Debug => record.level().to_string().blue().bold(),
                        log::Level::Trace => record.level().to_string().cyan().bold(),
                    };

                    let colored_timestamp = ts.to_string().bright_black();

                    let file_info = format!(
                        "{}:{}",
                        record.file().unwrap_or("unknown"),
                        record.line().unwrap_or(0)
                    )
                    .bright_black();

                    let console_line = format!(
                        "{} {} [{}] - {}\n",
                        file_info,
                        colored_timestamp,
                        colored_level,
                        record.args()
                    );

                    let file_line = format!(
                        "{}:{} {} [{}] - {}\n",
                        record.file().unwrap_or("unknown"),
                        record.line().unwrap_or(0),
                        ts,
                        record.level(),
                        record.args()
                    );

                    write!(buf, "{}", console_line)?;

                    let mut fh = file.lock();
                    let _ = fh.write_all(file_line.as_bytes());

                    Ok(())
                })
                .filter(Some("dropbear_engine"), LevelFilter::Trace)
                .filter(
                    Some(app_name.replace('-', "_").as_str()),
                    LevelFilter::Debug,
                )
                .filter(Some("eucalyptus_core"), LevelFilter::Debug)
                .init();

            // setup panic
            panic::set_hook();
        }

        // log::debug!("OUT_DIR: {}", std::env!("OUT_DIR"));
        log::info!("======================================================================");
        log::info!(
            "dropbear-engine v{} compiled with {}",
            env!("CARGO_PKG_VERSION"),
            rustc_version_runtime::version_meta().short_version_string
        );
        log::info!("Made by tk with love at https://github.com/4tkbytes/dropbear <3");
        log::info!("======================================================================");
        #[cfg(debug_assertions)]
        {
            log::warn!(
                "⚠️ Just a heads up: this is compiled with the debug profile. Expect shit to be slow..."
            );
        }
        log::info!("dropbear-engine running...");
        let ad = app_dirs2::get_app_root(AppDataType::UserData, &config.app_info);
        if let Ok(path) = ad {
            log::info!("App data is stored at {}", path.display())
        };
        #[cfg(debug_assertions)]
        log::debug!(
            "Additional nerdy build stuff: {:?}",
            rustc_version_runtime::version_meta()
        );
        let event_loop = EventLoop::with_user_event().build()?;
        log::debug!("Created new event loop");
        let mut app = Box::new(App::new(config, future_queue));
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
/// # Parameters
/// * config - [`WindowConfiguration`]: The configuration/settings of the window.
/// * queue - [`Option<Throwable<FutureQueue>>`]: An optional value for a [`FutureQueue`]
/// * setup - [`FnOnce`]: A function that sets up all the scenes. It shouldn't be loaded
///   but instead be set as an [`Arc<Mutex<T>>>`].
macro_rules! run_app {
    ($config:expr, $queue:expr, $setup:expr) => {
        $crate::App::run($config, env!("CARGO_PKG_NAME"), $queue, $setup)
    };
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let mut window_attributes =
            Window::default_attributes().with_title(self.config.title.clone());

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

        self.state = Some(block_on(State::new(window, self.future_queue.clone())).unwrap());

        if let Some(state) = &mut self.state {
            let size = state.window.inner_size();
            state.resize(size.width, size.height);
        }

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

        state
            .egui_renderer
            .lock()
            .handle_input(&state.window, &event);

        match event {
            WindowEvent::CloseRequested => {
                log::info!("Exiting app");
                event_loop.exit();
            }
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                self.future_queue.poll();

                let frame_start = Instant::now();

                let active_handlers = self.scene_manager.get_active_input_handlers();
                self.input_manager.set_active_handlers(active_handlers);

                self.input_manager.update(&mut self.gilrs);

                let render_result =
                    state.render(&mut self.scene_manager, self.delta_time, event_loop);

                if let Err(e) = render_result {
                    log::error!("Render failed: {:?}", e);
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
                self.future_queue.cleanup();
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
                if code == KeyCode::F11
                    && key_state.is_pressed()
                    && let Some(state) = &self.state
                {
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
                self.input_manager.handle_mouse_movement(position, self.delta_position);
                self.delta_position = None;
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _device_id: DeviceId, event: DeviceEvent) {
        #[allow(clippy::single_match)]
        match event {
            DeviceEvent::MouseMotion { delta } => {
                // log::debug!("Mouse motion: {:?}", delta);
                self.delta_position = Some(delta)
            }
            _ => {}
        }
    }
}

/// The window configuration of the app/game.
///
/// This struct is primitive but has purpose in the way that it sets the initial specs of the window.
/// That's all it does. And it can also display. But that's about it.
#[derive(Debug, Clone)]
pub struct WindowConfiguration {
    pub windowed_mode: WindowedModes,
    pub title: String,
    /// This reads from a config file.
    /// This will read from a client config file under {exe}/client.props, and on game exit will save the properties to the file.
    ///
    /// As of right now, it has not been implemented yet :(
    // TODO: Implement config reading.
    // pub read_from_config: Option<String>,
    pub max_fps: u32,
    pub app_info: AppInfo,
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
