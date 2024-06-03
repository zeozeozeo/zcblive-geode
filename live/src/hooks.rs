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
        let address = $addr;
        ::log::info!(
            "hooking {} -> {} @ {address:#x}",
            stringify!($original),
            stringify!($hook)
        );
        $original
            .initialize(::std::mem::transmute(address), $hook)?
            .enable()?;
    };
}

pub mod play_layer {
    use crate::{
        game::{PlayLayer, PlayerObject},
        BOT,
    };

    retour::static_detour! {
        pub static RESET_LEVEL_ORIGINAL: unsafe extern "fastcall" fn(PlayLayer);
        pub static DESTROY_PLAYER_ORIGINAL: unsafe extern "fastcall" fn(PlayLayer, usize, PlayerObject, usize);
        pub static ON_QUIT_ORIGINAL: unsafe extern "fastcall" fn(PlayLayer);
    }

    pub fn reset_level(this: PlayLayer) {
        unsafe {
            BOT.on_reset();
            RESET_LEVEL_ORIGINAL.call(this);
        }
    }

    pub fn destroy_player(this: PlayLayer, _edx: usize, player: PlayerObject, hit: usize) {
        unsafe {
            DESTROY_PLAYER_ORIGINAL.call(this, 0, player, hit);

            // check for noclip
            if player.is_dead() {
                BOT.on_death();
            }
        }
    }

    pub fn on_quit(this: PlayLayer) {
        unsafe {
            BOT.on_exit();
            ON_QUIT_ORIGINAL.call(this);
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
            let game_manager = GameManager::shared();
            let playlayer = game_manager.play_layer();
            BOT.playlayer = playlayer;
            if playlayer.is_null() && game_manager.level_editor_layer().is_null() {
                return;
            }
            let b = Button::from_u8(button.try_into().unwrap());
            if b.is_platformer() && !this.is_platformer() {
                return;
            }

            let player1 = !playlayer.is_null() && this == playlayer.player1();
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
        //pub static DESTRUCTOR_ORIGINAL: unsafe extern "fastcall" fn(usize);
        pub static INIT_ORIGINAL: unsafe extern "fastcall" fn (usize);
        pub static HANDLE_BUTTON_ORIGINAL: unsafe extern "fastcall" fn (usize, usize, bool, i32, bool);
        pub static UPDATE_ORIGINAL: unsafe extern "fastcall" fn (usize, usize, f32);
    }

    // pub fn destructor(this: usize) {
    //     unsafe {
    //         BOT.on_exit();
    //         DESTRUCTOR_ORIGINAL.call(this);
    //     }
    // }

    pub fn init(this: usize) {
        unsafe {
            BOT.on_init(GameManager::shared().play_layer().addr);
            INIT_ORIGINAL.call(this);
        }
    }

    pub fn handle_button(this: usize, _edx: usize, push: bool, button: i32, player1: bool) {
        unsafe {
            if !BOT.conf.use_alternate_hook {
                let playlayer = GameManager::shared().play_layer();
                BOT.playlayer = playlayer;

                let b = Button::from_u8(button.try_into().unwrap());

                // check if the button is left or right but the player
                // is not in platformer
                let is_invalid_platformer = !playlayer.is_null()
                    && b.is_platformer()
                    && !(player1 && playlayer.player1().is_platformer())
                    && (player1 || !playlayer.player2().is_platformer());
                if !is_invalid_platformer {
                    BOT.on_action(b, !player1, push);
                }
            }
            HANDLE_BUTTON_ORIGINAL.call(this, 0, push, button, player1);
        }
    }

    pub fn update(this: usize, _edx: usize, dt: f32) {
        unsafe {
            BOT.on_update(dt);
            UPDATE_ORIGINAL.call(this, 0, dt);
        }
    }
}

pub unsafe fn init_hooks() -> Result<()> {
    {
        use play_layer::*;
        hook!(RESET_LEVEL_ORIGINAL -> reset_level @ *BASE + 0x3958b0);
        hook!(DESTROY_PLAYER_ORIGINAL -> destroy_player @ *BASE + 0x3905a0);
        hook!(ON_QUIT_ORIGINAL -> on_quit @ *BASE + 0x397540);
    }
    {
        use player_object::*;
        hook!(PUSH_BUTTON_ORIGINAL -> push_button @ *BASE + 0x375f70);
        hook!(RELEASE_BUTTON_ORIGINAL -> release_button @ *BASE + 0x376200);
    }
    {
        use base_game_layer::*;
        //hook!(DESTRUCTOR_ORIGINAL -> destructor @ *BASE + 0x2dc080);
        hook!(INIT_ORIGINAL -> init @ *BASE + 0x1f7dd0);
        hook!(HANDLE_BUTTON_ORIGINAL -> handle_button @ *BASE + 0x2238a0);
        hook!(UPDATE_ORIGINAL -> update @ *BASE + 0x2277d0);
    }
    log::info!("all hooks initialized!");
    Ok(())
}

pub unsafe fn disable_hooks() -> Result<()> {
    log::info!("disabling hooks");
    {
        use play_layer::*;
        RESET_LEVEL_ORIGINAL.disable()?;
        DESTROY_PLAYER_ORIGINAL.disable()?;
        ON_QUIT_ORIGINAL.disable()?;
    }
    {
        use player_object::*;
        PUSH_BUTTON_ORIGINAL.disable()?;
        RELEASE_BUTTON_ORIGINAL.disable()?;
    }
    {
        use base_game_layer::*;
        //DESTRUCTOR_ORIGINAL.disable()?;
        INIT_ORIGINAL.disable()?;
        HANDLE_BUTTON_ORIGINAL.disable()?;
        UPDATE_ORIGINAL.disable()?;
    }
    log::info!("all hooks disabled");
    Ok(())
}
