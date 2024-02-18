#![feature(concat_idents)]

mod bot;

// #[cfg(not(feature = "geode"))]
// mod game_manager;

#[cfg(not(feature = "geode"))]
mod hooks;

mod utils;

use bot::{PlayerButton, BOT};
use retour::static_detour;
use std::{ffi::c_void, sync::Once};
use windows::Win32::{
    Foundation::{BOOL, HMODULE, HWND, LPARAM, LRESULT, TRUE, WPARAM},
    Graphics::Gdi::{WindowFromDC, HDC},
    System::{
        LibraryLoader::{FreeLibraryAndExitThread, GetModuleHandleA, GetProcAddress},
        SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
        Threading::{CreateThread, THREAD_CREATION_FLAGS},
    },
    UI::WindowsAndMessaging::{CallWindowProcA, SetWindowLongPtrA, GWLP_WNDPROC},
};

// wglSwapBuffers detour
static_detour! {
    static h_wglSwapBuffers: unsafe extern "system" fn(HDC) -> i32;
}

/// wglSwapBuffers function type
type FnWglSwapBuffers = unsafe extern "system" fn(HDC) -> i32;

/// returned from SetWindowLongPtrA
static mut O_WNDPROC: Option<i32> = None;

/// WNDPROC hook
#[no_mangle]
unsafe extern "system" fn h_wndproc(
    hwnd: HWND,
    umsg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        log::info!("CallWindowProcW hooked (new)");
    });

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

/// DLL entrypoint
///
/// # Safety
#[no_mangle]
pub unsafe extern "system" fn DllMain(dll: u32, reason: u32, _reserved: *mut c_void) -> BOOL {
    match reason {
        DLL_PROCESS_ATTACH => {
            CreateThread(
                None,
                0,
                Some(zcblive_main),
                Some(dll as _),
                THREAD_CREATION_FLAGS(0),
                None,
            )
            .unwrap();
        }
        DLL_PROCESS_DETACH => {
            #[cfg(not(feature = "geode"))]
            hooks::disable_hooks();
            FreeLibraryAndExitThread(std::mem::transmute::<_, HMODULE>(dll), 0);
        }
        _ => {}
    }
    TRUE
}

fn hk_wgl_swap_buffers(hdc: HDC) -> i32 {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        log::info!("wglSwapBuffers hooked");
    });

    unsafe {
        if hdc == HDC(0) {
            return h_wglSwapBuffers.call(hdc);
        }

        // initialize egui_gl_hook
        if !egui_gl_hook::is_init() {
            let hwnd = WindowFromDC(hdc);
            O_WNDPROC = Some(SetWindowLongPtrA(
                hwnd,
                GWLP_WNDPROC,
                h_wndproc as usize as i32,
            ));
            egui_gl_hook::init(hdc).unwrap();
        }

        // paint this frame
        let _ = egui_gl_hook::paint(
            hdc,
            Box::new(|ctx| {
                BOT.draw_ui(ctx);
            }),
        )
        .map_err(|e| log::error!("paint() failed: {e}"));
        h_wglSwapBuffers.call(hdc)
    }
}

/// Main function
#[no_mangle]
unsafe extern "system" fn zcblive_main(_hmod: *mut c_void) -> u32 {
    // wait for enter key on panics
    let panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info: &std::panic::PanicInfo<'_>| {
        panic_hook(info);
        let mut string = String::new();
        std::io::stdin().read_line(&mut string).unwrap();
        std::process::exit(1);
    }));

    BOT.maybe_alloc_console();

    // get swapbuffers function
    let opengl = GetModuleHandleA(windows::core::s!("OPENGL32.dll")).unwrap();
    let swap_buffers: FnWglSwapBuffers =
        std::mem::transmute(GetProcAddress(opengl, windows::core::s!("wglSwapBuffers")));

    log::info!("wglSwapBuffers: {:#X}", swap_buffers as usize);

    // initialize swapbuffers hook
    h_wglSwapBuffers
        .initialize(swap_buffers, hk_wgl_swap_buffers)
        .unwrap()
        .enable()
        .unwrap();

    // init bot
    BOT.init();
    1
}

// functions for other mods to call if they need it for some reason
// also used for geode

#[no_mangle]
unsafe extern "system" fn zcblive_on_action(button: u8, player2: bool) {
    if let Some(button) = PlayerButton::from_u8(button) {
        BOT.on_action(button, player2)
    }
}

#[no_mangle]
#[allow(unused_variables)]
unsafe extern "system" fn zcblive_set_playlayer(playlayer: *mut c_void /*PlayLayer*/) {
    #[cfg(not(feature = "geode"))]
    {
        BOT.playlayer = playlayer;
    }
}

#[no_mangle]
unsafe extern "system" fn zcblive_set_is_in_level(is_in_level: bool) {
    BOT.is_in_level = is_in_level;
}

#[no_mangle]
unsafe extern "system" fn zcblive_set_playlayer_time(playlayer_time: f64) {
    BOT.playlayer_time = playlayer_time;
}

#[no_mangle]
unsafe extern "system" fn zcblive_on_playlayer_init() {
    BOT.on_init()
}

#[no_mangle]
unsafe extern "system" fn zcblive_on_basegamelayer_reset() {
    BOT.on_reset()
}
