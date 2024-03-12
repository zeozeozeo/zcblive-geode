use anyhow::Result;
use once_cell::sync::Lazy;

pub static BASE: Lazy<usize> = Lazy::new(|| {
    unsafe {
        ::windows::Win32::System::LibraryLoader::GetModuleHandleA(::windows::core::PCSTR(0 as _))
    }
    .unwrap()
    .0 as usize
});

macro_rules! hook {
    ($original:ident -> $hook:ident @ $addr:expr) => {
        $original
            .initialize(::std::mem::transmute($addr), $hook)?
            .enable()?;
    };
}

pub mod play_layer {
    use crate::BOT;

    retour::static_detour! {
        pub static INIT_ORIGINAL: unsafe extern "fastcall" fn(usize, usize, usize, bool, bool);
    }

    pub fn init(this: usize, _edx: usize, gjgamelevel: usize, a: bool, b: bool) {
        unsafe {
            BOT.on_init(this);
            INIT_ORIGINAL.call(this, 0, gjgamelevel, a, b);
        }
    }
}

pub mod base_game_layer {
    use crate::{clickpack::Button, BOT};

    retour::static_detour! {
        pub static HANDLE_BUTTON_ORIGINAL: unsafe extern "fastcall" fn (usize, usize, bool, i32, bool);
    }

    pub fn handle_button(this: usize, _edx: usize, push: bool, button: i32, player1: bool) {
        log::info!("handleButton({push}, {button}, {player1})");
        unsafe {
            BOT.on_action(Button::from_u8(button.try_into().unwrap()), !player1, push);
            HANDLE_BUTTON_ORIGINAL.call(this, 0, push, button, player1);
        }
    }
}

pub unsafe fn init_hooks() -> Result<()> {
    {
        use play_layer::*;
        hook!(INIT_ORIGINAL -> init @ *BASE + 0x2DC4A0);
    }
    {
        use base_game_layer::*;
        hook!(HANDLE_BUTTON_ORIGINAL -> handle_button @ *BASE + 0x1B69F0);
    }
    Ok(())
}

pub unsafe fn disable_hooks() -> Result<()> {
    {
        use play_layer::*;
        INIT_ORIGINAL.disable()?;
    }
    {
        use base_game_layer::*;
        HANDLE_BUTTON_ORIGINAL.disable()?;
    }
    Ok(())
}
