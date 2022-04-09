use core::panic;
use std::{cell::Cell, fs, path::Path, rc::Rc};

use filament_bindings::{
    assimp::AssimpAsset,
    backend::{Backend, PixelBufferDescriptor, PixelDataFormat, PixelDataType},
    filament::{
        self, sRGBColor, Aabb, Camera, ClearOptions, Engine, Fov, IndirectLight,
        IndirectLightBuilder, LightBuilder, Projection, Renderer, Scene, SwapChain,
        SwapChainConfig, Texture, View, Viewport,
    },
    glftio::{
        AssetConfiguration, AssetLoader, MaterialProvider, ResourceConfiguration, ResourceLoader,
    },
    image::{ktx, KtxBundle},
    math::{Float3, Mat3f, Mat4f},
    utils::Entity,
};

const IDL_TEXTURE_DATA: &'static [u8] = include_bytes!("lightroom_14b_ibl.ktx");

pub struct SpaceThumbnailsRenderer {
    // need release
    engine: Engine,
    scene: Scene,
    ibl_texture: Texture,
    ibl: IndirectLight,
    swap_chain: SwapChain,
    renderer: Renderer,
    camera_entity: Entity,
    sunlight_entity: Entity,
    view: View,
    destory_asset: Option<Box<dyn FnOnce(&mut Engine, &mut Scene)>>,

    viewport: Viewport,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum RendererBackend {
    Default = 0,
    OpenGL = 1,
    Vulkan = 2,
    Metal = 3,
}

impl SpaceThumbnailsRenderer {
    pub fn new(backend: RendererBackend, width: u32, height: u32) -> Self {
        unsafe {
            let mut engine = Engine::create(match backend {
                RendererBackend::Default => Backend::DEFAULT,
                RendererBackend::OpenGL => Backend::OPENGL,
                RendererBackend::Vulkan => Backend::VULKAN,
                RendererBackend::Metal => Backend::METAL,
            })
            .unwrap();
            let mut scene = engine.create_scene().unwrap();
            let mut swap_chain = engine
                .create_headless_swap_chain(width, height, SwapChainConfig::TRANSPARENT)
                .unwrap();
            let mut renderer = engine.create_renderer().unwrap();
            let mut view = engine.create_view().unwrap();
            let mut entity_manager = engine.get_entity_manager().unwrap();
            let camera_entity = entity_manager.create();
            let mut camera = engine.create_camera(&camera_entity).unwrap();
            let ibl_texture = ktx::create_texture(
                &mut engine,
                KtxBundle::from(IDL_TEXTURE_DATA).unwrap(),
                false,
            )
            .unwrap();

            let mut ibl = IndirectLightBuilder::new()
                .unwrap()
                .reflections(&ibl_texture)
                .intensity(50000.0)
                .rotation(&Mat3f::rotation(-90.0, Float3::new(0.0, 1.0, 0.0)))
                .build(&mut engine)
                .unwrap();
            scene.set_indirect_light(&mut ibl);

            let sunlight_entity = entity_manager.create();
            LightBuilder::new(filament::LightType::SUN)
                .unwrap()
                .color(&sRGBColor(Float3::new(0.98, 0.92, 0.89)).to_linear_fast())
                .intensity(100000.0)
                .direction(&Float3::new(0.6, -1.0, -0.8).normalize())
                .cast_shadows(true)
                .sun_angular_radius(1.0)
                .sun_halo_size(2.0)
                .sun_halo_falloff(80.0)
                .build(&mut engine, &sunlight_entity)
                .unwrap();
            
            scene.add_entity(&sunlight_entity);

            view.set_camera(&mut camera);
            view.set_scene(&mut scene);
            renderer.set_clear_options(&ClearOptions {
                clear_color: [0.0, 0.0, 0.0, 0.0].into(),
                clear: true,
                discard: true,
            });

            let viewport = Viewport {
                left: 0,
                bottom: 0,
                width,
                height,
            };

            view.set_viewport(&viewport);

            // warming up
            renderer.begin_frame(&mut swap_chain);
            renderer.render(&mut view);
            renderer.end_frame();
            engine.flush_and_wait();

            Self {
                engine,
                scene,
                ibl_texture,
                ibl,
                swap_chain,
                renderer,
                camera_entity,
                sunlight_entity,
                view,
                destory_asset: None,
                viewport,
            }
        }
    }

    pub fn load_asset_from_file(&mut self, filename: impl AsRef<Path>) -> Option<&mut Self> {
        let extension = filename.as_ref().extension();

        if matches!(extension, Some(e) if e == "gltf") {
            self.load_gltf_asset(filename, false)
        } else if matches!(extension, Some(e) if e == "glb") {
            self.load_gltf_asset(filename, true)
        } else {
            let asset = AssimpAsset::from_file(&mut self.engine, filename).ok()?;
            self.load_assimp_asset(asset)
        }
    }

    pub fn load_asset_from_memory(&mut self, buffer: &[u8], filename: &str) -> Option<&mut Self> {
        let asset = AssimpAsset::from_memory(&mut self.engine, buffer, filename).ok()?;
        self.load_assimp_asset(asset)
    }

    pub fn load_assimp_asset(&mut self, mut asset: AssimpAsset) -> Option<&mut Self> {
        self.destory_opened_asset();

        unsafe {
            let aabb = asset.get_aabb();
            let transform = fit_into_unit_cube(aabb);

            let mut transform_manager = self.engine.get_transform_manager()?;
            let root_entity = asset.get_root_entity();
            let root_transform_instance = transform_manager.get_instance(root_entity)?;
            transform_manager.set_transform_float(&root_transform_instance, &transform);

            self.scene.add_entities(asset.get_renderables());

            self.scene.add_entity(root_entity);

            let mut camera = self
                .engine
                .get_camera_component(&self.camera_entity)
                .unwrap();

            camera.set_exposure_physical(16.0, 1.0 / 125.0, 100.0);

            if let Some(camera_info) = asset.get_main_camera() {
                let aspect = self.viewport.width as f64 / self.viewport.height as f64;
                if camera_info.horizontal_fov != 0.0 {
                    camera.set_projection_fov_direction(
                        camera_info.horizontal_fov,
                        aspect,
                        0.1,
                        f64::INFINITY,
                        Fov::HORIZONTAL,
                    );
                } else {
                    camera.set_projection(
                        Projection::ORTHO,
                        -camera_info.orthographic_width,
                        camera_info.orthographic_width,
                        -camera_info.orthographic_width / aspect,
                        camera_info.orthographic_width / aspect,
                        0.1,
                        100000.0,
                    );
                }
                transform_manager.set_transform_float(
                    &transform_manager.get_instance(&self.camera_entity).unwrap(),
                    &(transform
                        * Mat4f::look_at(
                            &camera_info.position,
                            &camera_info.look_at,
                            &camera_info.up,
                        )),
                )
            } else {
                setup_camera_surround_view(&mut camera, &aabb.transform(transform), &self.viewport);
            }

            self.destory_asset = Some(Box::new(move |engine, scene| {
                scene.remove_entities(asset.get_renderables());
                scene.remove_entity(asset.get_root_entity());
                asset.destory(engine)
            }));
        }

        Some(self)
    }

    pub fn load_gltf_asset(
        &mut self,
        filename: impl AsRef<Path>,
        binary: bool,
    ) -> Option<&mut Self> {
        self.destory_opened_asset();

        let filedata = fs::read(&filename).ok()?;
        let filename_str = filename.as_ref().to_str()?;

        unsafe {
            let materials = MaterialProvider::create_ubershader_loader(&mut self.engine)?;
            let mut entity_manager = self.engine.get_entity_manager()?;
            let mut transform_manager = self.engine.get_transform_manager()?;
            let mut loader = AssetLoader::create(AssetConfiguration {
                engine: &mut self.engine,
                materials,
                entities: Some(&mut entity_manager),
                default_node_name: None,
            })?;

            let mut asset = if binary {
                loader.create_asset_from_binary(&filedata)?
            } else {
                loader.create_asset_from_json(&filedata)?
            };

            ResourceLoader::create(ResourceConfiguration {
                engine: &mut self.engine,
                gltf_path: Some(filename_str.to_owned()),
                normalize_skinning_weights: true,
                recompute_bounding_boxes: false,
                ignore_bind_transform: false,
            })
            .unwrap()
            .load_resources(&mut asset);

            asset.release_source_data();

            let aabb = asset.get_bounding_box();
            let transform = fit_into_unit_cube(&aabb);
            let root_transform_instance = transform_manager.get_instance(&asset.get_root())?;

            transform_manager.set_transform_float(&root_transform_instance, &transform);

            self.scene.add_entities(asset.get_entities());

            let mut camera = self
                .engine
                .get_camera_component(&self.camera_entity)
                .unwrap();

            camera.set_exposure_physical(16.0, 1.0 / 125.0, 100.0);

            setup_camera_surround_view(&mut camera, &aabb.transform(transform), &self.viewport);

            self.destory_asset = Some(Box::new(move |_engine, scene| {
                scene.remove_entities(asset.get_entities());
                loader.destroy_asset(&asset);
                loader.destroy_materials();
                core::mem::drop(loader);
            }));
        }

        Some(self)
    }

    pub fn take_screenshot_sync(&mut self, output_memory: &mut [u8]) -> usize {
        let byte_count = self.get_screenshot_size_in_byte();

        if output_memory.len() < byte_count {
            panic!("Output memory space is not enough to take screenshot.")
        }

        unsafe {
            let ok: Rc<Cell<bool>> = Rc::new(Cell::new(false));
            let ok_inner = ok.clone();
            let pixel = PixelBufferDescriptor::from_raw_ptr_callback(
                output_memory.as_mut_ptr(),
                output_memory.len(),
                PixelDataFormat::RGBA,
                PixelDataType::UBYTE,
                move |_| ok_inner.set(true),
            );

            self.renderer.begin_frame(&mut self.swap_chain);
            self.renderer.render(&mut self.view);
            self.renderer
                .read_pixels(0, 0, self.viewport.width, self.viewport.height, pixel);
            self.renderer.end_frame();
            self.engine.flush_and_wait();

            if ok.get() == false {
                panic!("Take screenshot failed");
            }
        }

        byte_count
    }

    pub fn get_size(&self) -> (u32, u32) {
        (self.viewport.width, self.viewport.height)
    }

    pub fn get_screenshot_size_in_byte(&self) -> usize {
        (self.viewport.width * self.viewport.height * 4) as usize
    }

    pub fn destory_opened_asset(&mut self) -> &mut Self {
        let destory_asset = self.destory_asset.take();
        if let Some(destory) = destory_asset {
            destory(&mut self.engine, &mut self.scene)
        }

        self
    }
}

impl Drop for SpaceThumbnailsRenderer {
    fn drop(&mut self) {
        unsafe {
            self.destory_opened_asset();
            let mut entity_manager = self.engine.get_entity_manager().unwrap();
            self.engine.destroy_entity_components(&self.camera_entity);
            self.engine.destroy_entity_components(&self.sunlight_entity);
            entity_manager.destory(&mut self.camera_entity);
            entity_manager.destory(&mut self.sunlight_entity);
            self.engine.destroy_texture(&mut self.ibl_texture);
            self.engine.destroy_indirect_light(&mut self.ibl);
            self.engine.destroy_scene(&mut self.scene);
            self.engine.destroy_view(&mut self.view);
            self.engine.destroy_renderer(&mut self.renderer);
            self.engine.destroy_swap_chain(&mut self.swap_chain);
            Engine::destroy(&mut self.engine);
        }
    }
}

unsafe fn setup_camera_surround_view(camera: &mut Camera, aabb: &Aabb, viewport: &Viewport) {
    let aspect = viewport.width as f64 / viewport.height as f64;
    let half_extent = aabb.extent();
    camera.set_lens_projection(28.0, aspect, 0.01, f64::INFINITY);
    camera.look_at_up(
        &(aabb.center()
            + Float3::from(((half_extent[0] + half_extent[2]) / 2.0).max(half_extent[1]))
                * Float3::from([2.5, 1.7, 2.5])),
        &aabb.center(),
        &[0.0, 1.0, 0.0].into(),
    );
}

fn fit_into_unit_cube(bounds: &Aabb) -> Mat4f {
    let min = bounds.min;
    let max = bounds.max;
    let max_extent = f32::max(f32::max(max[0] - min[0], max[1] - min[1]), max[2] - min[2]);
    let scale_factor = 2.0 / max_extent;
    let center = (min + max) / 2.0;
    Mat4f::scaling(Float3::new(scale_factor, scale_factor, scale_factor))
        * Mat4f::translation(center * -1.0)
}

#[cfg(test)]
mod test {
    use std::{fs, io::Cursor, path::PathBuf, str::FromStr, time::Instant};

    use image::{ImageBuffer, ImageOutputFormat, Rgba};

    use crate::{RendererBackend, SpaceThumbnailsRenderer};

    #[test]
    fn render_test() {
        let models = fs::read_dir(
            PathBuf::from_str(env!("CARGO_MANIFEST_DIR"))
                .unwrap()
                .join("models"),
        )
        .unwrap();

        let mut renderer = SpaceThumbnailsRenderer::new(RendererBackend::Vulkan, 800, 800);

        for entry in models {
            let entry = entry.unwrap();

            if entry.file_type().unwrap().is_dir() {
                continue;
            }

            let filepath = entry.path();
            let filename = filepath.file_name().unwrap().to_str().unwrap();

            let now = Instant::now();
            renderer.load_asset_from_file(&filepath).unwrap();
            let elapsed = now.elapsed();
            println!("Load model file {}, Elapsed: {:.2?}", filename, elapsed);

            let mut screenshot_buffer = vec![0; renderer.get_screenshot_size_in_byte()];

            let now = Instant::now();
            renderer.take_screenshot_sync(screenshot_buffer.as_mut_slice());
            let elapsed = now.elapsed();
            println!("Render and take screenshot, Elapsed: {:.2?}", elapsed);

            let image = ImageBuffer::<Rgba<u8>, _>::from_raw(800, 800, screenshot_buffer).unwrap();
            let mut encoded = Cursor::new(Vec::new());
            image
                .write_to(&mut encoded, ImageOutputFormat::Png)
                .unwrap();
            test_results::save!(
                format!(
                    "{}-screenshot.png",
                    filepath
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .replace('.', "-")
                )
                .as_str(),
                encoded.get_ref().as_slice()
            )
        }
    }
}
