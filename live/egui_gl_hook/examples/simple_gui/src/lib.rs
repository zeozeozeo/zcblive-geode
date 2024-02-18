// warning terrible code :alert:

use std::os::raw::c_void;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{WindowFromDC, HDC};
use windows::Win32::System::{
    Console::AllocConsole,
    LibraryLoader::{GetModuleHandleA, GetProcAddress},
    SystemServices::DLL_PROCESS_ATTACH,
    Threading::{CreateThread, THREAD_CREATION_FLAGS},
};
use windows::Win32::UI::WindowsAndMessaging::{CallWindowProcA, SetWindowLongPtrA, GWLP_WNDPROC};

use retour::static_detour;

#[no_mangle]
pub unsafe extern "system" fn DllMain(dll: u32, reason: u32, _reserved: *mut c_void) -> u32 {
    if reason == DLL_PROCESS_ATTACH {
        //DLL_PROCESS_ATTACH
        CreateThread(
            None,
            0,
            Some(extension_main),
            Some(dll as _),
            THREAD_CREATION_FLAGS(0),
            None,
        )
        .unwrap();
    }
    1
}

static_detour! {
    static h_wglSwapBuffers: unsafe extern "system" fn(HDC) -> i32;
}

type FnWglSwapBuffers = unsafe extern "system" fn(HDC) -> i32;

static mut O_WNDPROC: Option<i32> = None;
static mut GUI_STATE: GuiState = GuiState {
    text: String::new(),
    checked: false,
};

#[derive(Default)]
struct GuiState {
    text: String,
    checked: bool,
}

unsafe extern "system" fn h_wndproc(
    hwnd: HWND,
    umsg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if egui_gl_hook::is_init() {
        let should_skip_wnd_proc = egui_gl_hook::on_event(umsg, wparam.0, lparam.0).unwrap();

        if should_skip_wnd_proc {
            return LRESULT(1);
        }
    }

    CallWindowProcA(
        std::mem::transmute(O_WNDPROC.unwrap()),
        hwnd,
        umsg,
        wparam,
        lparam,
    )
}

unsafe extern "system" fn extension_main(_dll: *mut c_void) -> u32 {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info: &std::panic::PanicInfo<'_>| {
        hook(info);
        let mut string = String::new();
        std::io::stdin().read_line(&mut string).unwrap();
        std::process::exit(1);
    }));

    AllocConsole().unwrap();

    let opengl = GetModuleHandleA(windows::core::s!("OPENGL32.dll")).unwrap();
    let swap_buffers: FnWglSwapBuffers =
        std::mem::transmute(GetProcAddress(opengl, windows::core::s!("wglSwapBuffers")));

    let (sx, rx) = std::sync::mpsc::channel();

    h_wglSwapBuffers
        .initialize(swap_buffers, move |hdc| {
            if hdc == HDC(0) {
                return h_wglSwapBuffers.call(hdc);
            }

            if !egui_gl_hook::is_init() {
                sx.send(hdc).unwrap();
                egui_gl_hook::init(hdc).unwrap();
            }

            egui_gl_hook::paint(
                hdc,
                Box::new(|ctx| {
                    let gui = &mut GUI_STATE;
                    egui::Window::new("hi").collapsible(false).show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("pls?");
                            ui.text_edit_singleline(&mut gui.text);
                        });
                        let _ = ui.button("wowie");
                        ui.checkbox(&mut gui.checked, "poop");
                    });
                }),
            )
            .unwrap();
            //println!("hi");
            return h_wglSwapBuffers.call(hdc);
        })
        .unwrap()
        .enable()
        .unwrap();

    let hdc = rx.recv().unwrap();
    let hwnd = WindowFromDC(hdc);

    O_WNDPROC = Some(SetWindowLongPtrA(hwnd, GWLP_WNDPROC, h_wndproc as _));

    0
}
