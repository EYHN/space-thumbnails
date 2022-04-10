use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn main() {
    println!("cargo:rerun-if-changed=assets/error256x256.png");
    png2argb(
        "assets/error256x256.png",
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("error256x256.bin"),
    );

    println!("cargo:rerun-if-changed=assets/timeout256x256.png");
    png2argb(
        "assets/timeout256x256.png",
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("timeout256x256.bin"),
    );

    println!("cargo:rerun-if-changed=assets/toolarge256x256.png");
    png2argb(
        "assets/toolarge256x256.png",
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("toolarge256x256.bin"),
    );
}

fn png2argb(source: impl AsRef<Path>, out: impl AsRef<Path>) {
    let img = image::open(source).unwrap();
    let rgba = img.to_rgba8();
    let mut argb = Vec::with_capacity(rgba.len());

    for (_, _, pixel) in rgba.enumerate_pixels() {
        argb.push(pixel.0[2]);
        argb.push(pixel.0[1]);
        argb.push(pixel.0[0]);
        argb.push(pixel.0[3]);
    }

    fs::write(out, argb).unwrap();
}
