#![allow(dead_code)]

use crate::loader;
use image::GenericImageView;
use miette::IntoDiagnostic;

#[allow(dead_code)]
pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: Option<wgpu::BindGroup>,
}

impl Texture {
    /// Load texture from path. The path is also used as label.
    pub async fn load_from_path(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_path: &str,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> miette::Result<Self> {
        let bytes = loader::load_bytes(texture_path).await?;
        Self::from_bytes(device, queue, &bytes, texture_path, bind_group_layout)
    }

    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> miette::Result<Self> {
        let img = image::load_from_memory(bytes).into_diagnostic()?;
        Self::from_image(device, queue, &img, label, bind_group_layout)
    }

    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: &str,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> miette::Result<Self> {
        let img_rgba = img.to_rgba8();
        let dimensions = img.dimensions();
        if dimensions.0 == 0 || dimensions.1 == 0 {
            return Err(miette::miette!(
                "Texture '{}' has zero dimension: {}x{}",
                label,
                dimensions.0,
                dimensions.1
            ));
        }

        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            // depth_or_array_layers is 1 because we're not using a texture array or 3D texture
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: texture_size,
            mip_level_count: 1, // We'll talk about this a little later
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // most images are stored using sRGB
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            // TEXTURE_BINDING allows us to use this texture in shaders
            // COPY_DST means we can copy data to this texture
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            // Tells wgpu where to copy the pixel data
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &img_rgba,
            // The layout of the texture
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            texture_size,
        );

        // We don't need to configure the texture view much, so let's
        // let wgpu define it.
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some(&format!("{label}_bind_group")),
        });

        Ok(Self {
            texture,
            view,
            sampler,
            bind_group: Some(bind_group),
        })
    }
}

/// Depth texture for use as a depth buffer in a render pass.
impl Texture {
    // We need the DEPTH_FORMAT for creating the depth stage of the render_pipeline and for creating the depth texture itself.
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn create_depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        label: &str,
    ) -> Self {
        // Our depth texture needs to be the same size as our screen if we want things to render correctly.
        // We can use our config to ensure our depth texture is the same size as our surface textures.
        let size = wgpu::Extent3d {
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            // Since we are rendering to this texture, we need to add the RENDER_ATTACHMENT flag to it.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        // We technically don't need a sampler for a depth texture, but our Texture struct requires it, and we need one if we ever want to sample it.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            // If we do decide to render our depth texture, we need to use CompareFunction::LessEqual.
            // This is due to how the sampler_comparison and textureSampleCompare() interact with the texture() function in GLSL.
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
            bind_group: None,
        }
    }
}
