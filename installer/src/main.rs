#![windows_subsystem = "windows"]
mod ui;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
    process::Command,
    sync::mpsc::Sender,
};
#[derive(Deserialize, Clone)]
struct Drive {
    letter: String,
    serial: String,
    free: u64,
    label: String,
    #[serde(skip)]
    guid: String,
}
#[derive(Deserialize)]
struct Part {
    name: String,
    url: String,
    size: u64,
    sha256: String,
}
#[derive(Deserialize)]
struct Manifest {
    version: String,
    required_free: u64,
    parts: Vec<Part>,
}
fn drives() -> Result<Vec<Drive>, String> {
    let script = r#"$ErrorActionPreference='Stop'; $items=@(Get-Partition | Where-Object DriveLetter | ForEach-Object { $p=$_; $d=$p|Get-Disk; if($d.BusType -eq 'USB' -and -not $d.IsBoot -and -not $d.IsSystem){$v=Get-Volume -DriveLetter $p.DriveLetter; if($v.FileSystem -in @('NTFS','exFAT')){[pscustomobject]@{letter=[string]$p.DriveLetter;serial=[string]$d.UniqueId;free=[uint64]$v.SizeRemaining;label=[string]$v.FileSystemLabel}}}}); ConvertTo-Json -Compress -InputObject $items"#;
    let out = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags_hidden()
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err("Не удалось прочитать список USB-накопителей".into());
    }
    let mut ds: Vec<Drive> = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
    for d in &mut ds {
        d.guid = volume_guid(&d.letter)?;
    }
    Ok(ds)
}
trait Hidden {
    fn creation_flags_hidden(&mut self) -> &mut Self;
}
impl Hidden for Command {
    fn creation_flags_hidden(&mut self) -> &mut Self {
        use std::os::windows::process::CommandExt;
        self.creation_flags(0x08000000)
    }
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetVolumeNameForVolumeMountPointW(path: *const u16, name: *mut u16, count: u32) -> i32;
}
fn volume_guid(letter: &str) -> Result<String, String> {
    let path: Vec<u16> = format!("{}:\\", letter)
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut name = [0u16; 128];
    if unsafe { GetVolumeNameForVolumeMountPointW(path.as_ptr(), name.as_mut_ptr(), 128) } == 0 {
        return Err("USB-накопитель недоступен".into());
    }
    Ok(String::from_utf16_lossy(
        &name[..name.iter().position(|x| *x == 0).unwrap_or(128)],
    ))
}
fn verify_drive(d: &Drive) -> Result<(), String> {
    if volume_guid(&d.letter)? == d.guid {
        Ok(())
    } else {
        Err("Флешка отключена или заменена. Установка остановлена.".into())
    }
}
fn github_url(url: &str) -> bool {
    url.starts_with("https://raw.githubusercontent.com/")
        || url.starts_with("https://github.com/")
        || url.starts_with("https://objects.githubusercontent.com/")
}
fn install(d: Drive, url: String, tx: &Sender<String>) -> Result<(), String> {
    verify_drive(&d)?;
    if !github_url(&url) {
        return Err("Манифест должен размещаться на GitHub по HTTPS".into());
    }
    let manifest: Manifest = ureq::get(&url)
        .call()
        .map_err(|e| e.to_string())?
        .into_json()
        .map_err(|e| e.to_string())?;
    if manifest.parts.is_empty() || manifest.parts.len() > 128 {
        return Err("Некорректный список файлов".into());
    }
    let target = PathBuf::from(format!("{}:\\", d.letter));
    if target.join("PocketOS").exists() {
        return Err(
            "На флешке уже есть PocketOS. Сначала сохраните или переименуйте эту папку.".into(),
        );
    }
    if drives()?
        .iter()
        .find(|x| x.serial == d.serial && x.letter == d.letter)
        .ok_or("Флешка отключена")?
        .free
        < manifest.required_free
    {
        return Err("Недостаточно свободного места".into());
    }
    let stage = target.join(".PocketOS-installing");
    if stage.exists() {
        return Err("Осталась незавершённая установка: удалите папку .PocketOS-installing на этой флешке и повторите.".into());
    }
    fs::create_dir(&stage).map_err(|e| e.to_string())?;
    let archive = stage.join("runtime.zip");
    let mut output = fs::File::create(&archive).map_err(|e| e.to_string())?;
    for (i, p) in manifest.parts.iter().enumerate() {
        verify_drive(&d)?;
        if !github_url(&p.url) || p.size > 100_000_000 || p.sha256.len() != 64 {
            return Err("Некорректная часть архива".into());
        }
        tx.send(format!(
            "Загрузка {}: часть {} из {} — {}",
            manifest.version,
            i + 1,
            manifest.parts.len(),
            p.name
        ))
        .ok();
        let mut input = ureq::get(&p.url)
            .call()
            .map_err(|e| e.to_string())?
            .into_reader();
        let mut hash = Sha256::new();
        let mut count = 0u64;
        let mut buf = [0u8; 262144];
        loop {
            let n = input.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            count += n as u64;
            if count > p.size {
                return Err("Превышен размер загрузки".into());
            }
            output.write_all(&buf[..n]).map_err(|e| e.to_string())?;
            hash.update(&buf[..n]);
        }
        if count != p.size || format!("{:x}", hash.finalize()) != p.sha256.to_lowercase() {
            return Err("Контрольная сумма не совпала. Файлы не установлены.".into());
        }
    }
    output.sync_all().map_err(|e| e.to_string())?;
    drop(output);
    tx.send("Распаковка системы на флешку…".into()).ok();
    let mut zip = zip::ZipArchive::new(fs::File::open(&archive).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let mut expanded = 0u64;
    for i in 0..zip.len() {
        verify_drive(&d)?;
        let mut file = zip.by_index(i).map_err(|e| e.to_string())?;
        let relative = file.enclosed_name().ok_or("Небезопасный путь в архиве")?;
        if relative.components().next().map(|x| x.as_os_str())
            != Some(std::ffi::OsStr::new("PocketOS"))
        {
            return Err("Архив содержит посторонние пути".into());
        }
        if file
            .unix_mode()
            .map(|m| m & 0o170000 == 0o120000)
            .unwrap_or(false)
        {
            return Err("Ссылки в архиве запрещены".into());
        }
        expanded = expanded
            .checked_add(file.size())
            .ok_or("Слишком большой архив")?;
        if expanded > manifest.required_free {
            return Err("Превышен лимит распаковки".into());
        }
        let dest = stage.join(relative);
        if file.is_dir() {
            fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
        } else {
            fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
            let mut f = fs::File::create(dest).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut f).map_err(|e| e.to_string())?;
        }
    }
    for name in [
        "PocketOS.exe",
        "pocketos-core.exe",
        "system/pocketos.qcow2",
        "system/initrd.img",
        "system/vmlinuz",
        "qemu/qemu-system-x86_64.exe",
    ] {
        if !stage.join("PocketOS").join(name).is_file() {
            return Err("Неполный комплект системы".into());
        }
    }
    verify_drive(&d)?;
    fs::rename(stage.join("PocketOS"), target.join("PocketOS")).map_err(|e| e.to_string())?;
    drop(zip);
    fs::remove_file(archive).map_err(|e| e.to_string())?;
    fs::remove_dir(stage).map_err(|e| e.to_string())?;
    Ok(())
}
fn main() {
    ui::run()
}
