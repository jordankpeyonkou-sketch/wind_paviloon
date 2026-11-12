use std::{sync::Arc, time::Duration};

use bytemuck::{Pod, Zeroable};
use chrono::{Local, Timelike};
use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalSize, window::Window};

const SHADER: &str = include_str!("glass.wgsl");

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ClockUniform {
    digits: [u32; 4],
    size: [f32; 2],
    padding: [u32; 2],
}

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    configuration: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    clock_buffer: wgpu::Buffer,
    clock_bind_group: wgpu::BindGroup,
    window: Arc<Window>,
    max_surface_dimension: u32,
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> Result<Self, Box<dyn std::error::Error>> {
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(Arc::clone(&window))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or("No compatible graphics adapter was found.")?;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("GlassDesk device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                },
                None,
            )
            .await?;

        let max_surface_dimension = device.limits().max_texture_dimension_2d;
        window.set_max_inner_size(Some(PhysicalSize::new(
            max_surface_dimension,
            max_surface_dimension,
        )));
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .or_else(|| capabilities.formats.first().copied())
            .ok_or("The graphics adapter exposes no surface formats.")?;
        let alpha_mode = choose_alpha_mode(&capabilities.alpha_modes)
            .ok_or("The graphics surface exposes no supported alpha modes.")?;
        let size = clamp_surface_size(window.inner_size(), max_surface_dimension);
        if size != window.inner_size() {
            let _ = window.request_inner_size(size);
        }
        let configuration = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &configuration);

        let clock_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GlassDesk clock"),
            contents: bytemuck::bytes_of(&clock_uniform(size)),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let clock_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("GlassDesk clock layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let clock_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GlassDesk clock"),
            layout: &clock_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: clock_buffer.as_entire_binding(),
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("GlassDesk frosted clock shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("GlassDesk render layout"),
            bind_group_layouts: &[&clock_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("GlassDesk frosted clock"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Ok(Self {
            surface,
            device,
            queue,
            configuration,
            pipeline,
            clock_buffer,
            clock_bind_group,
            window,
            max_surface_dimension,
        })
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }

        let size = clamp_surface_size(size, self.max_surface_dimension);
        self.configuration.width = size.width;
        self.configuration.height = size.height;
        self.surface.configure(&self.device, &self.configuration);
        self.update_clock();
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.update_clock();
        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("GlassDesk frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("GlassDesk transparent pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.clock_bind_group, &[]);
            pass.draw(0..6, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    fn update_clock(&self) {
        self.queue.write_buffer(
            &self.clock_buffer,
            0,
            bytemuck::bytes_of(&clock_uniform(PhysicalSize::new(
                self.configuration.width,
                self.configuration.height,
            ))),
        );
    }
}

fn clamp_surface_size(size: PhysicalSize<u32>, max_dimension: u32) -> PhysicalSize<u32> {
    PhysicalSize::new(
        size.width.max(1).min(max_dimension),
        size.height.max(1).min(max_dimension),
    )
}

fn choose_alpha_mode(modes: &[wgpu::CompositeAlphaMode]) -> Option<wgpu::CompositeAlphaMode> {
    [
        wgpu::CompositeAlphaMode::PreMultiplied,
        wgpu::CompositeAlphaMode::PostMultiplied,
    ]
    .into_iter()
    .find(|mode| modes.contains(mode))
    .or_else(|| {
        modes
            .contains(&wgpu::CompositeAlphaMode::Opaque)
            .then_some(wgpu::CompositeAlphaMode::Opaque)
    })
}

fn clock_uniform(size: PhysicalSize<u32>) -> ClockUniform {
    let now = Local::now();
    let hour = now.hour();
    let minute = now.minute();
    ClockUniform {
        digits: [hour / 10, hour % 10, minute / 10, minute % 10],
        size: [size.width as f32, size.height as f32],
        padding: [0; 2],
    }
}

pub fn until_next_minute() -> Duration {
    let now = Local::now();
    Duration::from_secs(60 - u64::from(now.second()))
        - Duration::from_nanos(u64::from(now.nanosecond()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_contains_two_digit_local_time_and_surface_size() {
        let uniform = clock_uniform(PhysicalSize::new(460, 190));

        assert!(uniform.digits[0] <= 2);
        assert!(uniform.digits[1] <= 9);
        assert!(uniform.digits[2] <= 5);
        assert!(uniform.digits[3] <= 9);
        assert_eq!(uniform.size, [460.0, 190.0]);
    }

    #[test]
    fn alpha_mode_prefers_a_transparent_composition_mode() {
        assert_eq!(
            choose_alpha_mode(&[
                wgpu::CompositeAlphaMode::Opaque,
                wgpu::CompositeAlphaMode::PreMultiplied,
            ]),
            Some(wgpu::CompositeAlphaMode::PreMultiplied)
        );
    }

    #[test]
    fn alpha_mode_falls_back_to_opaque_surfaces() {
        assert_eq!(
            choose_alpha_mode(&[wgpu::CompositeAlphaMode::Opaque]),
            Some(wgpu::CompositeAlphaMode::Opaque)
        );
    }

    #[test]
    fn alpha_mode_rejects_surfaces_without_any_modes() {
        assert_eq!(choose_alpha_mode(&[]), None);
    }

    #[test]
    fn surface_size_is_bounded_by_adapter_limits() {
        assert_eq!(
            clamp_surface_size(PhysicalSize::new(2404, 1512), 2048),
            PhysicalSize::new(2048, 1512)
        );
    }

    #[test]
    fn glass_shader_parses_and_validates() {
        let module = naga::front::wgsl::parse_str(SHADER).expect("shader should parse");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("shader should validate");
    }

    #[test]
    fn next_minute_delay_is_positive_and_at_most_one_minute() {
        let delay = until_next_minute();

        assert!(!delay.is_zero());
        assert!(delay <= Duration::from_secs(60));
    }
}
