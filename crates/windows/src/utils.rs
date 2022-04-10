use std::{
    io, mem,
    sync::{atomic::AtomicBool, Arc},
    thread,
    time::{Duration, Instant},
};

use windows::Win32::{
    Graphics::Gdi::{CreateDIBSection, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, HBITMAP, HDC},
    System::Com::{IStream, STATSTG},
};

pub fn run_timeout<T: Send + 'static>(
    func: impl FnOnce() -> T + Send + 'static,
    timeout: Duration,
) -> io::Result<T> {
    let done = Arc::new(AtomicBool::new(false));
    let done_inner = done.clone();

    let start_at = Instant::now();
    let thread_handler = thread::Builder::new().spawn(move || {
        let result = func();

        done_inner.swap(true, std::sync::atomic::Ordering::Relaxed);
        result
    })?;

    // wait for done or timeout
    loop {
        if done.load(std::sync::atomic::Ordering::Relaxed) {
            break match thread_handler.join() {
                Ok(result) => Ok(result),
                Err(_) => Err(io::Error::new(io::ErrorKind::Other, "Thread panic")),
            };
        } else if start_at.elapsed() > timeout {
            break Err(io::Error::new(io::ErrorKind::TimedOut, "Timeout"));
        } else {
            thread::sleep(Duration::from_millis(20));
            continue;
        }
    }
}

pub struct WinStream {
    stream: IStream,
}

impl WinStream {
    pub fn size(&self) -> windows::core::Result<u64> {
        unsafe {
            let mut stats = STATSTG::default();
            self.stream.Stat(&mut stats, 0)?;
            Ok(stats.cbSize)
        }
    }
}

impl From<IStream> for WinStream {
    fn from(stream: IStream) -> Self {
        Self { stream }
    }
}

impl io::Read for WinStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        let mut bytes_read = 0u32;
        unsafe {
            self.stream
                .Read(buf.as_mut_ptr() as _, buf.len() as u32, &mut bytes_read)
        }
        .map_err(|err| {
            std::io::Error::new(
                io::ErrorKind::Other,
                format!("IStream::Read failed: {}", err.code().0),
            )
        })?;
        Ok(bytes_read as usize)
    }
}

pub unsafe fn create_argb_bitmap(
    width: u32,
    height: u32,
    p_bits: &mut *mut core::ffi::c_void,
) -> HBITMAP {
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            ..Default::default()
        },
        ..Default::default()
    };
    CreateDIBSection(
        core::mem::zeroed::<HDC>(),
        &bmi,
        DIB_RGB_COLORS,
        p_bits,
        core::mem::zeroed::<windows::Win32::Foundation::HANDLE>(),
        0,
    )
}
