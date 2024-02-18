use crate::hooks::get_base;
use std::ffi::c_void;

pub struct LevelSettings {
    addr: usize,
}

impl LevelSettings {
    #[inline]
    pub fn is_2player(&self) -> bool {
        unsafe { *(((self.addr) as *const c_void).add(0x115) as *const bool) }
    }
}

pub struct GameManager {
    addr: usize,
}

impl GameManager {
    pub fn shared() -> Self {
        unsafe {
            Self {
                addr: (std::mem::transmute::<usize, unsafe extern "stdcall" fn() -> usize>(
                    get_base() + 0x121540,
                ))(),
            }
        }
    }

    #[inline]
    pub const fn level_settings(&self) -> LevelSettings {
        LevelSettings {
            addr: self.addr + 0x880,
        }
    }
}
