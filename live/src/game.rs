#![allow(dead_code)]
use crate::hooks::BASE;

macro_rules! impl_default {
    () => {
        pub const NULL: Self = Self { addr: 0 };

        pub const fn is_null(&self) -> bool {
            self.addr == 0
        }

        pub const fn new(addr: usize) -> Self {
            Self { addr: addr }
        }
    };
}

const fn read_mem<T>(address: usize) -> *mut T {
    address as _
}

#[macro_export]
macro_rules! impl_get_set {
    ($varname:ident, $set_varname:ident, $typ:ty, $addr:expr) => {
        #[inline]
        pub fn $varname(&self) -> $typ {
            unsafe { ((self.addr + $addr) as *const $typ).read() }
        }

        #[inline]
        pub fn $set_varname(&self, $varname: $typ) {
            unsafe { ((self.addr + $addr) as *mut $typ).write($varname) }
        }
    };
}

#[derive(PartialEq, Eq, Clone, Copy)]
#[repr(transparent)]
pub struct LevelSettings {
    pub addr: usize,
}

impl LevelSettings {
    impl_default!();
    impl_get_set!(is_2player, set_is_2player, bool, 0x115);
}

#[derive(PartialEq, Eq, Clone, Copy)]
#[repr(transparent)]
pub struct PlayerObject {
    pub addr: usize,
}

impl PlayerObject {
    impl_default!();
    impl_get_set!(is_platformer, set_is_platformer, bool, 0x920);
}

#[derive(PartialEq, Eq, Clone, Copy)]
#[repr(transparent)]
pub struct PlayLayer {
    pub addr: usize,
}

impl PlayLayer {
    impl_default!();
    impl_get_set!(time, set_time, f64, 0x328);
    impl_get_set!(player1, set_player1, PlayerObject, 0x878);
    impl_get_set!(player2, set_player2, PlayerObject, 0x87c);
    impl_get_set!(level_settings, set_level_settings, LevelSettings, 0x880);

    // erm TODO
    impl_get_set!(is_paused, set_is_paused, bool, 0x2f17);
}

#[derive(PartialEq, Eq, Clone, Copy)]
#[repr(transparent)]
pub struct GameManager {
    pub addr: usize,
}

impl GameManager {
    impl_default!();

    pub fn shared() -> Self {
        unsafe {
            Self {
                addr: (std::mem::transmute::<usize, unsafe extern "stdcall" fn() -> usize>(
                    *BASE + 0x121540,
                ))(),
            }
        }
    }

    impl_get_set!(play_layer, set_play_layer, PlayLayer, 0x198);
    // impl_get_set!(player1, set_player1, PlayerObject, 0x878);
    // impl_get_set!(player2, set_player2, PlayerObject, 0x87c);
}

#[derive(PartialEq, Eq, Clone, Copy)]
#[repr(transparent)]
pub struct BaseGameLayer {
    pub addr: usize,
}

impl BaseGameLayer {
    impl_default!();
    
}
