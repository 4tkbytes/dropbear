use wgpu::{CommandEncoder, TextureView};

use crate::State;

pub struct Graphics<'a> {
    pub state: &'a State,
    pub view: &'a TextureView,
    pub encoder: &'a mut CommandEncoder,
}

impl<'a> Graphics<'a> {
    pub fn new(state: &'a State, view: &'a TextureView, encoder: &'a mut CommandEncoder) -> Self {
        Self {
            state,
            view,
            encoder,
        }
    }

    pub fn clear_colour(&mut self) {
        let _render_pass = self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
    }
}
