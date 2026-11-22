#![allow(static_mut_refs)] // bleh!!

use clipboard::{windows_clipboard::WindowsClipboardContext, ClipboardProvider};
use egui::{Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};
use std::sync::Arc;

mod gl_state;
use gl_state::{GetVertexAttribIv, GlState};
use windows::{
    Wdk::System::SystemInformation::NtQuerySystemTime,
    Win32::{
        Foundation::RECT,
        Graphics::{
            Gdi::{WindowFromDC, HDC},
            OpenGL::{
                wglCreateContext, wglDeleteContext, wglGetCurrentContext, wglGetProcAddress,
                wglMakeCurrent, HGLRC,
            },
        },
        System::{
            LibraryLoader::{GetModuleHandleA, GetProcAddress},
            SystemServices::{MK_CONTROL, MK_SHIFT},
        },
        UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};

/// How the overlay gets a GL context to draw with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlMode {
    /// create our own context and switch to it (with `wglMakeCurrent`) every frame.
    /// this is what has always been done on Windows.
    Separate,
    /// draw inside the game's current context, saving and restoring the GL state we touch.
    /// used under Wine (CrossOver, Whisky, Proton...), where switching contexts on the
    /// game's window every frame breaks presentation (on macOS the window stops showing).
    Shared,
}

impl GlMode {
    /// picks the mode for this machine. `ZCBLIVE_GL_MODE=shared|separate` overrides it.
    pub fn detect() -> Self {
        match std::env::var("ZCBLIVE_GL_MODE")
            .map(|v| v.to_ascii_lowercase())
            .as_deref()
        {
            Ok("shared") => return Self::Shared,
            Ok("separate") => return Self::Separate,
            _ => {}
        }
        if wine_version().is_some() {
            Self::Shared
        } else {
            Self::Separate
        }
    }
}

/// returns the Wine version if running under Wine (or something based on it, like CrossOver)
pub fn wine_version() -> Option<String> {
    unsafe {
        let ntdll = GetModuleHandleA(windows::core::s!("ntdll.dll")).ok()?;
        let f = GetProcAddress(ntdll, windows::core::s!("wine_get_version"))?;
        let f: unsafe extern "C" fn() -> *const std::ffi::c_char = std::mem::transmute(f);
        let ptr = f();
        if ptr.is_null() {
            return Some("unknown".to_string());
        }
        Some(std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned())
    }
}

struct EguiState {
    egui_ctx: egui::Context,
    painter: egui_glow::Painter,
    gl: Arc<egui_glow::glow::Context>,
    mode: GlMode,
    has_vao: bool,
    get_vertex_attrib_iv: Option<GetVertexAttribIv>,
    events: Vec<egui::Event>,
    modifiers: Option<Modifiers>,
    window_handle: HDC,
    original_gl_context: HGLRC,
    new_gl_context: HGLRC,
    is_changing_scale: bool,
    prev_scale: f32,
}

static mut STATE: Option<EguiState> = None; // unsafe, sure, but also way easier to make work

pub fn set_changing_scale(is_changing_scale: bool, prev_scale: f32) {
    unsafe {
        if let Some(state) = &mut STATE {
            state.is_changing_scale = is_changing_scale;
            if is_changing_scale {
                state.prev_scale = prev_scale;
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("state was not initialized")]
    NotInit,
    #[error("state was already initialized")]
    AlreadyInit,

    #[error("couldn't lock state")]
    StateLock,
    #[error("couldn't create state")]
    StateCreation,

    #[error("failed to create gl context")]
    CtxCreate,
    #[error("failed to switch gl context")]
    CtxSwitch,
    #[error("failed to delete gl context")]
    CtxDelete,

    #[error("failed to get window size")]
    WindowSize,

    #[error("could not create painter: `{0}`")]
    PainterError(#[from] egui_glow::PainterError),
}

/// should be called when exiting to remove gl objects and such
pub fn destroy() -> Result<(), Error> {
    let state = unsafe {
        match &mut STATE {
            Some(s) => s,
            None => return Err(Error::NotInit),
        }
    };

    if state.mode == GlMode::Separate {
        unsafe {
            let _ = wglDeleteContext(state.new_gl_context);
        }
    }

    state.painter.destroy();

    Ok(())
}

/// checks if initialized
pub fn is_init() -> bool {
    unsafe { &STATE }.is_some()
}

/// initializes state; needed to be called before paint, on_event, get_window_rect, and destroy
/// looks up a GL function, first with `wglGetProcAddress`, then in opengl32.dll
unsafe fn gl_proc_address(name: &std::ffi::CStr) -> *const std::ffi::c_void {
    let name = windows::core::PCSTR::from_raw(name.as_ptr() as _);
    if let Some(f) = wglGetProcAddress(name) {
        return f as *const _;
    }
    // if that fails, use normal GetProcAddress (yes this is necessary)
    match GetModuleHandleA(windows::core::s!("OPENGL32.dll")) {
        Ok(opengl32) => GetProcAddress(opengl32, name)
            .map(|f| f as *const std::ffi::c_void)
            .unwrap_or(std::ptr::null()),
        Err(_) => std::ptr::null(),
    }
}

/// initializes state; needed to be called before paint, on_event, get_window_rect, and destroy
pub unsafe fn init(window_handle: HDC, mode: GlMode) -> Result<(), Error> {
    if is_init() {
        return Err(Error::AlreadyInit);
    };

    let original_gl_context = wglGetCurrentContext();
    let new_gl_context = match mode {
        // draw with the game's own context
        GlMode::Shared => original_gl_context,
        GlMode::Separate => match wglCreateContext(window_handle) {
            Ok(gl) => gl,
            Err(_) => return Err(Error::CtxCreate),
        },
    };

    // not sure if you need to change the gl context for initialization, but it doesn't hurt right?
    if mode == GlMode::Separate && wglMakeCurrent(window_handle, new_gl_context).is_err() {
        let _ = wglDeleteContext(new_gl_context);
        return Err(Error::CtxSwitch);
    }

    let gl = Arc::new(unsafe {
        egui_glow::glow::Context::from_loader_function_cstr(|s| gl_proc_address(s))
    });
    let has_vao = gl_state::supports_vao(&gl);
    let get_vertex_attrib_iv = {
        let f = gl_proc_address(c"glGetVertexAttribiv");
        (!f.is_null()).then(|| std::mem::transmute::<_, GetVertexAttribIv>(f))
    };

    // in shared mode, painter creation binds buffers/textures in the game's context too
    let saved = (mode == GlMode::Shared).then(|| GlState::save(&gl, has_vao, get_vertex_attrib_iv));
    let painter = egui_glow::Painter::new(gl.clone(), "", None);
    if let Some(saved) = &saved {
        saved.restore(&gl);
    }

    if mode == GlMode::Separate {
        let _ = wglMakeCurrent(window_handle, original_gl_context);
    }

    let painter = match painter {
        Ok(p) => p,
        Err(err) => {
            if mode == GlMode::Separate {
                let _ = wglDeleteContext(new_gl_context);
            }
            return Err(err.into());
        }
    };

    let egui_ctx = egui::Context::default();

    STATE = Some(EguiState {
        egui_ctx,
        painter,
        gl,
        mode,
        has_vao,
        get_vertex_attrib_iv,
        events: Vec::new(),
        modifiers: None,
        window_handle,
        original_gl_context,
        new_gl_context,
        is_changing_scale: false,
        prev_scale: 1.0,
    });

    Ok(())
}

/// runs ui function and makes opengl calls to render to specified window
pub unsafe fn paint(hdc: HDC, run_fn: Box<dyn Fn(&egui::Context)>) -> Result<(), Error> {
    let state = unsafe {
        match &mut STATE {
            Some(s) => s,
            None => return Err(Error::NotInit),
        }
    };

    state.window_handle = hdc;

    let saved = match state.mode {
        GlMode::Separate => {
            // always remember the game's context, it may have been recreated
            // (e.g. after toggling fullscreen)
            state.original_gl_context = wglGetCurrentContext();
            if wglMakeCurrent(state.window_handle, state.new_gl_context).is_err() {
                return Err(Error::CtxSwitch);
            }
            None
        }
        GlMode::Shared => Some(GlState::save(
            &state.gl,
            state.has_vao,
            state.get_vertex_attrib_iv,
        )),
    };
    let result = paint_inner(state, run_fn);

    match saved {
        Some(saved) => saved.restore(&state.gl),
        None => {
            if wglMakeCurrent(state.window_handle, state.original_gl_context).is_err() {
                return Err(Error::CtxSwitch);
            }
        }
    }

    result
}

unsafe fn paint_inner(
    state: &mut EguiState,
    run_fn: Box<dyn Fn(&egui::Context)>,
) -> Result<(), Error> {

    let raw_input = get_raw_input(state)?;
    // let dpi = match GetDpiForWindow(WindowFromDC(state.window_handle)) {
    //     0 => 96.0,
    //     dpi => dpi as f32,
    // };
    // let pixels_per_point = dpi / 96.0;

    let egui::FullOutput {
        platform_output,
        mut textures_delta,
        shapes,
        pixels_per_point,
        viewport_output: _,
    } = state.egui_ctx.run(raw_input, &*run_fn); // run through ui and get output

    if let Some(open_url) = platform_output.open_url {
        let _ = open::that(open_url.url).map_err(|e| eprintln!("failed to open url: {e}"));
    }
    if !platform_output.copied_text.is_empty() {
        let _ = WindowsClipboardContext
            .set_contents(platform_output.copied_text.clone())
            .map_err(|e| {
                eprintln!(
                    "failed to set clipboard contents `{}`: {e}",
                    platform_output.copied_text
                )
            });
    }

    for (id, image_delta) in textures_delta.set {
        state.painter.set_texture(id, &image_delta);
    }

    // convert to meshes
    let clipped_primitives = state.egui_ctx.tessellate(shapes, pixels_per_point);
    let dimensions = get_screen_size()?;

    state.painter.paint_primitives(
        [dimensions.0, dimensions.1],
        pixels_per_point,
        &clipped_primitives,
    ); // actual opengl calls to render

    for id in textures_delta.free.drain(..) {
        state.painter.free_texture(id);
    }

    Ok(())
}

/// returns if you should skip calling original wndproc
pub fn on_event(umsg: u32, wparam: usize, lparam: isize) -> Result<bool, Error> {
    let state = unsafe {
        match &mut STATE {
            Some(s) => s,
            None => return Err(Error::NotInit),
        }
    };

    match umsg {
        WM_MOUSEMOVE => {
            alter_modifiers(state, get_mouse_modifiers(wparam));

            state.events.push(Event::PointerMoved(get_pos(lparam)));
        }
        WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => {
            let modifiers = get_mouse_modifiers(wparam);
            alter_modifiers(state, modifiers);

            state.events.push(Event::PointerButton {
                pos: get_pos(lparam),
                button: PointerButton::Primary,
                pressed: true,
                modifiers,
            });
        }
        WM_LBUTTONUP => {
            let modifiers = get_mouse_modifiers(wparam);
            alter_modifiers(state, modifiers);

            state.events.push(Event::PointerButton {
                pos: get_pos(lparam),
                button: PointerButton::Primary,
                pressed: false,
                modifiers,
            });
        }
        WM_RBUTTONDOWN | WM_RBUTTONDBLCLK => {
            let modifiers = get_mouse_modifiers(wparam);
            alter_modifiers(state, modifiers);

            state.events.push(Event::PointerButton {
                pos: get_pos(lparam),
                button: PointerButton::Secondary,
                pressed: true,
                modifiers,
            });
        }
        WM_RBUTTONUP => {
            let modifiers = get_mouse_modifiers(wparam);
            alter_modifiers(state, modifiers);

            state.events.push(Event::PointerButton {
                pos: get_pos(lparam),
                button: PointerButton::Secondary,
                pressed: false,
                modifiers,
            });
        }
        WM_MBUTTONDOWN | WM_MBUTTONDBLCLK => {
            let modifiers = get_mouse_modifiers(wparam);
            alter_modifiers(state, modifiers);

            state.events.push(Event::PointerButton {
                pos: get_pos(lparam),
                button: PointerButton::Middle,
                pressed: true,
                modifiers,
            });
        }
        WM_MBUTTONUP => {
            let modifiers = get_mouse_modifiers(wparam);
            alter_modifiers(state, modifiers);

            state.events.push(Event::PointerButton {
                pos: get_pos(lparam),
                button: PointerButton::Middle,
                pressed: false,
                modifiers,
            });
        }
        WM_XBUTTONDOWN | WM_XBUTTONDBLCLK => {
            let modifiers = get_mouse_modifiers(wparam);
            alter_modifiers(state, modifiers);

            state.events.push(Event::PointerButton {
                pos: get_pos(lparam),
                button: if (wparam as u32) >> 16u32 & XBUTTON1 as u32 != 0u32 {
                    PointerButton::Extra1
                } else if (wparam as u32) >> 16u32 & XBUTTON2 as u32 != 0u32 {
                    PointerButton::Extra2
                } else {
                    unreachable!()
                },
                pressed: true,
                modifiers,
            });
        }
        WM_XBUTTONUP => {
            let modifiers = get_mouse_modifiers(wparam);
            alter_modifiers(state, modifiers);

            state.events.push(Event::PointerButton {
                pos: get_pos(lparam),
                button: if (wparam as u32) >> 16u32 & XBUTTON1 as u32 != 0u32 {
                    PointerButton::Extra1
                } else if (wparam as u32) >> 16u32 & XBUTTON2 as u32 != 0u32 {
                    PointerButton::Extra2
                } else {
                    unreachable!()
                },
                pressed: false,
                modifiers,
            });
        }
        WM_CHAR => {
            if let Some(ch) = char::from_u32(wparam as _) {
                if !ch.is_control() {
                    state.events.push(Event::Text(ch.into()));
                }
            }
        }
        WM_MOUSEWHEEL => {
            alter_modifiers(state, get_mouse_modifiers(wparam));

            let delta = (wparam >> 16) as i16 as f32 * 35.0 / WHEEL_DELTA as f32;

            if wparam & MK_CONTROL.0 as usize != 0 {
                state
                    .events
                    .push(Event::Zoom(if delta > 0.0 { 1.5 } else { 0.5 }));
            } else {
                state.events.push(Event::Scroll(Vec2::new(0.0, delta)));
            }
        }
        WM_MOUSEHWHEEL => {
            alter_modifiers(state, get_mouse_modifiers(wparam));

            let delta = (wparam >> 16) as i16 as f32 * 35.0 / WHEEL_DELTA as f32;

            if wparam & MK_CONTROL.0 as usize != 0 {
                state
                    .events
                    .push(Event::Zoom(if delta > 0. { 1.5 } else { 0.5 }));
            } else {
                state.events.push(Event::Scroll(Vec2::new(delta, 0.0)));
            }
        }
        msg @ (WM_KEYDOWN | WM_SYSKEYDOWN) => {
            let modifiers = get_key_modifiers(msg);
            state.modifiers = Some(modifiers);

            if let Some(key) = get_key(wparam) {
                if key == Key::V && modifiers.ctrl {
                    if let Some(clipboard) = get_clipboard_text() {
                        state.events.push(Event::Text(clipboard));
                    }
                }

                if key == Key::C && modifiers.ctrl {
                    state.events.push(Event::Copy);
                }

                if key == Key::X && modifiers.ctrl {
                    state.events.push(Event::Cut);
                }

                state.events.push(Event::Key {
                    pressed: true,
                    modifiers,
                    key,
                    repeat: lparam & (KF_REPEAT as isize) > 0,
                    physical_key: Some(key),
                });
            }
        }
        msg @ (WM_KEYUP | WM_SYSKEYUP) => {
            let modifiers = get_key_modifiers(msg);
            state.modifiers = Some(modifiers);

            if let Some(key) = get_key(wparam) {
                state.events.push(Event::Key {
                    pressed: false,
                    modifiers,
                    key,
                    repeat: lparam & (KF_REPEAT as isize) > 0,
                    physical_key: Some(key),
                });
            }
        }
        _ => {}
    }

    Ok((state.egui_ctx.wants_pointer_input()
        && matches!(
            umsg,
            WM_MOUSEMOVE
                | WM_LBUTTONDOWN
                | WM_LBUTTONDBLCLK
                | WM_LBUTTONUP
                | WM_RBUTTONDOWN
                | WM_RBUTTONDBLCLK
                | WM_RBUTTONUP
                | WM_MBUTTONDOWN
                | WM_MBUTTONDBLCLK
                | WM_MBUTTONUP
                | WM_MOUSEWHEEL
                | WM_MOUSEHWHEEL
        ))
        || (state.egui_ctx.wants_keyboard_input()
            && matches!(
                umsg,
                WM_CHAR | WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP
            )))
}

fn get_pos(lparam: isize) -> Pos2 {
    let state = unsafe { &mut STATE.as_mut().unwrap() };
    let zoom_factor = state.egui_ctx.zoom_factor();

    let x = (lparam & 0xFFFF) as i16 as f32;
    let y = (lparam >> 16 & 0xFFFF) as i16 as f32;

    if state.is_changing_scale {
        let prev_scale = state.prev_scale;
        Pos2::new(x / prev_scale, y / prev_scale)
    } else {
        Pos2::new(x / zoom_factor, y / zoom_factor)
    }
}

fn get_key_modifiers(msg: u32) -> Modifiers {
    let ctrl = unsafe { GetAsyncKeyState(VK_CONTROL.0 as _) != 0 };
    let shift = unsafe { GetAsyncKeyState(VK_LSHIFT.0 as _) != 0 };

    Modifiers {
        alt: msg == WM_SYSKEYDOWN,
        mac_cmd: false,
        command: ctrl,
        shift,
        ctrl,
    }
}

fn alter_modifiers(state: &mut EguiState, new: Modifiers) {
    if let Some(old) = state.modifiers.as_mut() {
        *old = new;
    }
}

/// https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes
fn get_key(wparam: usize) -> Option<Key> {
    match wparam {
        // number keys
        0x30..=0x39 => unsafe { Some(std::mem::transmute::<_, Key>(wparam as u8 - 0x10)) },
        // letter keys
        0x41..=0x5A => unsafe { Some(std::mem::transmute::<_, Key>(wparam as u8 - 0x17)) },
        // numpad keys
        0x60..=0x69 => unsafe { Some(std::mem::transmute::<_, Key>(wparam as u8 - 0x40)) },
        // f1-f20
        0x70..=0x83 => unsafe { Some(std::mem::transmute::<_, Key>(wparam as u8 - 0x2C)) },
        _ => match VIRTUAL_KEY(wparam as u16) {
            VK_DOWN => Some(Key::ArrowDown),
            VK_LEFT => Some(Key::ArrowLeft),
            VK_RIGHT => Some(Key::ArrowRight),
            VK_UP => Some(Key::ArrowUp),
            VK_ESCAPE => Some(Key::Escape),
            VK_TAB => Some(Key::Tab),
            VK_BACK => Some(Key::Backspace),
            VK_RETURN => Some(Key::Enter),
            VK_SPACE => Some(Key::Space),
            VK_INSERT => Some(Key::Insert),
            VK_DELETE => Some(Key::Delete),
            VK_HOME => Some(Key::Home),
            VK_END => Some(Key::End),
            VK_PRIOR => Some(Key::PageUp),
            VK_NEXT => Some(Key::PageDown),
            VK_SUBTRACT => Some(Key::Minus),
            _ => None,
        },
    }
}

fn get_mouse_modifiers(wparam: usize) -> Modifiers {
    Modifiers {
        alt: false,
        ctrl: (wparam & MK_CONTROL.0 as usize) != 0,
        shift: (wparam & MK_SHIFT.0 as usize) != 0,
        mac_cmd: false,
        command: (wparam & MK_CONTROL.0 as usize) != 0,
    }
}

fn get_clipboard_text() -> Option<String> {
    WindowsClipboardContext.get_contents().ok()
}

unsafe fn get_raw_input(state: &mut EguiState) -> Result<RawInput, Error> {
    Ok(RawInput {
        modifiers: state.modifiers.unwrap_or_default(),
        events: std::mem::take(&mut state.events),
        screen_rect: Some(get_screen_rect(state.egui_ctx.zoom_factor())?),
        time: Some(get_system_time()),
        max_texture_side: None,
        predicted_dt: 1.0 / 60.0,
        hovered_files: vec![],
        dropped_files: vec![],
        focused: true,
        ..Default::default()
    })
}

fn get_system_time() -> f64 {
    let mut time = 0;
    unsafe {
        let _ = NtQuerySystemTime(&mut time); // returns NTSTATUS
    }

    // nanoseconds
    (time as f64) / 10_000_000.0
}

pub fn get_screen_size() -> Result<(u32, u32), Error> {
    let state = unsafe {
        match &mut STATE {
            Some(s) => s,
            None => return Err(Error::NotInit),
        }
    };

    let mut rect = RECT::default();
    unsafe {
        if GetClientRect(WindowFromDC(state.window_handle), &mut rect).is_err() {
            return Err(Error::WindowSize);
        }
    }

    Ok((
        (rect.right - rect.left) as u32,
        (rect.bottom - rect.top) as u32,
    ))
}

fn get_screen_rect(zoom_factor: f32) -> Result<Rect, Error> {
    let size = get_screen_size()?;

    Ok(Rect {
        min: Pos2::ZERO,
        max: Pos2 {
            x: size.0 as f32 / zoom_factor,
            y: size.1 as f32 / zoom_factor,
        },
    })
}
