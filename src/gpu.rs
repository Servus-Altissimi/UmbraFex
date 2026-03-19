use std::borrow::Cow;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Uniforms { pub resolution: [f32; 2], pub time: f32, pub _pad: f32 }

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Vertex { pub pos: [f32; 2] }

pub const QUAD: &[Vertex] = &[
    Vertex { pos: [-1.0, -1.0] }, Vertex { pos: [ 1.0, -1.0] },
    Vertex { pos: [-1.0,  1.0] }, Vertex { pos: [ 1.0, -1.0] },
    Vertex { pos: [ 1.0,  1.0] }, Vertex { pos: [-1.0,  1.0] },
];

// Frame performance stats are sent from the render loop to the UI via PERF_TX.
#[derive(Clone, Default, PartialEq)]
pub struct PerfStats {
    pub fps:      f32,
    pub frame_ms: f32,
    pub w:        u32,
    pub h:        u32,
    pub gpu_name: String,
    pub backend:  String,
}

pub struct Gpu {
    pub surface:    wgpu::Surface<'static>,
    pub device:     wgpu::Device,
    pub queue:      wgpu::Queue,
    pub pipeline:   wgpu::RenderPipeline,
    pub vtx_buf:    wgpu::Buffer,
    pub uni_buf:    wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bgl:        wgpu::BindGroupLayout,
    pub config:     wgpu::SurfaceConfiguration,
    pub format:     wgpu::TextureFormat,
    pub start:      f64,
    pub canvas:     web_sys::HtmlCanvasElement, 
    pub gpu_name:   String,
    pub backend:    String,
}

impl Gpu {
    pub async fn new(canvas: web_sys::HtmlCanvasElement, shader_src: &str) -> Result<Self, String> {
        let w = canvas.client_width()  as u32;
        let h = canvas.client_height() as u32;
        canvas.set_width(w.max(1));
        canvas.set_height(h.max(1));

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|e| format!("Surface: {e}"))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface:     Some(&surface),
                power_preference:       wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| format!("Adapter: {e}"))?;

        let (device, queue): (wgpu::Device, wgpu::Queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label:                 None,
                required_features:     wgpu::Features::empty(),
                required_limits:       wgpu::Limits::downlevel_webgl2_defaults(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints:          Default::default(),
                trace:                 wgpu::Trace::Off,
            })
            .await
            .map_err(|e| format!("Device: {e}"))?;

        // Readable adapter info for the performance pane
        let info     = adapter.get_info();
        let gpu_name = info.name.clone();
        let backend  = format!("{:?}", info.backend);

        let caps   = surface.get_capabilities(&adapter);
        let format = caps.formats.first().copied().ok_or("No surface formats")?;
        let config  = wgpu::SurfaceConfiguration {
            usage:                         wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width:                         w.max(1),
            height:                        h.max(1),
            present_mode:                  wgpu::PresentMode::AutoVsync,
            alpha_mode:                    caps.alpha_modes[0],
            view_formats:                  vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let vtx_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vtx"), contents: bytemuck::cast_slice(QUAD),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let uni_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uni"), size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0, visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false, min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bg"), layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0, resource: uni_buf.as_entire_binding(),
            }],
        });
        let pipeline = build_pipeline(&device, shader_src, format, &bgl).await?;
        let start    = web_sys::window().unwrap().performance().unwrap().now();

        Ok(Gpu { surface, device, queue, pipeline, vtx_buf, uni_buf,
                  bind_group, bgl, config, format, start, canvas, gpu_name, backend })
    }

    pub fn maybe_resize(&mut self) {
        let cw = self.canvas.client_width()  as u32;
        let ch = self.canvas.client_height() as u32;
        if cw < 1 || ch < 1 { return; }
        if cw == self.config.width && ch == self.config.height { return; }
        self.canvas.set_width(cw);
        self.canvas.set_height(ch);
        self.config.width  = cw;
        self.config.height = ch;
        self.surface.configure(&self.device, &self.config);
    }

    pub async fn rebuild(&mut self, src: &str) -> Result<(), String> {
        self.pipeline = build_pipeline(&self.device, src, self.format, &self.bgl).await?;
        Ok(())
    }

    pub fn render(&mut self) {
        self.maybe_resize();

        let t = ((web_sys::window().unwrap().performance().unwrap().now() - self.start) / 1000.0) as f32;
        let u = Uniforms {
            resolution: [self.config.width as f32, self.config.height as f32],
            time: t, _pad: 0.0,
        };
        self.queue.write_buffer(&self.uni_buf, 0, bytemuck::cast_slice(&[u]));

        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            Err(_) => return,
        };
        let view = frame.texture.create_view(&Default::default());
        let mut enc = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view, resolve_target: None,
                    ops: wgpu::Operations {
                        load:  wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                multiview_mask: None,
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.vtx_buf.slice(..));
            pass.draw(0..6, 0..1);
        }
        self.queue.submit(Some(enc.finish()));
        frame.present();
    }
}

// Build pipeline and capture shader errors
pub async fn build_pipeline(
    device: &wgpu::Device,
    src:    &str,
    format: wgpu::TextureFormat,
    bgl:    &wgpu::BindGroupLayout,
) -> Result<wgpu::RenderPipeline, String> {
    // Push a validation error scope so shader errors are captured
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("s"),
        source: wgpu::ShaderSource::Wgsl(Cow::Owned(src.to_string())),
    });

    // Check compilation info for errors with line numbers
    let info = shader.get_compilation_info().await;
    let errors: Vec<String> = info.messages.iter()
        .filter(|m| m.message_type == wgpu::CompilationMessageType::Error)
        .map(|m| format!("line {}: {}", m.location.map_or(0, |l| l.line_number), m.message))
        .collect();

    // Pop error scope via the handle returned by push_error_scope
    let _ = scope.pop().await;
    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("pl"), bind_group_layouts: &[bgl], immediate_size: 0,
    });
    Ok(device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("rp"), layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader, entry_point: Some("vs_main"),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x2,
                }],
            }],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader, entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None, ..Default::default()
        },
        depth_stencil: None, multisample: Default::default(),
        multiview_mask: None, cache: None,
    }))
}
