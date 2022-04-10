use std::path::{Path, PathBuf};
use std::{fs, io};

pub fn download(path: impl AsRef<Path>, url: impl AsRef<str>) -> io::Result<PathBuf> {
    let resp = ureq::get(url.as_ref()).call();

    match resp {
        Ok(resp) => {
            println!("ok");
            let mut reader = resp.into_reader();

            let mut output_file = fs::File::create(&path)?;
            io::copy(&mut reader, &mut output_file)?;
            Ok(path.as_ref().to_owned())
        }
        Err(error) => Err(io::Error::new(io::ErrorKind::Other, error.to_string())),
    }
}
