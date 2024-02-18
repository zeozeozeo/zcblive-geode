use crate::{bot::PlayerButton, BOT};
// use geometrydash::{get_base, patch_mem, AddressUtils, GameManager, PlayLayer, PlayerObject, Ptr};
use retour::static_detour;
use std::{ffi::c_void, sync::Once};

//pub const IS_22: bool = true;

//type FnInit = unsafe extern "fastcall" fn(*mut c_void, bool) -> bool;
//type FnQuit = unsafe extern "fastcall" fn(*mut c_void, *mut c_void);
type FnReset = unsafe extern "fastcall" fn(*mut c_void, *mut c_void);
type FnPushButton = unsafe extern "fastcall" fn(*mut c_void, *mut c_void, i32) -> bool;
type FnReleaseButton = unsafe extern "fastcall" fn(*mut c_void, *mut c_void, i32) -> bool;
//type FnUpdate = unsafe extern "fastcall" fn(*mut c_void, *mut c_void, f32);
type FnHandleButton = unsafe extern "fastcall" fn(*mut c_void, *mut c_void, i32, i32, bool) -> i32;

static_detour! {
    static Init: unsafe extern "fastcall" fn(*mut c_void, bool) -> bool;
    static Quit: unsafe extern "fastcall" fn(*mut c_void, *mut c_void);
    static Reset: unsafe extern "fastcall" fn(*mut c_void, *mut c_void);
    static PushButton: unsafe extern "fastcall" fn(*mut c_void, *mut c_void, i32) -> bool;
    static ReleaseButton: unsafe extern "fastcall" fn(*mut c_void, *mut c_void, i32) -> bool;
    static Update: unsafe extern "fastcall" fn (*mut c_void, *mut c_void, f32);
    static HandleButton: unsafe extern "fastcall" fn(*mut c_void, *mut c_void, i32, i32, bool) -> i32;
}

macro_rules! make_minhook_statics {
    ($($static:ident),+) => {
        $(
            #[allow(non_upper_case_globals)]
            static mut $static: *mut ::std::ffi::c_void = 0 as _;
        )*
    };
}

make_minhook_statics!(
    // Init_MinHook,
    // Quit_MinHook,
    Reset_MinHook,
    PushButton_MinHook,
    ReleaseButton_MinHook,
    // Update_MinHook,
    // DestroyPlayer_MinHook,
    HandleButton_MinHook
);

/// Create a function wrapper without a specified calling convention
macro_rules! make_retour_fn {
    ($name:ident, $retour_name:ident($($($n:ident: $t:ty),+)?) $(-> $ret:ty)?) => {
        fn $retour_name($($($n: $t),+)?) $(-> $ret)? {
            unsafe { $name($($($n),+)?) }
        }
    };
}

macro_rules! call_hook {
    ($static:ident($($arg:expr),+)) => {
        if unsafe { BOT.used_minhook } {
            unsafe {
                ::std::mem::transmute::<*mut ::std::ffi::c_void, concat_idents!(Fn, $static)>(concat_idents!($static, _MinHook))($($arg),+) }
        } else {
            unsafe { $static.call($($arg),+) }
        }
    };
}

//#[inline]
//fn get_game_manager() -> usize {
//    unsafe {
//        (std::mem::transmute::<usize, unsafe extern "stdcall" fn() -> usize>(get_base() + 1206560))(
//        )
//    }
//}
//
//fn get_game_variable(var: &str) -> bool {
//    let var = std::ffi::CString::new(var).unwrap(); // convert to c string
//    unsafe {
//        (std::mem::transmute::<usize, unsafe extern "fastcall" fn(usize, usize, *const u8) -> bool>(
//            get_base() + 5145320,
//        ))(get_game_manager(), 0, var.as_ptr() as *const u8)
//    }
//}

//unsafe extern "fastcall" fn init(playlayer: *mut c_void, something: bool) -> bool {
//    let res = call_hook!(Init(playlayer, something));
//    log::debug!("init");
//    unsafe { BOT.playlayer = playlayer };
//    unsafe { BOT.on_init() };
//    res
//}
//
//make_retour_fn!(init, init_retour(gamelevel: *mut c_void, dead: bool) -> bool);
//
//unsafe extern "fastcall" fn quit(playlayer: *mut c_void, _edx: *mut c_void) {
//    call_hook!(Quit(playlayer, std::ptr::null_mut()));
//
//    // set playlayer to null
//    unsafe { BOT.playlayer = std::ptr::null_mut() };
//}
//
//make_retour_fn!(quit, quit_retour(playlayer: *mut c_void, _edx: *mut c_void));

unsafe extern "fastcall" fn reset(playlayer: *mut c_void, _edx: *mut c_void) {
    call_hook!(Reset(playlayer, std::ptr::null_mut()));

    if unsafe { BOT.playlayer.is_null() } && !playlayer.is_null() {
        log::debug!("reset init");
        unsafe { BOT.on_init() };
    }
    unsafe { BOT.playlayer = playlayer };

    log::debug!("reset");
    unsafe { BOT.on_reset() };
}

make_retour_fn!(reset, reset_retour(playlayer: *mut c_void, _edx: *mut c_void));

unsafe extern "fastcall" fn push_button(
    player: *mut c_void,
    _edx: *mut c_void,
    button: i32,
) -> bool {
    let res = call_hook!(PushButton(player, std::ptr::null_mut(), button));
    //log::info!("pbutton: {button}");
    // let is_p2 = BOT.is_player2_obj(player);
    // let is_2player = BOT.is_2player();

    BOT.on_action(PlayerButton::Push, BOT.is_player2_obj(player));
    res
}

make_retour_fn!(push_button, push_button_retour(player: *mut c_void, _edx: *mut c_void, button: i32) -> bool);

unsafe extern "fastcall" fn release_button(
    player: *mut c_void,
    _edx: *mut c_void,
    button: i32,
) -> bool {
    let res = call_hook!(ReleaseButton(player, std::ptr::null_mut(), button));
    BOT.on_action(PlayerButton::Release, BOT.is_player2_obj(player));
    res
}

make_retour_fn!(release_button, release_button_retour(player: *mut c_void, _edx: *mut c_void, button: i32) -> bool);

unsafe extern "fastcall" fn handle_button(
    basegamelayer: *mut c_void,
    _edx: *mut c_void,
    push: i32,
    button: i32,
    is_player1: bool,
) -> i32 {
    let res = call_hook!(HandleButton(
        basegamelayer,
        std::ptr::null_mut(),
        push,
        button,
        is_player1
    ));
    // log::info!("handle_button: {push}, {button}, {is_player1}");
    unsafe {
        BOT.on_action(
            PlayerButton::from_u8(push as u8).unwrap_or_default(),
            !is_player1,
        )
    };
    res
}

make_retour_fn!(
    handle_button,
    handle_button_retour(
        basegamelayer: *mut c_void,
        _edx: *mut c_void,
        a2: i32,
        a3: i32,
        button: bool
    ) -> i32
);

//macro_rules! patch {
//    ($addr:expr, $data:expr) => {
//        let len = $data.len();
//        let _ = patch_mem($addr, $data)
//            .map_err(|e| log::error!("failed to write {len} bytes at {:#x}: {e}", $addr));
//    };
//}

// unsafe extern "fastcall" fn update(basegamelayer: *mut c_void, _edx: *mut c_void, dt: f32) {
//     call_hook!(Update(basegamelayer, std::ptr::null_mut(), dt));
//     unsafe { BOT.restarted_ago = BOT.restarted_ago.saturating_add(1) };
// }
//
// make_retour_fn!(update, update_retour(basegamelayer: *mut c_void, _edx: *mut c_void, dt: f32));

/// GetModuleHandle(NULL)
#[inline]
pub fn get_base() -> usize {
    static ONCE: Once = Once::new();
    static mut BASE: usize = 0;
    ONCE.call_once(|| {
        use windows::core::PCSTR;
        use windows::Win32::System::LibraryLoader::GetModuleHandleA;
        unsafe {
            BASE = {
                let hmod = GetModuleHandleA(PCSTR(std::ptr::null())).unwrap();
                hmod.0 as usize
            };
        }
    });
    unsafe { BASE }
}

/// Copies the given data to the given address in memory.
//fn patch_mem(address: usize, data: &[u8]) -> windows::core::Result<()> {
//    use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;
//    use windows::Win32::System::Memory::{
//        VirtualProtectEx, PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS,
//    };
//    use windows::Win32::System::Threading::GetCurrentProcess;
//    unsafe {
//        let mut old_prot = PAGE_PROTECTION_FLAGS(0);
//        VirtualProtectEx(
//            GetCurrentProcess(),
//            address as _,
//            256,
//            PAGE_EXECUTE_READWRITE,
//            &mut old_prot as _,
//        )?;
//        WriteProcessMemory(
//            GetCurrentProcess(),
//            address as _,
//            data.as_ptr() as _,
//            data.len(),
//            None,
//        )
//    }
//}

macro_rules! hook {
    ($static:ident, $detour:ident, $addr:expr) => {
        let addr = get_base() + $addr;
        if unsafe { BOT.used_minhook } {
            ::log::info!("creating minhook hook: {} -> {:#x}", stringify!($static), $addr);
            concat_idents!($static, _MinHook) =
                ::minhook::MinHook::create_hook(addr as _, $detour as _)
                    .expect(stringify!(failed to hook $static with minhook));
        } else {
            ::log::info!("initializing retour hook: {} -> {:#x}", stringify!($static), $addr);
            $static
                .initialize(::std::mem::transmute(addr), concat_idents!($detour, _retour))
                .expect(stringify!(failed to hook $static with retour));
            ::log::info!("enabling retour hook: {} -> {:#x}", stringify!($static), $addr);
            $static
                .enable()
                .expect(stringify!(failed to enable $static retour hook));
        }
    };
}

pub unsafe fn init_hooks() {
    // std::thread::sleep(std::time::Duration::from_secs(2));
    log::warn!(">>>>>> If you are using Geode, YOU SHOULDN'T SEE THIS!!!");

    if unsafe { BOT.used_alternate_hook } {
        hook!(HandleButton, handle_button, 0x1b69f0);
    } else {
        hook!(PushButton, push_button, 0x2D1F70 - 0x240);
        hook!(ReleaseButton, release_button, 0x2D1F70);
    }
    // hook!(Init, init, 0x18cc80);
    hook!(Reset, reset, 0x2EA130);
    // hook!(Update, update, 0x1BA700);

    if unsafe { BOT.used_minhook } {
        log::info!("enabling all minhook hooks");
        unsafe { minhook::MinHook::enable_all_hooks().expect("failed to enable hooks") };
    }
}

macro_rules! disable_hooks {
    ($($static:ident),+) => {
        $(
            if unsafe { BOT.used_minhook } {
                log::info!("disabling {} minhook hook", stringify!($static));
                let _ = ::minhook::MinHook::disable_hook(concat_idents!($static, _MinHook))
                    .map_err(|e| ::log::error!("failed to disable {} minhook hook: {e:?}", stringify!($static)));
            } else {
                log::info!("disabling {} retour hook", stringify!($static));
                let _ = unsafe { $static.disable() }
                    .map_err(|e| ::log::error!("failed to disable {} hook: {e}", stringify!($static)));
            }
        )*
    };
}

pub unsafe fn disable_hooks() {
    log::info!("disabling hooks");

    disable_hooks!(Reset);
    if unsafe { BOT.used_alternate_hook } {
        disable_hooks!(HandleButton);
    } else {
        disable_hooks!(PushButton, ReleaseButton);
    }

    if unsafe { BOT.used_minhook } {
        log::info!("uninitializing minhook");
        minhook::MinHook::uninitialize();
    }
}
