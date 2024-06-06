mod bot;
mod clickpack;

#[cfg(not(feature = "geode"))]
mod game;

#[cfg(not(feature = "geode"))]
mod hooks;

mod utils;

use bot::{Bot, BOT};
use clickpack::Button;
use once_cell::sync::Lazy;
use retour::static_detour;
use std::{ffi::c_void, sync::Once};
use windows::Win32::{
    Foundation::{BOOL, HMODULE, HWND, LPARAM, LRESULT, TRUE, WPARAM},
    Graphics::Gdi::{WindowFromDC, HDC},
    System::{
        Console::FreeConsole,
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
static mut O_WNDPROC: Option<isize> = None;

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
        log::info!("CallWindowProcW hooked");
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
#[cfg(not(feature = "geode"))]
#[no_mangle]
pub unsafe extern "system" fn DllMain(dll: u64, reason: u32, _reserved: *mut c_void) -> BOOL {
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
            zcblive_uninitialize();
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
            O_WNDPROC = Some(SetWindowLongPtrA(hwnd, GWLP_WNDPROC, h_wndproc as isize));
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

/// Main function, first argument is unused
#[no_mangle]
unsafe extern "system" fn zcblive_main(_hmod: *mut c_void) -> u32 {
    zcblive_initialize();
    1
}

// DLL externs

#[no_mangle]
unsafe extern "C" fn zcblive_initialize() {
    // wait for enter key on panics
    let panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info: &std::panic::PanicInfo<'_>| {
        panic_hook(info);
        let mut string = String::new();
        std::io::stdin().read_line(&mut string).unwrap();
        std::process::exit(1);
    }));

    #[cfg(not(feature = "geode"))]
    BOT.maybe_alloc_console();

    #[cfg(feature = "geode")]
    let _ = simple_logger::SimpleLogger::new().init();

    // get swapbuffers function
    let opengl = GetModuleHandleA(windows::core::s!("OPENGL32.dll")).unwrap();
    let swap_buffers: FnWglSwapBuffers =
        std::mem::transmute(GetProcAddress(opengl, windows::core::s!("wglSwapBuffers")));

    log::info!("wglSwapBuffers: {:#X}", swap_buffers as usize);

    // initialize swapbuffers hook
    if let Ok(detour) = h_wglSwapBuffers
        .initialize(swap_buffers, hk_wgl_swap_buffers)
        .map_err(|e| log::error!("failed to initialize wglSwapBuffers: {e}"))
    {
        detour.enable().unwrap();
    }

    // init bot
    BOT.init();
}

#[no_mangle]
unsafe extern "C" fn zcblive_uninitialize() {
    log::info!("saving config & env before detach...");
    BOT.conf.save();
    BOT.env.save();

    #[cfg(not(feature = "geode"))]
    let _ = hooks::disable_hooks().map_err(|e| log::error!("failed to disable hooks: {e}"));

    if h_wglSwapBuffers
        .disable()
        .map_err(|e| log::error!("failed to disable wglSwapBuffers: {e}"))
        .is_ok()
        && BOT.conf.show_console
    {
        let _ = FreeConsole().map_err(|e| log::error!("FreeConsole failed: {e}"));
    }

    BOT = Lazy::new(Box::<Bot>::default);
}

#[no_mangle]
unsafe extern "C" fn zcblive_on_action(button: u8, player2: bool, push: bool) {
    BOT.on_action(Button::from_u8(button), player2, push);
}

/// optional implementation
#[no_mangle]
unsafe extern "C" fn zcblive_on_reset() {
    BOT.on_reset();
}

#[no_mangle]
unsafe extern "C" fn zcblive_set_is_in_level(is_in_level: bool) {
    BOT.is_in_level = is_in_level;
}

/// optional implementation
#[no_mangle]
unsafe extern "C" fn zcblive_set_playlayer_time(playlayer_time: f64) {
    BOT.playlayer_time = playlayer_time;
}

/// can pass NULL to `playlayer`
#[no_mangle]
unsafe extern "C" fn zcblive_on_init(playlayer: usize) {
    BOT.on_init(playlayer);
}

/// equivalent to passing NULL to `zcblive_on_init`. optional implementation
#[no_mangle]
unsafe extern "C" fn zcblive_on_quit() {
    BOT.on_exit();
}

/// optional implementation
#[no_mangle]
unsafe extern "C" fn zcblive_on_death() {
    BOT.on_death();
}

#[no_mangle]
unsafe extern "C" fn zcblive_do_force_player2_sounds() -> bool {
    BOT.conf.force_player2_sounds
}

#[no_mangle]
unsafe extern "C" fn zcblive_do_use_alternate_hook() -> bool {
    BOT.conf.use_alternate_hook
}

/// required for release buttons on death
#[no_mangle]
unsafe extern "C" fn zcblive_on_update(dt: f32) {
    BOT.on_update(dt);
}
