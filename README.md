# ZCB Live for Geode

Implementation details:

- The mod is written in Rust. This is simply a Geode wrapper that calls functions exported from Rust.
- The Rust code **does not** use any hooking framework, and it **does not** write or read any game memory.
- The Geode wrapper embeds the DLL built by Rust inside of its' DLL, writes it to disk and loads it with `LoadLibraryA` (hacky but works).
- The Geode wrapper calls `GetProcAddress` to get the address of the exported functions to later call them inside Geode hooks.
- Only works on Windows, for now.
- **Does not** use FMOD, uses [KittyAudio](https://github.com/zeozeozeo/kittyaudio) instead (based on cpal).
