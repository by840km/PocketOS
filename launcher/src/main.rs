mod boot;
mod supervisor;
use std::io::Read;
use std::process::Stdio;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{self, Command},
};

#[derive(Clone, Copy)]
enum Accel {
    Auto,
    Tcg,
    Whpx,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Graphics {
    Auto,
    Virgl,
    Software,
}

#[derive(Clone)]
struct Options {
    root: PathBuf,
    action: String,
    memory: u32,
    accel: Accel,
    graphics: Graphics,
    cpus: u32,
}

fn parse(args: impl IntoIterator<Item = String>, default_root: PathBuf) -> Result<Options, String> {
    let mut opts = Options {
        root: default_root,
        action: "start".into(),
        memory: 4096,
        accel: Accel::Auto,
        graphics: Graphics::Auto,
        cpus: std::thread::available_parallelism()
            .map(|n| n.get().saturating_sub(2).clamp(2, 4) as u32)
            .unwrap_or(2),
    };
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "doctor" | "start" | "plan" => opts.action = arg,
            "--root" => opts.root = PathBuf::from(args.next().ok_or("Missing --root value")?),
            "--memory" => {
                opts.memory = args
                    .next()
                    .ok_or("Missing --memory value")?
                    .parse()
                    .map_err(|_| "Invalid RAM value")?;
                if !(2048..=16384).contains(&opts.memory) {
                    return Err("RAM must be 2048..16384 MiB".into());
                }
            }
            "--cpus" => {
                opts.cpus = args
                    .next()
                    .ok_or("Missing --cpus value")?
                    .parse()
                    .map_err(|_| "Invalid CPU count")?;
                if !(1..=8).contains(&opts.cpus) {
                    return Err("CPU count must be 1..8".into());
                }
            }
            "--graphics" => {
                opts.graphics = match args.next().as_deref() {
                    Some("auto") => Graphics::Auto,
                    Some("virgl") => Graphics::Virgl,
                    Some("software") => Graphics::Software,
                    _ => return Err("Use --graphics auto, virgl or software".into()),
                };
            }
            "--accel" => {
                opts.accel = match args.next().as_deref() {
                    Some("auto") => Accel::Auto,
                    Some("tcg") => Accel::Tcg,
                    Some("whpx") => Accel::Whpx,
                    _ => return Err("Use --accel auto, tcg or whpx".into()),
                }
            }
            _ => return Err(format!("Unknown argument: {arg}")),
        }
    }
    Ok(opts)
}

fn drive_value(path: &Path) -> Result<String, String> {
    let text = path.to_str().ok_or("Disk path is not valid Unicode")?;
    // QEMU parses commas in -drive separately from Windows argument quoting.
    Ok(format!(
        "file={},format=qcow2,if=virtio,readonly=on",
        text.replace(',', ",,")
    ))
}

fn qemu_args(opts: &Options) -> Result<Vec<String>, String> {
    Ok(vec![
        "-L".into(),
        "share".into(),
        "-name".into(),
        "PocketOS".into(),
        "-nodefaults".into(),
        "-no-user-config".into(),
        "-monitor".into(),
        "none".into(),
        "-parallel".into(),
        "none".into(),
        "-machine".into(),
        format!(
            "q35,accel={}",
            match opts.accel {
                Accel::Auto => "whpx:tcg",
                Accel::Tcg => "tcg",
                Accel::Whpx => "whpx",
            }
        ),
        "-m".into(),
        opts.memory.to_string(),
        "-smp".into(),
        opts.cpus.to_string(),
        "-drive".into(),
        drive_value(Path::new("../system/pocketos.qcow2"))?,
        "-vga".into(),
        "none".into(),
        "-device".into(),
        if opts.graphics == Graphics::Software {
            "virtio-vga,xres=1920,yres=1080"
        } else {
            "virtio-vga-gl,xres=1920,yres=1080"
        }
        .into(),
        "-display".into(),
        if opts.graphics == Graphics::Software {
            "sdl,gl=off"
        } else {
            "sdl,gl=on"
        }
        .into(),
        "-device".into(),
        "virtio-rng-pci".into(),
        "-device".into(),
        "qemu-xhci".into(),
        "-device".into(),
        "usb-tablet".into(),
        "-netdev".into(),
        "user,id=net0".into(),
        "-device".into(),
        "virtio-net-pci,netdev=net0".into(),
        "-kernel".into(),
        "../system/vmlinuz".into(),
        "-initrd".into(),
        "../system/initrd.img".into(),
        "-append".into(),
        if opts.graphics == Graphics::Software {
            "console=ttyS0,115200 console=tty0 rw quiet loglevel=3"
        } else {
            "console=ttyS0,115200 console=tty0 rw quiet loglevel=3 pocketos.gfx=3d"
        }
        .into(),
    ])
}

fn missing(root: &Path) -> Vec<PathBuf> {
    [
        "qemu/qemu-system-x86_64.exe",
        "qemu/share/bios-256k.bin",
        "system/pocketos.qcow2",
        "system/vmlinuz",
        "system/initrd.img",
    ]
    .iter()
    .map(|p| root.join(p))
    .filter(|p| !p.is_file() || fs::metadata(p).map(|m| m.len() == 0).unwrap_or(true))
    .collect()
}

fn run() -> Result<(), String> {
    let exe = env::current_exe().map_err(|e| e.to_string())?;
    let opts = parse(
        env::args().skip(1),
        exe.parent().ok_or("No executable directory")?.into(),
    )?;
    // Canonicalizing also makes paths independent of QEMU's working directory.
    let opts = Options {
        root: fs::canonicalize(&opts.root).map_err(|e| format!("Invalid runtime root: {e}"))?,
        ..opts
    };
    let absent = missing(&opts.root);
    println!(
        "PocketOS development core 0.4.0\nRuntime: {}",
        opts.root.display()
    );
    for p in &absent {
        println!("MISSING: {}", p.display());
    }
    if opts.action == "doctor" {
        if !absent.is_empty() {
            return Err("Runtime incomplete. See docs/BUILD.md.".into());
        }
        println!("Required files present. Hardware, DLLs and guest boot still need validation.");
        return Ok(());
    }
    let args = qemu_args(&opts)?;
    if opts.action == "plan" {
        println!(
            "Executable: {}",
            opts.root.join("qemu/qemu-system-x86_64.exe").display()
        );
        for arg in args {
            println!("ARG: {arg}");
        }
        return Ok(());
    }
    if !absent.is_empty() {
        return Err("Cannot start: required runtime files are missing".into());
    }
    if !cfg!(target_os = "windows") {
        return Err("This launcher targets Windows hosts".into());
    }
    let mut graphics = opts.graphics;
    loop {
        let active = Options {
            graphics,
            ..opts.clone()
        };
        let (outcome, errors, was_ready) = launch(&active)?;
        match outcome {
            supervisor::Outcome::MediaRemoved => {
                println!("USB removed: PocketOS closed. Volatile session discarded.");
                return Ok(());
            }
            supervisor::Outcome::Exited(false)
                if graphics == Graphics::Auto && (!was_ready || graphics_error(&errors)) =>
            {
                eprintln!("3D initialization failed. Retrying software graphics; performance will be lower.");
                println!("UI:RETRY");
                graphics = Graphics::Software;
            }
            supervisor::Outcome::StartupTimeout if graphics == Graphics::Auto => {
                println!("UI:RETRY");
                graphics = Graphics::Software;
            }
            supervisor::Outcome::StartupTimeout => {
                return Err(format!("Desktop startup timed out: {errors}"))
            }
            supervisor::Outcome::Exited(false) => return Err(format!("QEMU failed: {errors}")),
            supervisor::Outcome::Exited(true) => break,
        }
    }
    println!("QEMU exited. This does not verify cloud upload completion.");
    Ok(())
}

fn graphics_error(errors: &str) -> bool {
    let lower = errors.to_ascii_lowercase();
    ["opengl", "virgl", "egl", "glx", "sdl"]
        .iter()
        .any(|word| lower.contains(word))
}

fn launch(opts: &Options) -> Result<(supervisor::Outcome, String, bool), String> {
    #[cfg(windows)]
    let volume = supervisor::windows::Volume::capture(&opts.root)?;
    let channel = boot::BootChannel::new()?;
    let mut args = qemu_args(opts)?;
    args.extend([
        "-chardev".into(),
        format!("socket,id=boot,host=127.0.0.1,port={}", channel.port),
        "-serial".into(),
        "chardev:boot".into(),
    ]);
    println!("UI:START");
    let mut child = Command::new(opts.root.join("qemu/qemu-system-x86_64.exe"))
        .args(args)
        .current_dir(opts.root.join("qemu"))
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not start QEMU: {e}"))?;
    #[cfg(windows)]
    let _job = match supervisor::windows::Job::attach(&child) {
        Ok(job) => job,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("Cannot supervise VM: {error}"));
        }
    };
    let mut pipe = child.stderr.take().ok_or("Missing QEMU stderr")?;
    let reader = std::thread::spawn(move || {
        let mut data = Vec::new();
        let mut buffer = [0u8; 4096];
        while let Ok(count) = pipe.read(&mut buffer) {
            if count == 0 {
                break;
            }
            data.extend_from_slice(&buffer[..count]);
            if data.len() > 65536 {
                data.drain(..data.len() - 65536);
            }
        }
        String::from_utf8_lossy(&data).into_owned()
    });
    let mut last_stage = String::from("START");
    let mut ready = false;
    let mut desktop_ready = false;
    let mut timeout = false;
    let mut failed = false;
    let start = std::time::Instant::now();
    let pid = child.id();
    let outcome = supervisor::supervise(
        &mut child,
        || {
            #[cfg(windows)]
            if !volume.present() {
                return false;
            }
            while let Ok(stage) = channel.events.try_recv() {
                last_stage = stage.clone();
                match stage.as_str() {
                    "READY" => {
                        desktop_ready = true;
                    }
                    value if value.starts_with("ERROR") => {
                        failed = true;
                        println!("UI:{}", value);
                    }
                    _ => println!("UI:{}", stage),
                }
            }
            #[cfg(windows)]
            if desktop_ready && !ready && boot::window(pid, true) {
                ready = true;
                println!("UI:READY");
            }
            if !ready {
                #[cfg(windows)]
                if !desktop_ready {
                    boot::window(pid, false);
                }
                if failed || start.elapsed() > std::time::Duration::from_secs(180) {
                    timeout = true;
                    return false;
                }
            }
            #[cfg(windows)]
            {
                true
            }
            #[cfg(not(windows))]
            {
                true
            }
        },
        std::time::Duration::from_millis(250),
    )?;
    #[cfg(windows)]
    drop(_job);
    let code = child
        .try_wait()
        .ok()
        .flatten()
        .and_then(|status| status.code())
        .unwrap_or(-1);
    Ok((
        if timeout {
            supervisor::Outcome::StartupTimeout
        } else {
            outcome
        },
        reader
            .join()
            .map(|text| {
                format!(
                    "Exit code {:#010x}. Last stage: {}. {}",
                    code as u32, last_stage, text
                )
            })
            .unwrap_or_else(|_| "Could not collect QEMU diagnostics".into()),
        ready,
    ))
}

fn main() {
    if let Err(e) = run() {
        eprintln!("PocketOS: {e}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unreasonable_ram_and_unknown_accel() {
        for args in [
            vec!["--memory", "0"],
            vec!["--accel", "invalid"],
            vec!["--root"],
        ] {
            assert!(parse(args.into_iter().map(String::from), PathBuf::from(".")).is_err());
        }
    }
    #[test]
    fn runtime_follows_explicit_root_not_drive_letter() {
        let opts = parse(
            ["start", "--root", "E:/Pocket OS"].map(String::from),
            PathBuf::from("."),
        )
        .unwrap();
        let args = qemu_args(&opts).unwrap();
        assert!(args
            .iter()
            .any(|a| a == "file=../system/pocketos.qcow2,format=qcow2,if=virtio,readonly=on"));
        assert!(!args
            .iter()
            .any(|a| a.contains("PhysicalDrive") || a == "-cdrom"));
    }
    #[test]
    fn auto_acceleration_has_software_fallback() {
        let opts = parse(Vec::<String>::new(), PathBuf::from(".")).unwrap();
        assert!(qemu_args(&opts)
            .unwrap()
            .iter()
            .any(|a| a == "q35,accel=whpx:tcg"));
    }
    #[test]
    fn software_profile_is_explicit_and_does_not_request_3d() {
        let opts = parse(
            ["--graphics", "software", "--cpus", "4"].map(String::from),
            PathBuf::from("."),
        )
        .unwrap();
        let args = qemu_args(&opts).unwrap();
        assert!(args.iter().any(|a| a == "sdl,gl=off"));
        assert!(!args
            .iter()
            .any(|a| a.contains("virtio-vga-gl") || a.contains("pocketos.gfx=3d")));
        assert!(args.iter().any(|a| a.contains("readonly=on")));
        assert!(!graphics_error("disk image is missing"));
        assert!(graphics_error("OpenGL initialization failed"));
    }
    #[test]
    fn escapes_qemu_option_separator() {
        assert_eq!(
            drive_value(Path::new("D:/my,usb/disk.qcow2")).unwrap(),
            "file=D:/my,,usb/disk.qcow2,format=qcow2,if=virtio,readonly=on"
        );
    }
}
