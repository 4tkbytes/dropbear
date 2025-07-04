pub mod scene;
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
    state: Option<State>
}

impl App {
    pub fn new(config: WindowConfiguration
    ) -> Self {
        Self {
            state: None,
            config
        }
    }

    pub fn run(config: WindowConfiguration) -> anyhow::Result<()> {
        if cfg!(debug_assertions) {
            let package_name = env!("CARGO_PKG_NAME").replace("-", "_");
            let log_config = format!("dropbear_engine=debug,{}=debug,warn", package_name);
            unsafe { std::env::set_var("RUST_LOG", log_config) };
        }

        env_logger::init();

        let event_loop = EventLoop::with_user_event().build()?;
        let mut app = App::new(config);

        event_loop.run_app(&mut app)?;

        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes()
            .with_title(self.config.title)
            .with_inner_size(PhysicalSize::new(self.config.width, self.config.height)
        );

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
                state.render();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        ..
                    },
                ..
            } => match (code, state.is_pressed()) {
                (KeyCode::Escape, true) => event_loop.exit(),
                _ => {}
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