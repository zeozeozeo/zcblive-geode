use std::path::PathBuf;

fn main() {
    let out_path =
        PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR enviroment variable not set"));

    let bindings = bindgen::Builder::default()
        .header("headers/fmod.h")
        .header("headers/fmod_codec.h")
        .header("headers/fmod_common.h")
        .header("headers/fmod_dsp.h")
        .header("headers/fmod_dsp_effects.h")
        .header("headers/fmod_errors.h")
        .header("headers/fmod_output.h")
        .prepend_enum_name(false)
        .derive_debug(true);

    bindings
        .generate()
        .expect("unable to generate bindings")
        .write_to_file(out_path.join("bindings.rs"))
        .expect("unable to write bindings");
}
