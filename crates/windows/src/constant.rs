use windows::core::GUID;

use crate::providers::{Provider, ThumbnailProvider, ThumbnailFileProvider};

lazy_static! {
    pub static ref PROVIDERS: Vec<Box<dyn Provider + 'static + Sync>> = vec![
        Box::new(ThumbnailProvider::new(
            GUID::from_u128(0x650a0a50_3a8c_49ca_ba26_13b31965b8ef),
            ".obj",
        )),
        Box::new(ThumbnailProvider::new(
            GUID::from_u128(0xbf2644df_ae9c_4524_8bfd_2d531b837e97),
            ".fbx",
        )),
        Box::new(ThumbnailProvider::new(
            GUID::from_u128(0xb9bcfb2d_6dc4_43a0_b161_64ca282a20ff),
            ".stl",
        )),
        Box::new(ThumbnailProvider::new(
            GUID::from_u128(0x7cacb561_20c5_4b90_bd1c_5aba58b978ca),
            ".dae",
        )),
        Box::new(ThumbnailProvider::new(
            GUID::from_u128(0xb0225f87_babe_4d50_92a9_37c3c668a3e4),
            ".ply",
        )),
        Box::new(ThumbnailProvider::new(
            GUID::from_u128(0x145e37f5_99a1_40f4_b74a_6534524f29ba),
            ".x3d",
        )),
        Box::new(ThumbnailProvider::new(
            GUID::from_u128(0x1ba6aa5e_ac9a_4d3a_bcd5_678e0669fb27),
            ".x3db",
        )),
        Box::new(ThumbnailProvider::new(
            GUID::from_u128(0x93c86d4a_6432_43e2_9082_64bdb6cbfa43),
            ".3ds",
        )),
        Box::new(ThumbnailFileProvider::new(
            GUID::from_u128(0xd13b767b_a97f_4753_a4a3_7c7c15f6b25c),
            ".gltf",
        )),
        Box::new(ThumbnailFileProvider::new(
            GUID::from_u128(0x99ff43f0_d914_4a7a_8325_a8013995c41d),
            ".glb",
        ))
    ];
}
