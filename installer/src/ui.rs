use super::*;
use std::{ffi::c_void, ptr, sync::mpsc};
type H = *mut c_void;
#[repr(C)]
struct Class {
    style: u32,
    proc: Option<unsafe extern "system" fn(H, u32, usize, isize) -> isize>,
    extra: i32,
    window_extra: i32,
    instance: H,
    icon: H,
    cursor: H,
    background: H,
    menu: *const u16,
    name: *const u16,
}
#[repr(C)]
struct Msg {
    window: H,
    message: u32,
    w: usize,
    l: isize,
    time: u32,
    x: i32,
    y: i32,
    private: u32,
}
#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterClassW(c: *const Class) -> u16;
    fn CreateWindowExW(
        ex: u32,
        class: *const u16,
        title: *const u16,
        style: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        parent: H,
        menu: H,
        instance: H,
        param: H,
    ) -> H;
    fn DefWindowProcW(h: H, m: u32, w: usize, l: isize) -> isize;
    fn ShowWindow(h: H, n: i32) -> i32;
    fn GetMessageW(m: *mut Msg, h: H, a: u32, b: u32) -> i32;
    fn TranslateMessage(m: *const Msg) -> i32;
    fn DispatchMessageW(m: *const Msg) -> isize;
    fn PostQuitMessage(n: i32);
    fn SendMessageW(h: H, m: u32, w: usize, l: isize) -> isize;
    fn SetWindowTextW(h: H, s: *const u16) -> i32;
    fn EnableWindow(h: H, b: i32) -> i32;
    fn SetTimer(h: H, id: usize, time: u32, cb: H) -> usize;
    fn MessageBoxW(h: H, text: *const u16, title: *const u16, kind: u32) -> i32;
    fn LoadCursorW(instance: H, id: usize) -> H;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> H;
}
struct State {
    combo: H,
    status: H,
    button: H,
    drives: Vec<Drive>,
    rx: mpsc::Receiver<String>,
    tx: mpsc::Sender<String>,
    busy: bool,
}
static mut STATE: *mut State = ptr::null_mut();
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
unsafe fn text(h: H, s: &str) {
    SetWindowTextW(h, wide(s).as_ptr());
}
unsafe fn child(
    h: H,
    class: &str,
    label: &str,
    style: u32,
    x: i32,
    y: i32,
    w: i32,
    height: i32,
    id: usize,
) -> H {
    CreateWindowExW(
        0,
        wide(class).as_ptr(),
        wide(label).as_ptr(),
        0x50000000 | style,
        x,
        y,
        w,
        height,
        h,
        id as H,
        GetModuleHandleW(ptr::null()),
        ptr::null_mut(),
    )
}
unsafe fn refresh(s: &mut State) {
    SendMessageW(s.combo, 0x14b, 0, 0);
    match drives() {
        Ok(ds) => {
            s.drives = ds;
            for d in &s.drives {
                let label = format!(
                    "{}:  {} — свободно {:.1} ГБ",
                    d.letter,
                    d.label,
                    d.free as f64 / 1e9
                );
                SendMessageW(s.combo, 0x143, 0, wide(&label).as_ptr() as isize);
            }
            SendMessageW(s.combo, 0x14e, 0, 0);
            text(s.status,"Выберите USB-накопитель. Имеющиеся файлы сохранятся.\r\nДля установки нужен интернет и около 5 ГБ свободного места.");
        }
        Err(e) => text(s.status, &e),
    }
}
unsafe extern "system" fn procedure(h: H, m: u32, w: usize, l: isize) -> isize {
    if !STATE.is_null() {
        let s = &mut *STATE;
        match m {
            0x111 => {
                match w & 0xffff {
                    2 if !s.busy => refresh(s),
                    3 if !s.busy => {
                        let index = SendMessageW(s.combo, 0x147, 0, 0);
                        if index >= 0 {
                            if let Some(d) = s.drives.get(index as usize).cloned() {
                                if MessageBoxW(h,wide(&format!("Установить PocketOS на {}: ({})?\nФорматирование не выполняется. Существующая папка PocketOS не заменяется.",d.letter,d.label)).as_ptr(),wide("Установка PocketOS").as_ptr(),0x24)==6{s.busy=true;EnableWindow(s.combo,0);EnableWindow(s.button,0);let tx=s.tx.clone();std::thread::spawn(move||{let result=install(d,env!("POCKETOS_MANIFEST_URL").into(),&tx);tx.send(match result{Ok(())=>"DONE:Готово. Откройте PocketOS\\PocketOS.exe на флешке.\r\nBeta: изменения сеанса временные, облако ещё не подключено.".into(),Err(e)=>format!("DONE:Установка остановлена: {e}")}).ok();});}
                            }
                        }
                    }
                    _ => {}
                }
                return 0;
            }
            0x113 => {
                while let Ok(message) = s.rx.try_recv() {
                    if let Some(result) = message.strip_prefix("DONE:") {
                        s.busy = false;
                        EnableWindow(s.combo, 1);
                        EnableWindow(s.button, 1);
                        text(s.status, result);
                    } else {
                        text(s.status, &message);
                    }
                }
                return 0;
            }
            0x10 if s.busy => {
                MessageBoxW(
                    h,
                    wide("Идёт запись. Дождитесь завершения установки и не извлекайте флешку.")
                        .as_ptr(),
                    wide("PocketOS").as_ptr(),
                    0x40,
                );
                return 0;
            }
            2 => {
                PostQuitMessage(0);
                return 0;
            }
            _ => {}
        }
    }
    DefWindowProcW(h, m, w, l)
}
pub fn run() {
    unsafe {
        let instance = GetModuleHandleW(ptr::null());
        let name = wide("PocketOSSetup");
        let class = Class {
            style: 0,
            proc: Some(procedure),
            extra: 0,
            window_extra: 0,
            instance,
            icon: ptr::null_mut(),
            cursor: LoadCursorW(ptr::null_mut(), 32512),
            background: 6usize as H,
            menu: ptr::null(),
            name: name.as_ptr(),
        };
        RegisterClassW(&class);
        let h = CreateWindowExW(
            0,
            name.as_ptr(),
            wide("PocketOS — установка на USB · Beta").as_ptr(),
            0x00ca0000,
            200,
            180,
            690,
            390,
            ptr::null_mut(),
            ptr::null_mut(),
            instance,
            ptr::null_mut(),
        );
        child(
            h,
            "STATIC",
            "PocketOS  /  KDE Plasma",
            0,
            28,
            24,
            610,
            30,
            0,
        );
        child(
            h,
            "STATIC",
            "USB-накопитель (exFAT или NTFS)",
            0,
            28,
            72,
            610,
            24,
            0,
        );
        let combo = child(
            h,
            "COMBOBOX",
            "",
            0x00200003 | 0x00010000,
            28,
            104,
            480,
            200,
            1,
        );
        child(h, "BUTTON", "Обновить", 0x10000, 520, 103, 125, 30, 2);
        let status = child(h, "STATIC", "", 0, 28, 152, 615, 115, 0);
        let button = child(h, "BUTTON", "Установить", 0x10001, 475, 290, 170, 38, 3);
        let (tx, rx) = mpsc::channel();
        STATE = Box::into_raw(Box::new(State {
            combo,
            status,
            button,
            drives: Vec::new(),
            rx,
            tx,
            busy: false,
        }));
        refresh(&mut *STATE);
        SetTimer(h, 1, 250, ptr::null_mut());
        ShowWindow(h, 5);
        let mut msg: Msg = std::mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        drop(Box::from_raw(STATE));
        STATE = ptr::null_mut();
    }
}
