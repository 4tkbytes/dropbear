use egui::{Context, FontDefinitions};
use wgpu::{CommandEncoder, Device, Queue, TextureFormat, TextureView};
use egui_wgpu_backend::{wgpu, RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use winit::event::WindowEvent;
use winit::window::Window;

pub struct EguiRenderer {
    state: Platform,
    renderer: RenderPass,
    frame_started: bool,
}

impl EguiRenderer {
    pub fn context(&mut self) -> Context {
        self.state.context()
    }

    pub fn renderer(&mut self) -> &mut RenderPass {
        &mut self.renderer
    }

    pub fn new(
        device: &Device,
        output_color_format: TextureFormat,
        msaa_samples: u32,
        window: &Window,
    ) -> EguiRenderer {
        let size = window.inner_size();

        let platform = Platform::new(PlatformDescriptor {
            physical_width: size.width as u32,
            physical_height: size.height as u32,
            scale_factor: window.scale_factor(),
            font_definitions: FontDefinitions::default(),
            style: Default::default(),
        });

        let egui_renderer = RenderPass::new(device, output_color_format, msaa_samples);

        EguiRenderer {
            state: platform,
            renderer: egui_renderer,
            frame_started: false,
        }
    }

    pub fn handle_input(&mut self, event: &WindowEvent) {
        let _ = self.state.handle_event(event);
    }

    pub fn ppp(&mut self, v: f32) {
        self.context().set_pixels_per_point(v);
    }

    pub fn begin_frame(&mut self) {
        self.state.begin_pass();
        self.frame_started = true;
    }

    pub fn end_frame_and_draw(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        window: &Window,
        window_surface_view: &TextureView,
        screen_descriptor: ScreenDescriptor,
    ) {
        if !self.frame_started {
            return;
        }

        let full_output = self.state.end_pass(Some(window));
        let paint_jobs = self.state.context().tessellate(full_output.shapes, self.state.context().pixels_per_point());
        let textures_delta: egui::TexturesDelta = full_output.textures_delta;

        self.renderer
            .add_textures(device, queue, &textures_delta)
            .expect("add texture ok");
        self.renderer
            .update_buffers(device, queue, &paint_jobs, &screen_descriptor);

        self.renderer
            .execute(
                encoder,
                window_surface_view,
                &paint_jobs,
                &screen_descriptor,
                Some(wgpu::Color::BLACK),
            )
            .expect("egui execute ok");

        // self.ppp(window.scale_factor() as f32);

        self.renderer
            .remove_textures(textures_delta)
            .expect("remove texture ok");

        self.frame_started = false;
    }
}
