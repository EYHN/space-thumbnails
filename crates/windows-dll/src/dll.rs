use windows::{
    core::{implement, IUnknown, Interface, GUID, HRESULT},
    Win32::{
        Foundation::*,
        System::{
            Com::*,
            LibraryLoader::{DisableThreadLibraryCalls, GetModuleFileNameW},
            SystemServices::DLL_PROCESS_ATTACH,
        },
    },
};
use winreg::{enums::HKEY_CLASSES_ROOT, RegKey};

use space_thumbnails_windows::{constant::PROVIDERS, providers::Provider, registry::RegistryData};

static mut DLL_INSTANCE: HINSTANCE = HINSTANCE(0);

fn get_module_path(instance: HINSTANCE) -> Result<String, HRESULT> {
    let mut path: Vec<u16> = Vec::new();
    path.resize(1024, 0);
    let path_len = unsafe { GetModuleFileNameW(instance, path.as_mut_slice()) };

    let path_len = path_len as usize;
    if path_len == 0 || path_len >= path.len() {
        return Err(E_FAIL);
    }
    path.truncate(path_len + 1);
    String::from_utf16(&path).map_err(|_| E_FAIL)
}

#[implement(windows::Win32::System::Com::IClassFactory)]
struct ClassFactory {
    provider: &'static Box<dyn Provider + Sync + 'static>,
}

impl IClassFactory_Impl for ClassFactory {
    fn CreateInstance(
        &self,
        punkouter: &Option<IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut core::ffi::c_void,
    ) -> windows::core::Result<()> {
        if punkouter.is_some() {
            return CLASS_E_NOAGGREGATION.ok();
        }
        self.provider.create_instance(riid, ppvobject)
    }

    fn LockServer(&self, _flock: BOOL) -> windows::core::Result<()> {
        E_NOTIMPL.ok()
    }
}

fn shell_change_notify() {
    use std::ptr::null_mut;
    use windows::Win32::UI::Shell::{SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST};
    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, null_mut(), null_mut()) };
}

#[no_mangle]
#[allow(non_snake_case)]
#[doc(hidden)]
pub unsafe extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    pout: *mut windows::core::RawPtr,
) -> HRESULT {
    if *riid != windows::Win32::System::Com::IClassFactory::IID {
        return E_UNEXPECTED;
    }

    for provider in PROVIDERS.iter() {
        if provider.clsid() == *rclsid {
            let factory = ClassFactory { provider };
            let unknown: IUnknown = factory.into();
            return unknown.query(&*riid, pout);
        }
    }

    CLASS_E_CLASSNOTAVAILABLE
}

#[no_mangle]
#[allow(non_snake_case)]
#[doc(hidden)]
pub extern "stdcall" fn DllMain(
    dll_instance: HINSTANCE,
    reason: u32,
    _reserved: *mut core::ffi::c_void,
) -> bool {
    if reason == DLL_PROCESS_ATTACH {
        unsafe {
            DLL_INSTANCE = dll_instance;
            DisableThreadLibraryCalls(dll_instance);
        }
    }
    true
}

#[no_mangle]
#[allow(non_snake_case)]
#[doc(hidden)]
pub unsafe extern "system" fn DllRegisterServer() -> HRESULT {
    let module_path = {
        let result = get_module_path(DLL_INSTANCE);
        if let Err(err) = result {
            return err;
        }
        result.unwrap()
    };
    if register(&module_path).is_ok() {
        shell_change_notify();
        S_OK
    } else {
        E_FAIL
    }
}

#[no_mangle]
#[allow(non_snake_case)]
#[doc(hidden)]
pub unsafe extern "system" fn DllUnregisterServer() -> HRESULT {
    if unregister().is_ok() {
        shell_change_notify();
        S_OK
    } else {
        E_FAIL
    }
}

fn register(module_path: &str) -> std::io::Result<()> {
    for provider in PROVIDERS.iter() {
        for key in provider.register(module_path) {
            let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
            let (regkey, _) = hkcr.create_subkey(key.path)?;
            for val in key.values {
                match val.1 {
                    RegistryData::Str(data) => regkey.set_value(val.0, &data)?,
                    RegistryData::U32(data) => regkey.set_value(val.0, &data)?,
                }
            }
        }
    }

    Ok(())
}

fn unregister() -> std::io::Result<()> {
    for provider in PROVIDERS.iter() {
        for key in provider.register("") {
            let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
            hkcr.delete_subkey_all(key.path).ok();
        }
    }

    Ok(())
}
