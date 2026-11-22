//! Saving and restoring the GL state that `egui_glow` touches.
//!
//! When the overlay draws into the game's own GL context (instead of a separate one),
//! everything egui changes would leak into the game's next frame. cocos2d also caches
//! a lot of this state (bound program, textures, enabled vertex attributes, blend
//! function), so a change behind its back makes it skip calls it thinks are redundant.
//! We snapshot the state before painting and put it back afterwards.

use egui_glow::glow::{self, HasContext};
use std::num::NonZeroU32;

/// `glGetVertexAttribiv`, which glow doesn't expose.
pub type GetVertexAttribIv = unsafe extern "system" fn(index: u32, pname: u32, params: *mut i32);

/// how many vertex attribute slots to save. cocos2d uses 0..=2, egui uses 0..=2; 8 is plenty.
const SAVED_VERTEX_ATTRIBS: usize = 8;

fn nz(v: i32) -> Option<NonZeroU32> {
    NonZeroU32::new(v as u32)
}

pub struct GlState {
    program: i32,
    array_buffer: i32,
    element_array_buffer: i32,
    vertex_array: Option<i32>,
    active_texture: i32,
    texture_2d: i32,
    blend: bool,
    blend_src_rgb: i32,
    blend_dst_rgb: i32,
    blend_src_alpha: i32,
    blend_dst_alpha: i32,
    blend_eq_rgb: i32,
    blend_eq_alpha: i32,
    scissor_test: bool,
    scissor_box: [i32; 4],
    cull_face: bool,
    depth_test: bool,
    viewport: [i32; 4],
    unpack_alignment: i32,
    attribs_enabled: [Option<bool>; SAVED_VERTEX_ATTRIBS],
}

/// whether `glBindVertexArray` is safe to call (same rule `egui_glow` uses)
pub fn supports_vao(gl: &glow::Context) -> bool {
    if gl.version().major >= 3 {
        return true;
    }
    let ext = gl.supported_extensions();
    ext.contains("ARB_vertex_array_object") || ext.contains("GL_ARB_vertex_array_object")
}

impl GlState {
    pub unsafe fn save(
        gl: &glow::Context,
        has_vao: bool,
        get_vertex_attrib_iv: Option<GetVertexAttribIv>,
    ) -> Self {
        let mut scissor_box = [0; 4];
        let mut viewport = [0; 4];
        gl.get_parameter_i32_slice(glow::SCISSOR_BOX, &mut scissor_box);
        gl.get_parameter_i32_slice(glow::VIEWPORT, &mut viewport);

        let active_texture = gl.get_parameter_i32(glow::ACTIVE_TEXTURE);
        // egui only ever uses texture unit 0
        gl.active_texture(glow::TEXTURE0);
        let texture_2d = gl.get_parameter_i32(glow::TEXTURE_BINDING_2D);

        let mut attribs_enabled = [None; SAVED_VERTEX_ATTRIBS];
        if let Some(get) = get_vertex_attrib_iv {
            for (i, slot) in attribs_enabled.iter_mut().enumerate() {
                let mut v = 0;
                get(i as u32, glow::VERTEX_ATTRIB_ARRAY_ENABLED, &mut v);
                *slot = Some(v != 0);
            }
        }

        let state = Self {
            program: gl.get_parameter_i32(glow::CURRENT_PROGRAM),
            array_buffer: gl.get_parameter_i32(glow::ARRAY_BUFFER_BINDING),
            element_array_buffer: gl.get_parameter_i32(glow::ELEMENT_ARRAY_BUFFER_BINDING),
            vertex_array: has_vao.then(|| gl.get_parameter_i32(glow::VERTEX_ARRAY_BINDING)),
            active_texture,
            texture_2d,
            blend: gl.is_enabled(glow::BLEND),
            blend_src_rgb: gl.get_parameter_i32(glow::BLEND_SRC_RGB),
            blend_dst_rgb: gl.get_parameter_i32(glow::BLEND_DST_RGB),
            blend_src_alpha: gl.get_parameter_i32(glow::BLEND_SRC_ALPHA),
            blend_dst_alpha: gl.get_parameter_i32(glow::BLEND_DST_ALPHA),
            blend_eq_rgb: gl.get_parameter_i32(glow::BLEND_EQUATION_RGB),
            blend_eq_alpha: gl.get_parameter_i32(glow::BLEND_EQUATION_ALPHA),
            scissor_test: gl.is_enabled(glow::SCISSOR_TEST),
            scissor_box,
            cull_face: gl.is_enabled(glow::CULL_FACE),
            depth_test: gl.is_enabled(glow::DEPTH_TEST),
            viewport,
            unpack_alignment: gl.get_parameter_i32(glow::UNPACK_ALIGNMENT),
            attribs_enabled,
        };

        drain_errors(gl);
        state
    }

    pub unsafe fn restore(&self, gl: &glow::Context) {
        let set = |cap: u32, on: bool| {
            if on {
                gl.enable(cap)
            } else {
                gl.disable(cap)
            }
        };

        // VAO first: it owns the element array binding and attribute state when bound
        if let Some(vao) = self.vertex_array {
            gl.bind_vertex_array(nz(vao).map(glow::NativeVertexArray));
        }
        gl.bind_buffer(
            glow::ELEMENT_ARRAY_BUFFER,
            nz(self.element_array_buffer).map(glow::NativeBuffer),
        );
        gl.bind_buffer(glow::ARRAY_BUFFER, nz(self.array_buffer).map(glow::NativeBuffer));
        for (i, enabled) in self.attribs_enabled.iter().enumerate() {
            match enabled {
                Some(true) => gl.enable_vertex_attrib_array(i as u32),
                Some(false) => gl.disable_vertex_attrib_array(i as u32),
                None => {}
            }
        }

        gl.use_program(nz(self.program).map(glow::NativeProgram));

        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, nz(self.texture_2d).map(glow::NativeTexture));
        gl.active_texture(self.active_texture as u32);

        set(glow::BLEND, self.blend);
        gl.blend_equation_separate(self.blend_eq_rgb as u32, self.blend_eq_alpha as u32);
        gl.blend_func_separate(
            self.blend_src_rgb as u32,
            self.blend_dst_rgb as u32,
            self.blend_src_alpha as u32,
            self.blend_dst_alpha as u32,
        );

        set(glow::SCISSOR_TEST, self.scissor_test);
        let [x, y, w, h] = self.scissor_box;
        gl.scissor(x, y, w, h);
        set(glow::CULL_FACE, self.cull_face);
        set(glow::DEPTH_TEST, self.depth_test);
        let [x, y, w, h] = self.viewport;
        gl.viewport(x, y, w, h);
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, self.unpack_alignment);

        drain_errors(gl);
    }
}

/// clear the GL error flag so the game doesn't see errors we caused
pub unsafe fn drain_errors(gl: &glow::Context) {
    for _ in 0..16 {
        if gl.get_error() == glow::NO_ERROR {
            break;
        }
    }
}
