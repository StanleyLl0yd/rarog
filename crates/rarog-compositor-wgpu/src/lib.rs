use std::fmt;

use rarog_compositor::{
    CompositorBackend, DisplayListRevision, FrameId, FrameSubmission, FrameUpdateKind, SurfaceSize,
};
use rarog_paint::{DamageRegion, Framebuffer, FramebufferError};
use rarog_types::Size;

pub const STAGING_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WgpuCompositorError {
    SuspendedSurface,
    MissingRetainedFrame,
    MissingStagingTexture,
    RowPitchOverflow,
    Framebuffer(FramebufferError),
}

impl fmt::Display for WgpuCompositorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SuspendedSurface => {
                formatter.write_str("wgpu compositor cannot submit a suspended surface")
            }
            Self::MissingRetainedFrame => {
                formatter.write_str("partial compositor update has no matching retained frame")
            }
            Self::MissingStagingTexture => {
                formatter.write_str("wgpu compositor has no staging texture to present")
            }
            Self::RowPitchOverflow => formatter.write_str("wgpu texture row pitch overflow"),
            Self::Framebuffer(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for WgpuCompositorError {}

impl From<FramebufferError> for WgpuCompositorError {
    fn from(error: FramebufferError) -> Self {
        Self::Framebuffer(error)
    }
}

#[derive(Debug)]
struct CpuStage {
    size: SurfaceSize,
    framebuffer: Framebuffer,
}

struct Presenter {
    format: wgpu::TextureFormat,
    sampler: wgpu::Sampler,
    pipeline: wgpu::RenderPipeline,
}

pub struct WgpuCompositorBackend {
    device: wgpu::Device,
    queue: wgpu::Queue,
    texture: Option<wgpu::Texture>,
    texture_size: Option<SurfaceSize>,
    stage: Option<CpuStage>,
    presenter: Option<Presenter>,
    last_frame: Option<FrameId>,
    last_revision: Option<DisplayListRevision>,
}

impl WgpuCompositorBackend {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        Self {
            device,
            queue,
            texture: None,
            texture_size: None,
            stage: None,
            presenter: None,
            last_frame: None,
            last_revision: None,
        }
    }

    pub const fn texture_format(&self) -> wgpu::TextureFormat {
        STAGING_TEXTURE_FORMAT
    }

    pub fn texture(&self) -> Option<&wgpu::Texture> {
        self.texture.as_ref()
    }

    pub const fn texture_size(&self) -> Option<SurfaceSize> {
        self.texture_size
    }

    pub const fn last_frame(&self) -> Option<FrameId> {
        self.last_frame
    }

    pub const fn last_revision(&self) -> Option<DisplayListRevision> {
        self.last_revision
    }

    pub fn present_to_texture(
        &mut self,
        target: &wgpu::Texture,
        target_format: wgpu::TextureFormat,
    ) -> Result<(), WgpuCompositorError> {
        if self.texture.is_none() {
            return Err(WgpuCompositorError::MissingStagingTexture);
        }
        self.ensure_presenter(target_format);

        let source = self
            .texture
            .as_ref()
            .ok_or(WgpuCompositorError::MissingStagingTexture)?;
        let presenter = self
            .presenter
            .as_ref()
            .expect("presenter is established before presentation");
        let source_view = source.create_view(&wgpu::TextureViewDescriptor::default());
        let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group_layout = presenter.pipeline.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rarog-compositor-present-bind-group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&source_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&presenter.sampler),
                },
            ],
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("rarog-compositor-present-encoder"),
            });
        {
            let attachments = [Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })];
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("rarog-compositor-present-pass"),
                color_attachments: &attachments,
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&presenter.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
        Ok(())
    }

    fn ensure_texture(&mut self, size: SurfaceSize) {
        if self.texture_size == Some(size) && self.texture.is_some() {
            return;
        }

        self.texture = Some(self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rarog-compositor-staging"),
            size: texture_extent(size),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: STAGING_TEXTURE_FORMAT,
            usage: wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        }));
        self.texture_size = Some(size);
    }

    fn ensure_presenter(&mut self, target_format: wgpu::TextureFormat) {
        if self
            .presenter
            .as_ref()
            .is_some_and(|presenter| presenter.format == target_format)
        {
            return;
        }

        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("rarog-compositor-present-shader"),
                source: wgpu::ShaderSource::Wgsl(PRESENT_SHADER.into()),
            });
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("rarog-compositor-present-bind-group-layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
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
                });
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("rarog-compositor-present-pipeline-layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });
        let targets = [Some(wgpu::ColorTargetState {
            format: target_format,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("rarog-compositor-present-pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &targets,
                }),
                multiview: None,
                cache: None,
            });
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("rarog-compositor-present-sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        self.presenter = Some(Presenter {
            format: target_format,
            sampler,
            pipeline,
        });
    }

    fn upload_stage(&self) -> Result<(), WgpuCompositorError> {
        let stage = self
            .stage
            .as_ref()
            .ok_or(WgpuCompositorError::MissingRetainedFrame)?;
        let texture = self
            .texture
            .as_ref()
            .ok_or(WgpuCompositorError::MissingRetainedFrame)?;
        let bytes_per_row = stage
            .size
            .width
            .checked_mul(4)
            .ok_or(WgpuCompositorError::RowPitchOverflow)?;
        let bytes = stage.framebuffer.to_rgba8();

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(stage.size.height),
            },
            texture_extent(stage.size),
        );
        Ok(())
    }
}

impl CompositorBackend for WgpuCompositorBackend {
    type Error = WgpuCompositorError;

    fn submit(&mut self, frame: FrameSubmission<'_>) -> Result<(), Self::Error> {
        let size = frame.plan.size();
        if size.is_suspended() {
            return Err(WgpuCompositorError::SuspendedSurface);
        }

        update_cpu_stage(&mut self.stage, &frame)?;
        self.ensure_texture(size);
        self.upload_stage()?;
        self.last_frame = Some(frame.plan.id());
        self.last_revision = Some(frame.plan.revision());
        Ok(())
    }
}

fn update_cpu_stage(
    stage: &mut Option<CpuStage>,
    frame: &FrameSubmission<'_>,
) -> Result<(), WgpuCompositorError> {
    let size = frame.plan.size();
    match frame.plan.update_kind() {
        FrameUpdateKind::Full => {
            let mut framebuffer = Framebuffer::try_new(
                Size {
                    width: size.width as f32,
                    height: size.height as f32,
                },
                frame.clear_color,
            )?;
            framebuffer.rasterize(frame.display_list);
            *stage = Some(CpuStage { size, framebuffer });
        }
        FrameUpdateKind::Partial => {
            let retained = stage
                .as_mut()
                .filter(|retained| retained.size == size)
                .ok_or(WgpuCompositorError::MissingRetainedFrame)?;
            retained.framebuffer.rasterize_damage(
                frame.display_list,
                &DamageRegion {
                    rects: frame.plan.damage().to_vec(),
                },
                frame.clear_color,
            );
        }
    }
    Ok(())
}

const PRESENT_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0),
    );

    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    output.uv = uvs[vertex_index];
    return output;
}

@group(0) @binding(0)
var source_texture: texture_2d<f32>;

@group(0) @binding(1)
var source_sampler: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(source_texture, source_sampler, input.uv);
}
"#;

const fn texture_extent(size: SurfaceSize) -> wgpu::Extent3d {
    wgpu::Extent3d {
        width: size.width,
        height: size.height,
        depth_or_array_layers: 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rarog_compositor::{
        DisplayListRevision, FrameCause, FrameDecision, FramePlanner, SurfaceId,
    };
    use rarog_paint::DisplayList;
    use rarog_types::{Color, Rect};

    #[test]
    fn cpu_stage_applies_partial_damage_without_repainting_unchanged_pixels() {
        let surface = SurfaceId::new(11).unwrap();
        let size = SurfaceSize::new(4, 2);
        let mut planner = FramePlanner::new(surface);
        let list = DisplayList::default();
        let mut stage = None;

        let FrameDecision::Submit(initial) = planner
            .plan(
                size,
                DisplayListRevision::new(1),
                &DamageRegion::default(),
                FrameCause::SceneChange,
            )
            .unwrap()
        else {
            panic!("initial frame must submit");
        };
        update_cpu_stage(
            &mut stage,
            &FrameSubmission {
                plan: &initial,
                display_list: &list,
                clear_color: Color::BLACK,
            },
        )
        .unwrap();
        planner.complete(initial.id()).unwrap();

        let damage = DamageRegion {
            rects: vec![Rect::new(1.0, 0.0, 2.0, 1.0)],
        };
        let FrameDecision::Submit(partial) = planner
            .plan(
                size,
                DisplayListRevision::new(2),
                &damage,
                FrameCause::SceneChange,
            )
            .unwrap()
        else {
            panic!("damaged frame must submit");
        };
        update_cpu_stage(
            &mut stage,
            &FrameSubmission {
                plan: &partial,
                display_list: &list,
                clear_color: Color::WHITE,
            },
        )
        .unwrap();

        let bytes = stage.unwrap().framebuffer.to_rgba8();
        let black = [0, 0, 0, 255];
        let white = [255, 255, 255, 255];
        assert_eq!(&bytes[0..4], &black);
        assert_eq!(&bytes[4..8], &white);
        assert_eq!(&bytes[8..12], &white);
        assert_eq!(&bytes[12..16], &black);
        assert!(bytes[16..].chunks_exact(4).all(|pixel| pixel == black));
    }

    #[test]
    fn partial_update_requires_matching_retained_surface() {
        let surface = SurfaceId::new(12).unwrap();
        let size = SurfaceSize::new(4, 2);
        let mut planner = FramePlanner::new(surface);
        let list = DisplayList::default();

        let FrameDecision::Submit(initial) = planner
            .plan(
                size,
                DisplayListRevision::new(1),
                &DamageRegion::default(),
                FrameCause::SceneChange,
            )
            .unwrap()
        else {
            panic!("initial frame must submit");
        };
        planner.complete(initial.id()).unwrap();

        let damage = DamageRegion {
            rects: vec![Rect::new(0.0, 0.0, 1.0, 1.0)],
        };
        let FrameDecision::Submit(partial) = planner
            .plan(
                size,
                DisplayListRevision::new(2),
                &damage,
                FrameCause::SceneChange,
            )
            .unwrap()
        else {
            panic!("damaged frame must submit");
        };

        assert_eq!(
            update_cpu_stage(
                &mut None,
                &FrameSubmission {
                    plan: &partial,
                    display_list: &list,
                    clear_color: Color::WHITE,
                },
            ),
            Err(WgpuCompositorError::MissingRetainedFrame)
        );
    }

    #[test]
    fn missing_staging_texture_has_stable_error() {
        assert_eq!(
            WgpuCompositorError::MissingStagingTexture.to_string(),
            "wgpu compositor has no staging texture to present"
        );
    }

    #[test]
    fn staging_texture_extent_tracks_surface_size() {
        assert_eq!(
            texture_extent(SurfaceSize::new(640, 480)),
            wgpu::Extent3d {
                width: 640,
                height: 480,
                depth_or_array_layers: 1,
            }
        );
    }
}
