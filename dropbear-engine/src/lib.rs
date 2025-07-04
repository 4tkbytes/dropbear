pub mod input;
pub mod scene;

pub use winit;
pub use wgpu;
pub use log;

use std::sync::Arc;

use winit::{application::ApplicationHandler, dpi::PhysicalSize, event::{KeyEvent, WindowEvent}, event_loop::{ActiveEventLoop, EventLoop}, keyboard::{KeyCode, PhysicalKey}, window::Window};

pub struct State {
    window: Arc<Window>,
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        Ok(Self {
            window,
        })
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        
    }

    pub fn render(&mut self) {
        self.window.request_redraw();
    }
}

pub struct App {
    config: WindowConfiguration,
    state: Option<State>,
    scene_manager: scene::Manager,
    input_manager: input::Manager
}

impl App {
    pub fn new(config: WindowConfiguration
    ) -> Self {
        Self {
            state: None,
            config,
            scene_manager: scene::Manager::new(),
            input_manager: input::Manager::new(),
        }
    }

    pub fn run<F>(config: WindowConfiguration, app_name: &str, setup: F) -> anyhow::Result<()>
    where F: FnOnce(&mut scene::Manager, &mut input::Manager) {
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
            WindowEvent::Resized(size) => state.resize(size.width as f32, size.height as f32),
            WindowEvent::RedrawRequested => {
                self.scene_manager.update(0.016); // todo: get update to be calculated properly
                self.scene_manager.render();

                self.input_manager.update();

                state.render();
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
                self.input_manager.handle_key_input(code, key_state.is_pressed(), event_loop);
            },
            WindowEvent::MouseInput { button, state: button_state, .. } => {
                self.input_manager.handle_mouse_input(button, button_state.is_pressed());
            },
            WindowEvent::CursorMoved { position, .. } => {
                self.input_manager.handle_mouse_movement(position);
            },
            _ => {}
        }
    }
}

pub struct WindowConfiguration {
    pub width: f32,
    pub height: f32,
    pub title: &'static str,
}