use std::path::{Path, PathBuf};
use std::{env, fs, io};

pub fn download(name: impl AsRef<str>, url: impl AsRef<str>) -> io::Result<PathBuf> {
    let resp = ureq::get(url.as_ref()).call();
    let download_dir = Path::new(&env::var("OUT_DIR").unwrap()).join("download");
    fs::create_dir_all(&download_dir).unwrap();
    let output_path = download_dir.join(name.as_ref());

    match resp {
        Ok(resp) => {
            println!("ok");
            let mut reader = resp.into_reader();
            
            let mut output_file = fs::File::create(&output_path)?;
            io::copy(&mut reader, &mut output_file)?;
            Ok(output_path)
        }
        Err(error) => Err(io::Error::new(io::ErrorKind::Other, error.to_string())),
    }
}
