mod scene_builder;

pub use scene_builder::{build_vello_scene, set_external_font};

use rusty_mermaid_core::{Color, Scene, Theme};
use rusty_mermaid_viewport::ViewportState;

/// Convert a rusty-mermaid Scene into a vello Scene ready for GPU rendering.
pub fn render(
    scene: &Scene,
    theme: &Theme,
    viewport: &ViewportState,
) -> vello::Scene {
    build_vello_scene(scene, theme, viewport)
}

/// Render a single Scene to PNG bytes via GPU (headless).
/// Creates a fresh GPU context — use `GpuRenderer` for batch rendering.
pub fn render_to_png(
    scene: &Scene,
    theme: &Theme,
    scale: f64,
) -> Vec<u8> {
    let mut gpu = GpuRenderer::new();
    gpu.render_scene_to_png(scene, theme, scale)
}

/// Reusable GPU renderer — creates device/queue once, renders many scenes.
pub struct GpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: vello::Renderer,
}

impl GpuRenderer {
    pub fn new() -> Self {
        pollster::block_on(Self::new_async())
    }

    async fn new_async() -> Self {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .expect("no GPU adapter found");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("failed to create wgpu device");

        let renderer = vello::Renderer::new(
            &device,
            vello::RendererOptions {
                use_cpu: false,
                antialiasing_support: vello::AaSupport::all(),
                num_init_threads: None,
                pipeline_cache: None,
            },
        )
        .expect("failed to create vello renderer");

        Self { device, queue, renderer }
    }

    /// Render a Scene to PNG bytes, reusing the GPU device.
    pub fn render_scene_to_png(
        &mut self,
        scene: &Scene,
        theme: &Theme,
        scale: f64,
    ) -> Vec<u8> {
        let padding = theme.padding;
        let max_dim: u32 = 8192;

        let raw_w = ((scene.width + padding * 2.0) * scale).ceil() as u32;
        let raw_h = ((scene.height + padding * 2.0) * scale).ceil() as u32;

        let effective_scale = if raw_w > max_dim || raw_h > max_dim {
            let sx = max_dim as f64 / (scene.width + padding * 2.0);
            let sy = max_dim as f64 / (scene.height + padding * 2.0);
            sx.min(sy).min(scale)
        } else {
            scale
        };

        let width = ((scene.width + padding * 2.0) * effective_scale).ceil() as u32;
        let height = ((scene.height + padding * 2.0) * effective_scale).ceil() as u32;

        let viewport = ViewportState {
            zoom: effective_scale,
            ..Default::default()
        };
        let vello_scene = build_vello_scene(scene, theme, &viewport);
        let bg = theme.background;

        self.render_vello_to_png(&vello_scene, width, height, bg)
    }

    fn render_vello_to_png(
        &mut self,
        scene: &vello::Scene,
        width: u32,
        height: u32,
        background: Color,
    ) -> Vec<u8> {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vello_target"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bg = vello::peniko::Color::from_rgba8(background.r, background.g, background.b, background.a);

        self.renderer
            .render_to_texture(
                &self.device,
                &self.queue,
                scene,
                &view,
                &vello::RenderParams {
                    base_color: bg,
                    width,
                    height,
                    antialiasing_method: vello::AaConfig::Msaa16,
                },
            )
            .expect("vello render failed");

        // Read back pixels
        let bytes_per_row = (width * 4 + 255) & !255;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (bytes_per_row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        self.queue.submit(Some(encoder.finish()));

        let slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        rx.recv().unwrap().expect("buffer map failed");

        let data = slice.get_mapped_range();
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height {
            let start = (row * bytes_per_row) as usize;
            let end = start + (width * 4) as usize;
            pixels.extend_from_slice(&data[start..end]);
        }
        drop(data);
        buffer.unmap();

        encode_png(&pixels, width, height)
    }
}

fn encode_png(pixels: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut encoder = png::Encoder::new(&mut buf, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("PNG header write failed");
    writer.write_image_data(pixels).expect("PNG data write failed");
    writer.finish().expect("PNG finish failed");
    buf
}
