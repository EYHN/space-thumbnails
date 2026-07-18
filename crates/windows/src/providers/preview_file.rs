#![allow(non_camel_case_types, non_snake_case)]

use std::{
    cell::RefCell,
    ffi::OsString,
    os::windows::prelude::OsStringExt,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use space_thumbnails::{RendererBackend, SpaceThumbnailsRenderer};
use windows::{
    core::{implement, IUnknown, Interface, GUID, HRESULT, PCWSTR},
    Win32::{
        Foundation::{BOOL, E_FAIL, E_NOTIMPL, HWND, LPARAM, RECT, S_FALSE, WPARAM},
        Graphics::Gdi::{DeleteObject, HBITMAP, HGDIOBJ},
        System::Ole::{IObjectWithSite_Impl, IOleWindow_Impl},
        UI::{
            Input::KeyboardAndMouse::{GetFocus, SetFocus},
            Shell::PropertiesSystem::IInitializeWithFile_Impl,
            WindowsAndMessaging::{
                CreateWindowExW, DestroyWindow, SendMessageTimeoutW, SetWindowPos, HMENU,
                IMAGE_BITMAP, MSG, SMTO_ABORTIFHUNG, SS_BITMAP, SS_CENTERIMAGE, STM_SETIMAGE,
                SWP_NOZORDER, WINDOW_EX_STYLE, WINDOW_STYLE, WS_CHILD, WS_VISIBLE,
            },
        },
    },
};

use crate::{
    registry::{register_clsid, RegistryData, RegistryKey, RegistryValue},
    utils::create_argb_bitmap,
};

use super::Provider;

const PREVIEW_HANDLER_IID: GUID = GUID::from_u128(0x8895b1c6_b41f_4c1c_a562_0d564250836f);
const PREVHOST_APPID: &str = "{6d2b5079-2f0b-48dd-ab7f-97cec514d30b}";

#[repr(transparent)]
pub struct IPreviewHandler(IUnknown);

unsafe impl Interface for IPreviewHandler {
    type Vtable = IPreviewHandler_Vtbl;
    const IID: GUID = PREVIEW_HANDLER_IID;
}

#[repr(C)]
pub struct IPreviewHandler_Vtbl {
    pub base: windows::core::IUnknownVtbl,
    pub SetWindow: unsafe extern "system" fn(*mut core::ffi::c_void, HWND, *const RECT) -> HRESULT,
    pub SetRect: unsafe extern "system" fn(*mut core::ffi::c_void, *const RECT) -> HRESULT,
    pub DoPreview: unsafe extern "system" fn(*mut core::ffi::c_void) -> HRESULT,
    pub Unload: unsafe extern "system" fn(*mut core::ffi::c_void) -> HRESULT,
    pub SetFocus: unsafe extern "system" fn(*mut core::ffi::c_void) -> HRESULT,
    pub QueryFocus: unsafe extern "system" fn(*mut core::ffi::c_void, *mut HWND) -> HRESULT,
    pub TranslateAccelerator:
        unsafe extern "system" fn(*mut core::ffi::c_void, *const MSG) -> HRESULT,
}

pub trait IPreviewHandler_Impl: Sized {
    fn SetWindow(&self, hwnd: HWND, prc: *const RECT) -> windows::core::Result<()>;
    fn SetRect(&self, prc: *const RECT) -> windows::core::Result<()>;
    fn DoPreview(&self) -> windows::core::Result<()>;
    fn Unload(&self) -> windows::core::Result<()>;
    fn SetFocus(&self) -> windows::core::Result<()>;
    fn QueryFocus(&self) -> windows::core::Result<HWND>;
    fn TranslateAccelerator(&self, pmsg: *const MSG) -> windows::core::Result<()>;
}

impl IPreviewHandler_Vtbl {
    pub const fn new<
        Identity: windows::core::IUnknownImpl,
        Impl: IPreviewHandler_Impl,
        const OFFSET: isize,
    >() -> IPreviewHandler_Vtbl {
        unsafe extern "system" fn SetWindow<
            Identity: windows::core::IUnknownImpl,
            Impl: IPreviewHandler_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            hwnd: HWND,
            prc: *const RECT,
        ) -> HRESULT {
            let this = (this as *mut windows::core::RawPtr).offset(OFFSET) as *mut Identity;
            let this = (*this).get_impl() as *mut Impl;
            (*this).SetWindow(hwnd, prc).into()
        }
        unsafe extern "system" fn SetRect<
            Identity: windows::core::IUnknownImpl,
            Impl: IPreviewHandler_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            prc: *const RECT,
        ) -> HRESULT {
            let this = (this as *mut windows::core::RawPtr).offset(OFFSET) as *mut Identity;
            let this = (*this).get_impl() as *mut Impl;
            (*this).SetRect(prc).into()
        }
        unsafe extern "system" fn DoPreview<
            Identity: windows::core::IUnknownImpl,
            Impl: IPreviewHandler_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
        ) -> HRESULT {
            let this = (this as *mut windows::core::RawPtr).offset(OFFSET) as *mut Identity;
            let this = (*this).get_impl() as *mut Impl;
            (*this).DoPreview().into()
        }
        unsafe extern "system" fn Unload<
            Identity: windows::core::IUnknownImpl,
            Impl: IPreviewHandler_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
        ) -> HRESULT {
            let this = (this as *mut windows::core::RawPtr).offset(OFFSET) as *mut Identity;
            let this = (*this).get_impl() as *mut Impl;
            (*this).Unload().into()
        }
        unsafe extern "system" fn SetFocus<
            Identity: windows::core::IUnknownImpl,
            Impl: IPreviewHandler_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
        ) -> HRESULT {
            let this = (this as *mut windows::core::RawPtr).offset(OFFSET) as *mut Identity;
            let this = (*this).get_impl() as *mut Impl;
            (*this).SetFocus().into()
        }
        unsafe extern "system" fn QueryFocus<
            Identity: windows::core::IUnknownImpl,
            Impl: IPreviewHandler_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            phwnd: *mut HWND,
        ) -> HRESULT {
            let this = (this as *mut windows::core::RawPtr).offset(OFFSET) as *mut Identity;
            let this = (*this).get_impl() as *mut Impl;
            match (*this).QueryFocus() {
                Ok(hwnd) => {
                    phwnd.write(hwnd);
                    HRESULT(0)
                }
                Err(err) => err.into(),
            }
        }
        unsafe extern "system" fn TranslateAccelerator<
            Identity: windows::core::IUnknownImpl,
            Impl: IPreviewHandler_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            pmsg: *const MSG,
        ) -> HRESULT {
            let this = (this as *mut windows::core::RawPtr).offset(OFFSET) as *mut Identity;
            let this = (*this).get_impl() as *mut Impl;
            (*this).TranslateAccelerator(pmsg).into()
        }

        Self {
            base: windows::core::IUnknownVtbl::new::<Identity, OFFSET>(),
            SetWindow: SetWindow::<Identity, Impl, OFFSET>,
            SetRect: SetRect::<Identity, Impl, OFFSET>,
            DoPreview: DoPreview::<Identity, Impl, OFFSET>,
            Unload: Unload::<Identity, Impl, OFFSET>,
            SetFocus: SetFocus::<Identity, Impl, OFFSET>,
            QueryFocus: QueryFocus::<Identity, Impl, OFFSET>,
            TranslateAccelerator: TranslateAccelerator::<Identity, Impl, OFFSET>,
        }
    }

    pub fn matches(iid: &GUID) -> bool {
        iid == &IPreviewHandler::IID
    }
}

pub struct PreviewFileProvider {
    pub clsid: GUID,
    pub file_extension: &'static str,
    pub backend: RendererBackend,
}

impl PreviewFileProvider {
    pub fn new(clsid: GUID, file_extension: &'static str, backend: RendererBackend) -> Self {
        Self {
            clsid,
            file_extension,
            backend,
        }
    }
}

impl Provider for PreviewFileProvider {
    fn clsid(&self) -> GUID {
        self.clsid
    }

    fn register(&self, module_path: &str) -> Vec<RegistryKey> {
        let mut result = register_clsid(&self.clsid(), module_path, true);
        let clsid = format!("{{{:?}}}", &self.clsid());
        result.push(RegistryKey::classes_root(
            format!("CLSID\\{}", clsid),
            vec![
                RegistryValue(
                    "".to_owned(),
                    RegistryData::Str("Space Thumbnails Preview Handler".to_owned()),
                ),
                RegistryValue(
                    "AppID".to_owned(),
                    RegistryData::Str(PREVHOST_APPID.to_owned()),
                ),
            ],
        ));
        result.push(RegistryKey::classes_root(
            format!(
                "{}\\ShellEx\\{{{:?}}}",
                self.file_extension,
                IPreviewHandler::IID
            ),
            vec![RegistryValue(
                "".to_owned(),
                RegistryData::Str(clsid.clone()),
            )],
        ));
        result.push(RegistryKey::current_user_values(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\PreviewHandlers".to_owned(),
            vec![RegistryValue(
                clsid,
                RegistryData::Str("Space Thumbnails Preview Handler".to_owned()),
            )],
        ));
        result
    }

    fn create_instance(
        &self,
        riid: *const GUID,
        ppv_object: *mut *mut core::ffi::c_void,
    ) -> windows::core::Result<()> {
        PreviewFileHandler::new(riid, ppv_object, self.backend)
    }
}

struct PreviewWorker {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl PreviewWorker {
    fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            thread.join().ok();
        }
    }
}

struct PreviewState {
    filepath: String,
    parent_hwnd: HWND,
    rect: RECT,
    preview_hwnd: HWND,
    worker: Option<PreviewWorker>,
}

#[implement(
    IPreviewHandler,
    windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithFile,
    windows::Win32::System::Ole::IObjectWithSite,
    windows::Win32::System::Ole::IOleWindow
)]
pub struct PreviewFileHandler {
    state: RefCell<PreviewState>,
    backend: RendererBackend,
}

impl PreviewFileHandler {
    pub fn new(
        riid: *const GUID,
        ppv_object: *mut *mut core::ffi::c_void,
        backend: RendererBackend,
    ) -> windows::core::Result<()> {
        let unknown: IUnknown = PreviewFileHandler {
            state: RefCell::new(PreviewState {
                filepath: String::new(),
                parent_hwnd: HWND(0),
                rect: RECT::default(),
                preview_hwnd: HWND(0),
                worker: None,
            }),
            backend,
        }
        .into();
        unsafe { unknown.query(&*riid, ppv_object).ok() }
    }

    fn stop_preview(&self) {
        let mut state = self.state.borrow_mut();
        if let Some(worker) = state.worker.take() {
            worker.stop();
        }
        if state.preview_hwnd.0 != 0 {
            unsafe {
                replace_preview_bitmap(state.preview_hwnd, HBITMAP(0));
                DestroyWindow(state.preview_hwnd);
            }
            state.preview_hwnd = HWND(0);
        }
    }
}

impl IPreviewHandler_Impl for PreviewFileHandler {
    fn SetWindow(&self, hwnd: HWND, prc: *const RECT) -> windows::core::Result<()> {
        let mut state = self.state.borrow_mut();
        state.parent_hwnd = hwnd;
        if !prc.is_null() {
            state.rect = unsafe { *prc };
        }
        if state.preview_hwnd.0 != 0 {
            resize_preview_window(state.preview_hwnd, &state.rect);
        }
        Ok(())
    }

    fn SetRect(&self, prc: *const RECT) -> windows::core::Result<()> {
        if prc.is_null() {
            return Err(E_FAIL.into());
        }
        let mut state = self.state.borrow_mut();
        state.rect = unsafe { *prc };
        if state.preview_hwnd.0 != 0 {
            resize_preview_window(state.preview_hwnd, &state.rect);
        }
        Ok(())
    }

    fn DoPreview(&self) -> windows::core::Result<()> {
        self.stop_preview();

        let mut state = self.state.borrow_mut();
        if state.filepath.is_empty() || state.parent_hwnd.0 == 0 {
            return Err(E_FAIL.into());
        }

        let preview_hwnd = create_preview_window(state.parent_hwnd, &state.rect);
        if preview_hwnd.0 == 0 {
            return Err(E_FAIL.into());
        }

        let filepath = state.filepath.clone();
        let rect = state.rect;
        let backend = self.backend;
        let hwnd_value = preview_hwnd.0;
        let stop = Arc::new(AtomicBool::new(false));
        let stop_inner = stop.clone();
        let thread = thread::spawn(move || {
            run_preview_loop(HWND(hwnd_value), rect, filepath, backend, stop_inner);
        });

        state.preview_hwnd = preview_hwnd;
        state.worker = Some(PreviewWorker {
            stop,
            thread: Some(thread),
        });

        Ok(())
    }

    fn Unload(&self) -> windows::core::Result<()> {
        self.stop_preview();
        self.state.borrow_mut().filepath.clear();
        Ok(())
    }

    fn SetFocus(&self) -> windows::core::Result<()> {
        let state = self.state.borrow();
        if state.preview_hwnd.0 != 0 {
            unsafe {
                SetFocus(state.preview_hwnd);
            }
        }
        Ok(())
    }

    fn QueryFocus(&self) -> windows::core::Result<HWND> {
        Ok(unsafe { GetFocus() })
    }

    fn TranslateAccelerator(&self, _pmsg: *const MSG) -> windows::core::Result<()> {
        Err(S_FALSE.into())
    }
}

impl IInitializeWithFile_Impl for PreviewFileHandler {
    fn Initialize(&self, pszfilepath: &PCWSTR, _grfmode: u32) -> windows::core::Result<()> {
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
            self.state.borrow_mut().filepath = filepath;
            Ok(())
        } else {
            Err(E_FAIL.into())
        }
    }
}

impl IObjectWithSite_Impl for PreviewFileHandler {
    fn SetSite(&self, _punksite: &Option<IUnknown>) -> windows::core::Result<()> {
        Ok(())
    }

    fn GetSite(
        &self,
        _riid: *const GUID,
        _ppvsite: *mut *mut core::ffi::c_void,
    ) -> windows::core::Result<()> {
        Err(E_FAIL.into())
    }
}

impl IOleWindow_Impl for PreviewFileHandler {
    fn GetWindow(&self) -> windows::core::Result<HWND> {
        let hwnd = self.state.borrow().preview_hwnd;
        if hwnd.0 == 0 {
            Err(E_FAIL.into())
        } else {
            Ok(hwnd)
        }
    }

    fn ContextSensitiveHelp(&self, _fenterMode: BOOL) -> windows::core::Result<()> {
        Err(E_NOTIMPL.into())
    }
}

fn create_preview_window(parent: HWND, rect: &RECT) -> HWND {
    let class_name: Vec<u16> = "STATIC\0".encode_utf16().collect();
    unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(core::ptr::null()),
            WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | SS_BITMAP as u32 | SS_CENTERIMAGE as u32),
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
            parent,
            HMENU(0),
            None,
            core::ptr::null(),
        )
    }
}

fn resize_preview_window(hwnd: HWND, rect: &RECT) {
    unsafe {
        SetWindowPos(
            hwnd,
            HWND(0),
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
            SWP_NOZORDER,
        );
    }
}

fn run_preview_loop(
    preview_hwnd: HWND,
    rect: RECT,
    filepath: String,
    backend: RendererBackend,
    stop: Arc<AtomicBool>,
) {
    let width = ((rect.right - rect.left).max(256) as u32).min(800);
    let height = ((rect.bottom - rect.top).max(256) as u32).min(800);
    let mut renderer = SpaceThumbnailsRenderer::new(backend, width, height);
    if renderer.load_asset_from_file(filepath).is_none() {
        return;
    }

    let mut frame = 0u32;
    while !stop.load(Ordering::Relaxed) {
        renderer.set_model_y_rotation(frame as f32 * 0.08);
        let mut screenshot_buffer = vec![0; renderer.get_screenshot_size_in_byte()];
        renderer.take_screenshot_sync(screenshot_buffer.as_mut_slice());
        if let Some(bitmap) = bitmap_from_rgba(width, height, &screenshot_buffer) {
            unsafe {
                replace_preview_bitmap(preview_hwnd, bitmap);
            }
        }
        frame = frame.wrapping_add(1);
        thread::sleep(Duration::from_millis(67));
    }
}

unsafe fn replace_preview_bitmap(preview_hwnd: HWND, bitmap: HBITMAP) {
    let mut result = 0usize;
    let sent = SendMessageTimeoutW(
        preview_hwnd,
        STM_SETIMAGE,
        WPARAM(IMAGE_BITMAP.0 as usize),
        LPARAM(bitmap.0),
        SMTO_ABORTIFHUNG,
        250,
        &mut result,
    );
    if sent.0 == 0 {
        if bitmap.0 != 0 {
            DeleteObject(HGDIOBJ(bitmap.0));
        }
        return;
    }
    if result != 0 {
        DeleteObject(HGDIOBJ(result as isize));
    }
}

fn bitmap_from_rgba(width: u32, height: u32, screenshot_buffer: &[u8]) -> Option<HBITMAP> {
    unsafe {
        let mut p_bits: *mut core::ffi::c_void = core::ptr::null_mut();
        let hbmp = create_argb_bitmap(width, height, &mut p_bits);
        if hbmp.0 == 0 {
            return None;
        }
        for y in 0..height {
            for x in 0..width {
                let index = ((y * width + x) * 4) as usize;
                let r = screenshot_buffer[index];
                let g = screenshot_buffer[index + 1];
                let b = screenshot_buffer[index + 2];
                let a = screenshot_buffer[index + 3];
                (p_bits.add(((y * width + x) * 4) as usize) as *mut u32)
                    .write((a as u32) << 24 | (r as u32) << 16 | (g as u32) << 8 | b as u32)
            }
        }
        Some(hbmp)
    }
}
