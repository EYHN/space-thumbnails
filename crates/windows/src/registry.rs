use windows::core::GUID;

#[derive(Debug)]
pub enum RegistryData {
    Str(String),
    U32(u32),
}

#[derive(Debug)]
pub struct RegistryValue(pub String, pub RegistryData);

#[derive(Debug)]
pub struct RegistryKey {
    pub path: String,
    pub values: Vec<RegistryValue>,
}

pub fn register_clsid(
    clsid: &GUID,
    module_path: &str,
    disable_process_isolation: bool,
) -> Vec<RegistryKey> {
    vec![
        RegistryKey {
            path: format!("CLSID\\{{{:?}}}", clsid),
            values: vec![
                RegistryValue(
                    "".to_owned(),
                    RegistryData::Str("Model Thumbnail Handler".to_owned()),
                ),
                RegistryValue(
                    "DisableProcessIsolation".to_owned(),
                    RegistryData::U32(if disable_process_isolation { 1 } else { 0 }),
                ),
            ],
        },
        RegistryKey {
            path: format!("CLSID\\{{{:?}}}\\InProcServer32", clsid),
            values: vec![
                RegistryValue("".to_owned(), RegistryData::Str(module_path.to_owned())),
                RegistryValue(
                    "ThreadingModel".to_owned(),
                    RegistryData::Str("Both".to_owned()),
                ),
            ],
        },
    ]
}
