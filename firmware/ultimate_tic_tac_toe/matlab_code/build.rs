use std::env;
use std::path::PathBuf;

fn main() {
    // We can assume that the user has installed the esp toolchain using 'espup'.

    if let Ok(target) = env::var("TARGET") {
        if target == "xtensa-esp32s3-none-elf" {
            let xtensa_esp_elf: String;
            let esp_clang: String;

            #[cfg(target_os = "linux")]
            {
                let mut home = env::var("HOME").expect("Missing HOME env var");
                if home.ends_with('/') {
                    home.pop();
                }

                xtensa_esp_elf = format!(
                    "{home}/.rustup/toolchains/esp/xtensa-esp-elf/esp-14.2.0_20240906/xtensa-esp-elf"
                );
                esp_clang = format!(
                    "{home}/.rustup/toolchains/esp/xtensa-esp32-elf-clang/esp-19.1.2_20250225/esp-clang"
                );
            }

            #[cfg(target_os = "windows")]
            {
                let mut home = env::var("USERPROFILE").expect("Missing USERPROFILE env var");
                if home.ends_with('\\') {
                    home.pop();
                }

                xtensa_esp_elf = format!("{home}\\.rustup\\toolchains\\esp\\xtensa-esp-elf");
                esp_clang =
                    format!("{home}\\.rustup\\toolchains\\esp\\xtensa-esp32-elf-clang\\esp-clang");
            }

            unsafe {
                env::set_var(
                    "BINDGEN_EXTRA_CLANG_ARGS",
                    format!("-I{xtensa_esp_elf}/xtensa-esp-elf/include"),
                );
                env::set_var("LIBCLANG_PATH", format!("{esp_clang}/lib"));

                env::set_var(
                    "CC_xtensa_esp32s3_none_elf",
                    format!("{xtensa_esp_elf}/bin/xtensa-esp32s3-elf-gcc"),
                );
                env::set_var(
                    "AR_xtensa_esp32s3_none_elf",
                    format!("{xtensa_esp_elf}/bin/xtensa-esp32s3-elf-ar"),
                );
            }
        }
    }

    // Path to MATLAB generated code
    let matlab_code_path = "../../../matlab/codegen/lib/ultimate_tic_tac_toe_logic";
    let include_path = "../../../matlab/include";

    // Tell cargo to invalidate the built crate whenever the MATLAB files change
    println!("cargo:rerun-if-changed={}", matlab_code_path);
    println!("cargo:rerun-if-changed={}", include_path);

    // Compile the MATLAB generated C code
    cc::Build::new()
        .files(&[
            format!("{}/ultimate_tic_tac_toe_logic.c", matlab_code_path),
            format!("{}/ultimate_tic_tac_toe_logic_initialize.c", matlab_code_path),
            format!("{}/ultimate_tic_tac_toe_logic_terminate.c", matlab_code_path),
            format!("{}/all.c", matlab_code_path),
            format!("{}/diag.c", matlab_code_path),
            format!("{}/flipud.c", matlab_code_path),
        ])
        .include(matlab_code_path)
        .include(include_path)
    .flag_if_supported("-mlongcalls")
    .compile("ultimate_tic_tac_toe_logic");

    // Generate Rust bindings for the C code
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate bindings for
    .header(format!("{}/ultimate_tic_tac_toe_logic.h", matlab_code_path))
    .header(format!("{}/ultimate_tic_tac_toe_logic_initialize.h", matlab_code_path))
    .header(format!("{}/ultimate_tic_tac_toe_logic_terminate.h", matlab_code_path))
        // Add the MATLAB code path and include path to the include path
        .clang_arg(format!("-I{}", matlab_code_path))
        .clang_arg(format!("-I{}", include_path))
        // Tell bindgen to generate Rust bindings for these types
    .allowlist_type("struct0_T")
    .allowlist_type("struct1_T")
        // Function that we want to generate bindings for
    .allowlist_function("ultimate_tic_tac_toe_logic")
    .allowlist_function("ultimate_tic_tac_toe_logic_initialize")
    .allowlist_function("ultimate_tic_tac_toe_logic_terminate")
        // General options
        .use_core()
        // Generate bindings
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // Link with the C library we compiled
    println!("cargo:rustc-link-lib=ultimate_tic_tac_toe_logic");
}
