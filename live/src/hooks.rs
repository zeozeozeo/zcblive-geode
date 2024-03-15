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
    use crate::{game::PlayLayer, BOT};

    retour::static_detour! {
        pub static INIT_ORIGINAL: unsafe extern "fastcall" fn(PlayLayer, usize, usize, bool, bool);
        pub static DESTRUCTOR_ORIGINAL: unsafe extern "fastcall" fn(PlayLayer);
    }

    pub fn init(this: PlayLayer, _edx: usize, gjgamelevel: usize, a: bool, b: bool) {
        unsafe {
            BOT.on_init(this.addr);
            INIT_ORIGINAL.call(this, 0, gjgamelevel, a, b);
        }
    }

    pub fn destructor(this: PlayLayer) {
        unsafe {
            BOT.on_exit();
            DESTRUCTOR_ORIGINAL.call(this);
        }
    }
}

pub mod player_object {
    use crate::{
        clickpack::Button,
        game::{GameManager, PlayerObject},
        BOT,
    };

    retour::static_detour! {
        pub static PUSH_BUTTON_ORIGINAL: unsafe extern "fastcall" fn(PlayerObject, usize, i32);
        pub static RELEASE_BUTTON_ORIGINAL: unsafe extern "fastcall" fn(PlayerObject, usize, i32);
    }

    unsafe fn handle_push_or_release_button(this: PlayerObject, button: i32, push: bool) {
        if BOT.conf.use_alternate_hook {
            let playlayer = GameManager::shared().play_layer();
            BOT.playlayer = playlayer;
            if playlayer.is_null() {
                return;
            }
            let b = Button::from_u8(button.try_into().unwrap());
            if b.is_platformer() && !this.is_platformer() {
                return;
            }

            let player1 = this == playlayer.player1();
            BOT.on_action(b, !player1, push);
        }
    }

    pub fn push_button(this: PlayerObject, _edx: usize, button: i32) {
        unsafe {
            handle_push_or_release_button(this, button, true);
            PUSH_BUTTON_ORIGINAL.call(this, 0, button);
        }
    }

    pub fn release_button(this: PlayerObject, _edx: usize, button: i32) {
        unsafe {
            handle_push_or_release_button(this, button, false);
            RELEASE_BUTTON_ORIGINAL.call(this, 0, button);
        }
    }
}

pub mod base_game_layer {
    use crate::{clickpack::Button, game::GameManager, BOT};

    retour::static_detour! {
        pub static HANDLE_BUTTON_ORIGINAL: unsafe extern "fastcall" fn (usize, usize, bool, i32, bool);
    }

    pub fn handle_button(this: usize, _edx: usize, push: bool, button: i32, player1: bool) {
        // log::info!("handleButton({push}, {button}, {player1})");
        unsafe {
            if !BOT.conf.use_alternate_hook {
                let playlayer = GameManager::shared().play_layer();
                BOT.playlayer = playlayer;

                if !playlayer.is_null() {
                    let b = Button::from_u8(button.try_into().unwrap());

                    // check if the button is left or right but the player
                    // is not in platformer
                    let is_invalid_platformer = b.is_platformer()
                        && !(player1 && playlayer.player1().is_platformer())
                        && !(!player1 && playlayer.player2().is_platformer());
                    if !is_invalid_platformer {
                        BOT.on_action(b, !player1, push);
                    }
                }
            }
            HANDLE_BUTTON_ORIGINAL.call(this, 0, push, button, player1);
        }
    }
}

pub unsafe fn init_hooks() -> Result<()> {
    {
        use play_layer::*;
        hook!(INIT_ORIGINAL -> init @ *BASE + 0x2dc4a0);
        hook!(DESTRUCTOR_ORIGINAL -> destructor @ *BASE + 0x2dc080);
    }
    {
        use player_object::*;
        hook!(PUSH_BUTTON_ORIGINAL -> push_button @ *BASE + 0x2d1d30);
        hook!(RELEASE_BUTTON_ORIGINAL -> release_button @ *BASE + 0x2d1f70);
    }
    {
        use base_game_layer::*;
        hook!(HANDLE_BUTTON_ORIGINAL -> handle_button @ *BASE + 0x1b69f0);
    }
    Ok(())
}

pub unsafe fn disable_hooks() -> Result<()> {
    {
        use play_layer::*;
        INIT_ORIGINAL.disable()?;
        DESTRUCTOR_ORIGINAL.disable()?;
    }
    {
        use player_object::*;
        PUSH_BUTTON_ORIGINAL.disable()?;
        RELEASE_BUTTON_ORIGINAL.disable()?;
    }
    {
        use base_game_layer::*;
        HANDLE_BUTTON_ORIGINAL.disable()?;
    }
    Ok(())
}
