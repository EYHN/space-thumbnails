use windows::core::GUID;

#[derive(Debug)]
pub enum RegistryData {
    Str(String),
    U32(u32),
}

#[derive(Debug)]
pub struct RegistryValue(pub String, pub RegistryData);

#[derive(Debug)]
pub enum RegistryRoot {
    ClassesRoot,
    CurrentUser,
}

#[derive(Debug)]
pub struct RegistryKey {
    pub root: RegistryRoot,
    pub path: String,
    pub values: Vec<RegistryValue>,
    pub delete_key_on_unregister: bool,
}

impl RegistryKey {
    pub fn classes_root(path: String, values: Vec<RegistryValue>) -> Self {
        Self {
            root: RegistryRoot::ClassesRoot,
            path,
            values,
            delete_key_on_unregister: true,
        }
    }

    pub fn current_user(path: String, values: Vec<RegistryValue>) -> Self {
        Self {
            root: RegistryRoot::CurrentUser,
            path,
            values,
            delete_key_on_unregister: true,
        }
    }

    pub fn current_user_values(path: String, values: Vec<RegistryValue>) -> Self {
        Self {
            root: RegistryRoot::CurrentUser,
            path,
            values,
            delete_key_on_unregister: false,
        }
    }
}

pub fn register_clsid(
    clsid: &GUID,
    module_path: &str,
    disable_process_isolation: bool,
) -> Vec<RegistryKey> {
    vec![
        RegistryKey::classes_root(
            format!("CLSID\\{{{:?}}}", clsid),
            vec![
                RegistryValue(
                    "".to_owned(),
                    RegistryData::Str("Model Thumbnail Handler".to_owned()),
                ),
                RegistryValue(
                    "DisableProcessIsolation".to_owned(),
                    RegistryData::U32(if disable_process_isolation { 1 } else { 0 }),
                ),
            ],
        ),
        RegistryKey::classes_root(
            format!("CLSID\\{{{:?}}}\\InProcServer32", clsid),
            vec![
                RegistryValue("".to_owned(), RegistryData::Str(module_path.to_owned())),
                RegistryValue(
                    "ThreadingModel".to_owned(),
                    RegistryData::Str("Both".to_owned()),
                ),
            ],
        ),
    ]
}
