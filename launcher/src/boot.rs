use std::{
    io::Read,
    net::TcpListener,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
        Arc,
    },
    thread,
    time::Duration,
};

pub struct BootChannel {
    pub port: u16,
    pub events: Receiver<String>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl BootChannel {
    pub fn new() -> Result<Self, String> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let stop = Arc::new(AtomicBool::new(false));
        let cancel = stop.clone();
        let (tx, events) = mpsc::channel();
        let worker = thread::spawn(move || {
            while !cancel.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let _ = stream.set_nonblocking(false);
                        let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
                        let mut pending = Vec::new();
                        let mut chunk = [0u8; 2048];
                        while !cancel.load(Ordering::Relaxed) {
                            match stream.read(&mut chunk) {
                                Ok(0) => break,
                                Ok(n) => {
                                    pending.extend_from_slice(&chunk[..n]);
                                    while let Some(end) = pending.iter().position(|b| *b == b'\n') {
                                        let line = String::from_utf8_lossy(&pending[..end])
                                            .trim()
                                            .to_owned();
                                        pending.drain(..=end);
                                        if let Some(offset) = line.find("POCKETOS:") {
                                            let _ = tx.send(line[offset + 9..].to_owned());
                                        }
                                    }
                                    if pending.len() > 16384 {
                                        pending.clear();
                                    }
                                }
                                Err(e)
                                    if matches!(
                                        e.kind(),
                                        std::io::ErrorKind::WouldBlock
                                            | std::io::ErrorKind::TimedOut
                                    ) =>
                                {
                                    ()
                                }
                                Err(_) => break,
                            }
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(25))
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            port,
            events,
            stop,
            worker: Some(worker),
        })
    }
}
impl Drop for BootChannel {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(windows)]
pub fn window(pid: u32, show: bool) -> bool {
    use std::ffi::c_void;
    type Hwnd = *mut c_void;
    #[repr(C)]
    #[derive(Default)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn EnumWindows(callback: unsafe extern "system" fn(Hwnd, isize) -> i32, data: isize)
            -> i32;
        fn GetWindowThreadProcessId(window: Hwnd, pid: *mut u32) -> u32;
        fn GetWindowTextW(window: Hwnd, text: *mut u16, count: i32) -> i32;
        fn SetThreadDpiAwarenessContext(context: isize) -> isize;
        fn GetWindowRect(window: Hwnd, rect: *mut Rect) -> i32;
        fn ShowWindowAsync(window: Hwnd, mode: i32) -> i32;
        fn IsWindowVisible(window: Hwnd) -> i32;
        fn GetWindowLongPtrW(window: Hwnd, index: i32) -> isize;
        fn SetWindowLongPtrW(window: Hwnd, index: i32, value: isize) -> isize;
        fn SetWindowPos(
            window: Hwnd,
            after: Hwnd,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            flags: u32,
        ) -> i32;
        fn GetSystemMetrics(index: i32) -> i32;
        fn SetForegroundWindow(window: Hwnd) -> i32;
    }
    struct Search {
        pid: u32,
        found: Hwnd,
        consoles: u32,
    }
    unsafe extern "system" fn find(window: Hwnd, data: isize) -> i32 {
        let search = unsafe { &mut *(data as *mut Search) };
        let mut owner = 0;
        unsafe {
            GetWindowThreadProcessId(window, &mut owner);
        }
        let mut title = [0u16; 256];
        let length = unsafe { GetWindowTextW(window, title.as_mut_ptr(), 256) };
        let title = String::from_utf16_lossy(&title[..length.max(0) as usize]);
        if owner == search.pid && title.starts_with("QEMU (PocketOS") {
            search.consoles += 1;
        }
        // SDL gives each QEMU console its own top-level window. Only the GPU
        // console is the desktop; never maximize a monitor/parallel console.
        if owner == search.pid && (title.contains("PocketOS-0") || title == "QEMU (PocketOS)") {
            search.found = window;
        }
        1
    }
    let mut search = Search {
        pid,
        found: std::ptr::null_mut(),
        consoles: 0,
    };
    unsafe {
        EnumWindows(find, &mut search as *mut _ as isize);
    }
    if search.found.is_null() {
        return false;
    }
    let mut positioned = true;
    unsafe {
        if show {
            // Match SDL's per-monitor physical pixels even on scaled Windows desktops.
            let old_dpi = SetThreadDpiAwarenessContext(-4);
            // Borderless window avoids the SDL exclusive-fullscreen transition during GL startup.
            let style = GetWindowLongPtrW(search.found, -16);
            SetWindowLongPtrW(
                search.found,
                -16,
                (style & !(0x00c00000 | 0x00040000)) | 0x80000000,
            );
            ShowWindowAsync(search.found, 9);
            positioned = SetWindowPos(
                search.found,
                std::ptr::null_mut(),
                0,
                0,
                GetSystemMetrics(0),
                GetSystemMetrics(1),
                0x0040 | 0x0020 | 0x4000,
            ) != 0;
            SetForegroundWindow(search.found);
            let mut rect = Rect::default();
            positioned = positioned
                && GetWindowRect(search.found, &mut rect) != 0
                && rect.left == 0
                && rect.top == 0
                && rect.right == GetSystemMetrics(0)
                && rect.bottom == GetSystemMetrics(1);
            if positioned {
                println!(
                    "UI:WINDOW:count={},bounds={},{},{},{}",
                    search.consoles, rect.left, rect.top, rect.right, rect.bottom
                );
            }
            if old_dpi != 0 {
                SetThreadDpiAwarenessContext(old_dpi);
            }
        } else {
            // Keep the GL context alive underneath the opaque topmost loader.
            if IsWindowVisible(search.found) == 0 {
                ShowWindowAsync(search.found, 4);
            }
            SetWindowPos(
                search.found,
                1usize as Hwnd,
                0,
                0,
                0,
                0,
                0x0001 | 0x0002 | 0x0010 | 0x4000,
            );
        }
    }
    positioned
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, net::TcpStream};
    #[test]
    fn fragmented_serial_readiness_is_not_a_timer() {
        let channel = BootChannel::new().unwrap();
        let mut stream = TcpStream::connect(("127.0.0.1", channel.port)).unwrap();
        stream.write_all(b"kernel noise\nPOCKETOS:RO").unwrap();
        stream
            .write_all(b"OT\r\npocketos login: POCKETOS:READY\n")
            .unwrap();
        assert_eq!(
            channel.events.recv_timeout(Duration::from_secs(2)).unwrap(),
            "ROOT"
        );
        assert_eq!(
            channel.events.recv_timeout(Duration::from_secs(2)).unwrap(),
            "READY"
        );
    }
    #[test]
    fn channel_can_close_without_a_guest_connection() {
        let c = BootChannel::new().unwrap();
        let start = std::time::Instant::now();
        drop(c);
        assert!(start.elapsed() < Duration::from_secs(1));
    }
}
