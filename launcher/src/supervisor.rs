use std::{process::Child, thread, time::Duration};

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Exited(bool),
    MediaRemoved,
    StartupTimeout,
}

pub fn supervise(
    child: &mut Child,
    mut present: impl FnMut() -> bool,
    interval: Duration,
) -> Result<Outcome, String> {
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return Ok(Outcome::Exited(status.success()));
        }
        if !present() {
            // Kill only the child owned by this launcher; never search processes by name.
            if let Err(error) = child.kill() {
                if child.try_wait().map_err(|e| e.to_string())?.is_none() {
                    return Err(error.to_string());
                }
            }
            child.wait().map_err(|e| e.to_string())?;
            return Ok(Outcome::MediaRemoved);
        }
        thread::sleep(interval);
    }
}

#[cfg(windows)]
pub mod windows {
    use std::{
        ffi::c_void,
        mem,
        os::windows::{ffi::OsStrExt, io::AsRawHandle},
        path::Path,
        process::Child,
        ptr,
    };
    type Handle = *mut c_void;
    #[repr(C)]
    #[derive(Default)]
    struct BasicLimits {
        process_time: i64,
        job_time: i64,
        flags: u32,
        min_ws: usize,
        max_ws: usize,
        active: u32,
        affinity: usize,
        priority: u32,
        scheduling: u32,
    }
    #[repr(C)]
    #[derive(Default)]
    struct IoCounters {
        read_ops: u64,
        write_ops: u64,
        other_ops: u64,
        read_bytes: u64,
        write_bytes: u64,
        other_bytes: u64,
    }
    #[repr(C)]
    #[derive(Default)]
    struct ExtendedLimits {
        basic: BasicLimits,
        io: IoCounters,
        process_memory: usize,
        job_memory: usize,
        peak_process: usize,
        peak_job: usize,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateJobObjectW(attributes: *mut c_void, name: *const u16) -> Handle;
        fn SetInformationJobObject(job: Handle, class: i32, data: *const c_void, size: u32) -> i32;
        fn AssignProcessToJobObject(job: Handle, process: Handle) -> i32;
        fn CloseHandle(handle: Handle) -> i32;
        fn GetVolumePathNameW(path: *const u16, volume: *mut u16, size: u32) -> i32;
        fn GetVolumeInformationW(
            root: *const u16,
            name: *mut u16,
            name_size: u32,
            serial: *mut u32,
            max_component: *mut u32,
            flags: *mut u32,
            fs: *mut u16,
            fs_size: u32,
        ) -> i32;
        fn GetVolumeNameForVolumeMountPointW(root: *const u16, name: *mut u16, size: u32) -> i32;
        fn GetLogicalDrives() -> u32;
    }
    fn error() -> String {
        std::io::Error::last_os_error().to_string()
    }
    pub struct Job(Handle);
    impl Job {
        pub fn attach(child: &Child) -> Result<Self, String> {
            let handle = unsafe { CreateJobObjectW(ptr::null_mut(), ptr::null()) };
            if handle.is_null() {
                return Err(error());
            }
            let job = Self(handle);
            let mut limits = ExtendedLimits::default();
            limits.basic.flags = 0x2000; // JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            if unsafe {
                SetInformationJobObject(
                    handle,
                    9,
                    &limits as *const _ as *const c_void,
                    mem::size_of::<ExtendedLimits>() as u32,
                )
            } == 0
            {
                return Err(error());
            }
            if unsafe { AssignProcessToJobObject(handle, child.as_raw_handle()) } == 0 {
                return Err(error());
            }
            Ok(job)
        }
    }
    impl Drop for Job {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    pub struct Volume {
        root: Vec<u16>,
        serial: u32,
        guid: Vec<u16>,
        drive_bit: u32,
    }
    impl Volume {
        fn identity(root: &[u16]) -> Option<(u32, Vec<u16>)> {
            let mut serial = 0;
            let mut guid = vec![0; 64];
            if unsafe {
                GetVolumeInformationW(
                    root.as_ptr(),
                    ptr::null_mut(),
                    0,
                    &mut serial,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    0,
                )
            } == 0
            {
                return None;
            }
            if unsafe {
                GetVolumeNameForVolumeMountPointW(
                    root.as_ptr(),
                    guid.as_mut_ptr(),
                    guid.len() as u32,
                )
            } == 0
            {
                return None;
            }
            Some((serial, guid))
        }
        pub fn capture(path: &Path) -> Result<Self, String> {
            let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            let mut root = vec![0; 32768];
            if unsafe { GetVolumePathNameW(wide.as_ptr(), root.as_mut_ptr(), root.len() as u32) }
                == 0
            {
                return Err(error());
            }
            root.truncate(
                root.iter()
                    .position(|x| *x == 0)
                    .ok_or("Invalid volume path")?
                    + 1,
            );
            let text = String::from_utf16_lossy(&root);
            let bytes = text.trim_start_matches("\\\\?\\").as_bytes();
            let drive_bit = if bytes.len() > 1 && bytes[1] == b':' {
                1u32 << (bytes[0].to_ascii_uppercase() - b'A')
            } else {
                0
            };
            let (serial, guid) = Self::identity(&root).ok_or_else(error)?;
            Ok(Self {
                root,
                serial,
                guid,
                drive_bit,
            })
        }
        pub fn present(&self) -> bool {
            // The drive mask is queried first so an unplug does not wait on cached file IO.
            if self.drive_bit != 0 && unsafe { GetLogicalDrives() } & self.drive_bit == 0 {
                return false;
            }
            matches!(Self::identity(&self.root),Some((serial,guid)) if serial==self.serial && guid==self.guid)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, process::Command, time::Instant};
    fn helper() -> Child {
        Command::new(env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "supervisor::tests::helper_child",
                "--nocapture",
            ])
            .spawn()
            .unwrap()
    }
    #[test]
    #[ignore = "subprocess helper, run only by supervision tests"]
    fn helper_child() {
        thread::sleep(Duration::from_secs(60));
    }
    #[test]
    fn absent_media_reaps_owned_child() {
        let mut child = helper();
        let start = Instant::now();
        let mut polls = 0;
        let result = supervise(
            &mut child,
            || {
                polls += 1;
                polls < 3
            },
            Duration::from_millis(10),
        )
        .unwrap();
        assert_eq!(result, Outcome::MediaRemoved);
        assert!(child.try_wait().unwrap().is_some());
        assert!(start.elapsed() < Duration::from_secs(3));
    }
    #[cfg(windows)]
    #[test]
    fn job_closure_kills_child_when_launcher_disappears() {
        let mut child = helper();
        let job = windows::Job::attach(&child).unwrap();
        drop(job);
        let start = Instant::now();
        while child.try_wait().unwrap().is_none() {
            assert!(start.elapsed() < Duration::from_secs(3));
            thread::sleep(Duration::from_millis(10));
        }
    }
    #[cfg(windows)]
    #[test]
    fn current_volume_identity_can_be_monitored() {
        let volume = windows::Volume::capture(&env::current_exe().unwrap()).unwrap();
        assert!(volume.present());
    }
}
