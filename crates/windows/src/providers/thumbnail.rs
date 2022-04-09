use std::{cell::RefCell, io::Read, mem::size_of};

use space_thumbnails::{SpaceThumbnailsRenderer, RendererBackend};
use windows::{
    core::{implement, IUnknown, Interface, GUID},
    Win32::{
        Foundation::{ERROR_ALREADY_INITIALIZED, E_FAIL},
        Graphics::Gdi::*,
        System::Com::*,
        UI::Shell::{PropertiesSystem::*, *},
    },
};

use crate::{registry::{register_clsid, RegistryKey, RegistryValue, RegistryData}, win_stream::WinStream};

use super::Provider;

pub struct ThumbnailProvider {
    pub clsid: GUID,
    pub file_extension: &'static str,
}

impl ThumbnailProvider {
    pub fn new(clsid: GUID, file_extension: &'static str) -> Self {
        Self {
            clsid,
            file_extension,
        }
    }
}

impl Provider for ThumbnailProvider {
    fn clsid(&self) -> windows::core::GUID {
        self.clsid
    }

    fn register(&self, module_path: &str) -> Vec<crate::registry::RegistryKey> {
        let mut result = register_clsid(&self.clsid(), module_path, false);
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
        ThumbnailHandler::new(self.file_extension, riid, ppv_object)
    }
}

#[implement(
    windows::Win32::UI::Shell::IThumbnailProvider,
    windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithStream
)]
pub struct ThumbnailHandler {
    filename_hint: &'static str,
    stream: RefCell<Option<WinStream>>,
}

impl ThumbnailHandler {
    pub fn new(
        filename_hint: &'static str,
        riid: *const GUID,
        ppv_object: *mut *mut core::ffi::c_void,
    ) -> windows::core::Result<()> {
        let unknown: IUnknown = ThumbnailHandler {
            filename_hint,
            stream: RefCell::new(None),
        }
        .into();
        unsafe { unknown.query(&*riid, ppv_object).ok() }
    }
}

impl IThumbnailProvider_Impl for ThumbnailHandler {
    fn GetThumbnail(
        &self,
        cx: u32,
        phbmp: *mut HBITMAP,
        pdwalpha: *mut WTS_ALPHATYPE,
    ) -> windows::core::Result<()> {
        let mut stream_ref = self.stream.borrow_mut();
        let stream = stream_ref
            .as_mut()
            .ok_or(windows::core::Error::from(E_FAIL))?;
        let mut buffer = Vec::new();
        stream
            .read_to_end(&mut buffer)
            .ok()
            .ok_or(windows::core::Error::from(E_FAIL))?;

        let mut renderer = SpaceThumbnailsRenderer::new(RendererBackend::Vulkan, cx, cx);
        renderer
            .load_asset_from_memory(buffer.as_slice(), self.filename_hint)
            .unwrap();
        let mut screenshot_buffer = vec![0; renderer.get_screenshot_size_in_byte()];
        renderer.take_screenshot_sync(screenshot_buffer.as_mut_slice());

        unsafe {
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: cx as i32,
                    biHeight: -(cx as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut p_bits: *mut core::ffi::c_void = core::ptr::null_mut();
            let hbmp = CreateDIBSection(
                core::mem::zeroed::<HDC>(),
                &bmi,
                DIB_RGB_COLORS,
                &mut p_bits,
                core::mem::zeroed::<windows::Win32::Foundation::HANDLE>(),
                0,
            );
            for x in 0..cx {
                for y in 0..cx {
                    let index = ((x * cx + y) * 4) as usize;
                    let r = screenshot_buffer[index];
                    let g = screenshot_buffer[index + 1];
                    let b = screenshot_buffer[index + 2];
                    let a = screenshot_buffer[index + 3];
                    (p_bits.add(((x * cx + y) * 4) as usize) as *mut u32)
                        .write((a as u32) << 24 | (r as u32) << 16 | (g as u32) << 8 | b as u32)
                }
            }
            phbmp.write(hbmp);
            pdwalpha.write(WTSAT_ARGB);
        }
        Ok(())
    }
}

impl IInitializeWithStream_Impl for ThumbnailHandler {
    fn Initialize(&self, pstream: &Option<IStream>, _grfmode: u32) -> windows::core::Result<()> {
        if let Some(stream) = pstream {
            if let Ok(mut handle_stream) = self.stream.try_borrow_mut() {
                *handle_stream = Some(WinStream::from(stream.to_owned()));
                Ok(())
            } else {
                Err(ERROR_ALREADY_INITIALIZED.into())
            }
        } else {
            Err(E_FAIL.into())
        }
    }
}
