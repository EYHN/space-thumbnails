use std::{
    cell::Cell,
    ffi::OsString,
    fs, io,
    os::windows::prelude::OsStringExt,
    time::{Duration, Instant},
};

use log::info;
use space_thumbnails::{RendererBackend, SpaceThumbnailsRenderer};
use windows::{
    core::{implement, IUnknown, Interface, GUID},
    Win32::{
        Foundation::E_FAIL,
        Graphics::Gdi::HBITMAP,
        UI::Shell::{
            IThumbnailProvider_Impl, PropertiesSystem::IInitializeWithFile_Impl, WTSAT_ARGB,
            WTS_ALPHATYPE,
        },
    },
};

use crate::{
    constant::{ERROR_256X256_ARGB, TIMEOUT_256X256_ARGB, TOOLARGE_256X256_ARGB},
    registry::{register_clsid, RegistryData, RegistryKey, RegistryValue},
    utils::{create_argb_bitmap, run_timeout},
};

use super::Provider;

pub struct ThumbnailFileProvider {
    pub clsid: GUID,
    pub file_extension: &'static str,
    pub backend: RendererBackend,
}

impl ThumbnailFileProvider {
    pub fn new(clsid: GUID, file_extension: &'static str, backend: RendererBackend) -> Self {
        Self {
            clsid,
            file_extension,
            backend,
        }
    }
}

impl Provider for ThumbnailFileProvider {
    fn clsid(&self) -> windows::core::GUID {
        self.clsid
    }

    fn register(&self, module_path: &str) -> Vec<crate::registry::RegistryKey> {
        let mut result = register_clsid(&self.clsid(), module_path, true);
        result.append(&mut vec![RegistryKey {
            path: format!(
                "{}\\ShellEx\\{{{:?}}}",
                self.file_extension,
                windows::Win32::UI::Shell::IThumbnailProvider::IID
            ),
            values: vec![RegistryValue(
                "".to_owned(),
                RegistryData::Str(format!("{{{:?}}}", &self.clsid())),
            )],
        }]);
        result
    }

    fn create_instance(
        &self,
        riid: *const windows::core::GUID,
        ppv_object: *mut *mut core::ffi::c_void,
    ) -> windows::core::Result<()> {
        ThumbnailFileHandler::new(riid, ppv_object, self.backend)
    }
}

#[implement(
    windows::Win32::UI::Shell::IThumbnailProvider,
    windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithFile
)]
pub struct ThumbnailFileHandler {
    filepath: Cell<String>,
    backend: RendererBackend,
}

impl ThumbnailFileHandler {
    pub fn new(
        riid: *const GUID,
        ppv_object: *mut *mut core::ffi::c_void,
        backend: RendererBackend,
    ) -> windows::core::Result<()> {
        let unknown: IUnknown = ThumbnailFileHandler {
            filepath: Cell::new(String::new()),
            backend,
        }
        .into();
        unsafe { unknown.query(&*riid, ppv_object).ok() }
    }
}

impl IThumbnailProvider_Impl for ThumbnailFileHandler {
    fn GetThumbnail(
        &self,
        _: u32,
        phbmp: *mut HBITMAP,
        pdwalpha: *mut WTS_ALPHATYPE,
    ) -> windows::core::Result<()> {
        let filepath = self.filepath.take();
        let size = 256;

        if filepath.is_empty() {
            return Err(windows::core::Error::from(E_FAIL));
        }

        if matches!(fs::metadata(&filepath), Ok(metadata) if metadata.len() > 300 * 1024 * 1024 /* 300 MB */)
        {
            unsafe {
                let mut p_bits: *mut core::ffi::c_void = core::ptr::null_mut();
                let hbmp = create_argb_bitmap(256, 256, &mut p_bits);
                std::ptr::copy(
                    TOOLARGE_256X256_ARGB.as_ptr(),
                    p_bits as *mut _,
                    TOOLARGE_256X256_ARGB.len(),
                );
                phbmp.write(hbmp);
                pdwalpha.write(WTSAT_ARGB);
            }
            return Ok(());
        }

        let start_time = Instant::now();
        info!(target: "ThumbnailFileProvider", "Getting thumbnail from file: {}", filepath);

        let filepath_clone = filepath.clone();
        let backend = self.backend;
        let timeout_result = run_timeout(
            move || {
                let mut renderer = SpaceThumbnailsRenderer::new(backend, size, size);
                renderer.load_asset_from_file(filepath_clone)?;
                let mut screenshot_buffer = vec![0; renderer.get_screenshot_size_in_byte()];
                renderer.take_screenshot_sync(screenshot_buffer.as_mut_slice());
                Some(screenshot_buffer)
            },
            Duration::from_secs(5),
        );

        match timeout_result {
            Ok(Some(screenshot_buffer)) => {
                info!(target: "ThumbnailFileProvider", "Rendering thumbnails success file: {}, Elapsed: {:.2?}", filepath, start_time.elapsed());
                unsafe {
                    let mut p_bits: *mut core::ffi::c_void = core::ptr::null_mut();
                    let hbmp = create_argb_bitmap(size, size, &mut p_bits);
                    for x in 0..size {
                        for y in 0..size {
                            let index = ((x * size + y) * 4) as usize;
                            let r = screenshot_buffer[index];
                            let g = screenshot_buffer[index + 1];
                            let b = screenshot_buffer[index + 2];
                            let a = screenshot_buffer[index + 3];
                            (p_bits.add(((x * size + y) * 4) as usize) as *mut u32).write(
                                (a as u32) << 24 | (r as u32) << 16 | (g as u32) << 8 | b as u32,
                            )
                        }
                    }
                    phbmp.write(hbmp);
                    pdwalpha.write(WTSAT_ARGB);
                }
                Ok(())
            }
            Err(err) if err.kind() == io::ErrorKind::TimedOut => {
                info!(target: "ThumbnailFileProvider", "Rendering thumbnails timeout file: {}, Elapsed: {:.2?}", filepath, start_time.elapsed());
                unsafe {
                    let mut p_bits: *mut core::ffi::c_void = core::ptr::null_mut();
                    let hbmp = create_argb_bitmap(256, 256, &mut p_bits);
                    std::ptr::copy(
                        TIMEOUT_256X256_ARGB.as_ptr(),
                        p_bits as *mut _,
                        TIMEOUT_256X256_ARGB.len(),
                    );
                    phbmp.write(hbmp);
                    pdwalpha.write(WTSAT_ARGB);
                }
                Ok(())
            }
            Err(_) | Ok(None) => {
                info!(target: "ThumbnailFileProvider", "Rendering thumbnails error file: {}, Elapsed: {:.2?}", filepath, start_time.elapsed());
                unsafe {
                    let mut p_bits: *mut core::ffi::c_void = core::ptr::null_mut();
                    let hbmp = create_argb_bitmap(256, 256, &mut p_bits);
                    std::ptr::copy(
                        ERROR_256X256_ARGB.as_ptr(),
                        p_bits as *mut _,
                        ERROR_256X256_ARGB.len(),
                    );
                    phbmp.write(hbmp);
                    pdwalpha.write(WTSAT_ARGB);
                }
                Ok(())
            }
        }
    }
}

impl IInitializeWithFile_Impl for ThumbnailFileHandler {
    fn Initialize(
        &self,
        pszfilepath: &windows::core::PCWSTR,
        _grfmode: u32,
    ) -> windows::core::Result<()> {
        let filepath = unsafe {
            let str_p = pszfilepath.0;
            let mut str_len = 0;
            loop {
                if str_p.add(str_len).read() != 0 {
                    str_len += 1;
                    if str_len > 1024 {
                        return Err(E_FAIL.into());
                    }
                    continue;
                } else {
                    break;
                }
            }
            if str_len > 0 {
                OsString::from_wide(core::slice::from_raw_parts(str_p, str_len))
                    .to_str()
                    .map(|s| s.to_owned())
            } else {
                None
            }
        };
        if let Some(filepath) = filepath {
            self.filepath.set(filepath);
            Ok(())
        } else {
            Err(E_FAIL.into())
        }
    }
}
