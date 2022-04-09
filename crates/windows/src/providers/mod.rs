mod thumbnail;
mod thumbnail_file;

pub use thumbnail::*;
pub use thumbnail_file::*;

use crate::registry::RegistryKey;

pub trait Provider {
    fn clsid(&self) -> windows::core::GUID;
    fn register(&self, module_path: &str) -> Vec<RegistryKey>;
    fn create_instance(
        &self,
        riid: *const windows::core::GUID,
        ppv_object: *mut *mut core::ffi::c_void,
    ) -> windows::core::Result<()>;
}
