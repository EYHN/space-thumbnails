use std::{cell::RefCell, ffi::OsString, mem::size_of, os::windows::prelude::OsStringExt};

use space_thumbnails::{RendererBackend, SpaceThumbnailsRenderer};
use windows::{
    core::{implement, IUnknown, Interface, GUID},
    Win32::{
        Foundation::{ERROR_ALREADY_INITIALIZED, E_FAIL},
        Graphics::Gdi::{
            CreateDIBSection, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, HBITMAP, HDC,
        },
        UI::Shell::{
            IThumbnailProvider_Impl, PropertiesSystem::IInitializeWithFile_Impl, WTSAT_ARGB,
            WTS_ALPHATYPE,
        },
    },
};

use crate::registry::{register_clsid, RegistryData, RegistryKey, RegistryValue};

use super::Provider;

pub struct ThumbnailFileProvider {
    pub clsid: GUID,
    pub file_extension: &'static str,
}

impl ThumbnailFileProvider {
    pub fn new(clsid: GUID, file_extension: &'static str) -> Self {
        Self {
            clsid,
            file_extension,
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
        ThumbnailFileHandler::new(riid, ppv_object)
    }
}

#[implement(
    windows::Win32::UI::Shell::IThumbnailProvider,
    windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithFile
)]
pub struct ThumbnailFileHandler {
    filepath: RefCell<Option<String>>,
}

impl ThumbnailFileHandler {
    pub fn new(
        riid: *const GUID,
        ppv_object: *mut *mut core::ffi::c_void,
    ) -> windows::core::Result<()> {
        let unknown: IUnknown = ThumbnailFileHandler {
            filepath: RefCell::new(None),
        }
        .into();
        unsafe { unknown.query(&*riid, ppv_object).ok() }
    }
}

impl IThumbnailProvider_Impl for ThumbnailFileHandler {
    fn GetThumbnail(
        &self,
        cx: u32,
        phbmp: *mut HBITMAP,
        pdwalpha: *mut WTS_ALPHATYPE,
    ) -> windows::core::Result<()> {
        let mut filepath_ref = self.filepath.borrow_mut();
        let filepath = filepath_ref
            .as_mut()
            .ok_or(windows::core::Error::from(E_FAIL))?;

        let mut renderer = SpaceThumbnailsRenderer::new(RendererBackend::Vulkan, cx, cx);
        renderer.load_asset_from_file(filepath).unwrap();
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
            if let Ok(mut handle_filepath) = self.filepath.try_borrow_mut() {
                *handle_filepath = Some(filepath.to_owned());
                Ok(())
            } else {
                Err(ERROR_ALREADY_INITIALIZED.into())
            }
        } else {
            Err(E_FAIL.into())
        }
    }
}
