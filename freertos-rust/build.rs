extern crate bindgen;
extern crate cc;

use std::env;
use std::path::PathBuf;

fn print_env() {
    let env_keys =["TARGET", "OUT_DIR", "HOST"];
    env::vars().for_each(|(key, val)| {
        if key.starts_with("CARGO") {
            println!("cargo:warning={}={}", key, val);
        } else if env_keys.contains(&key.as_str()) {
            println!("cargo:warning={}={}", key, val);
        } else {
            //println!("cargo:warning={}={}", key, val);
        }
    });
}

#[allow(unused_variables)]
// See: https://doc.rust-lang.org/cargo/reference/build-scripts.html
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let shim_c = PathBuf::from(manifest_dir.as_str()).join("src/freertos/shim.c");
    println!("cargo:warning=Setting FREERTOS_SHIM={}",shim_c.to_str().unwrap());
    //env::set_var("FREERTOS_SHIM", shim_c);

    println!("cargo:SHIM={}", PathBuf::from(manifest_dir).join("src/freertos").to_str().unwrap());
    //print_env();

    return;
    let build_freertos = false;
    println!("run build.rs");
    println!("cargo:warning=Printing some infos as warnings:");

    let mut build_shim = cc::Build::new();

    println!("cargo:rerun-if-changed=src/freertos/shim.c");
    build_shim.file("src/freertos/shim.c"); // TODO: make separate lib file for shim?
    build_shim.compile("freertos_shim");


    // ENV variables:
    // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
    let target = env::var("TARGET").unwrap();
    // x86_64-pc-windows-msvc, x86_64-pc-windows-gnu, thumbv7m-none-eabi

    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap(); // msvc, gnu, ...
    let target_family = env::var("CARGO_CFG_TARGET_FAMILY").unwrap_or_default(); // unix, windows
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap(); // x86_64
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap(); // none, windows, linux, macos

    let env_keys =["TARGET", "OUT_DIR", "HOST"];
    env::vars().for_each(|(key, val)| {
        if key.starts_with("CARGO") {
            println!("cargo:warning={}={}", key, val);
        } else if env_keys.contains(&key.as_str()) {
            println!("cargo:warning={}={}", key, val);
        } else {
            //println!("cargo:warning={}={}", key, val);
        }
    });

    let port = match (target.as_str(), target_arch.as_str(), target_os.as_str(), target_env.as_str()) {
        (_, "x86_64", "windows", _) => "MSVC-MingW",
        ("thumbv7m-none-eabi", _, _, _) => "GCC/ARM_CM3",
        // TODO We should support feature "trustzone"
        ("thumbv8m.main-none-eabi", _, _, _) => "GCC/ARM_CM33_NTZ/non_secure",
        ("thumbv8m.main-none-eabihf", _, _, _) => "GCC/ARM_CM33_NTZ/non_secure",
        _ => {
            println!("cargo:warning=Unknown arch: '{}'", target_arch);
            "MSVC-MingW"
        }
    };
    println!("cargo:warning=Using FreeRTOS port '{}'", port);
    println!("cargo:warning=---------------------------------");


    //println!("cargo:rerun-if-changed=always");
    println!("cargo:rerun-if-changed=build.rs");

    // TODO: link scripts will be in final crate
    build_linker_script("examples/stm32-cortex-m3/layout.ld");

    // Build FreeRTOS for Windows
    let freertos_src_path = PathBuf::from("FreeRTOS/FreeRTOS/Source/");
    let freertos_plus_src_path = PathBuf::from("FreeRTOS/FreeRTOS-Plus/Source/");
    let freertos_demo_path = PathBuf::from("FreeRTOS/FreeRTOS/Demo");


    // For Windows GNU compilation we need the winmm library
    if target.as_str() == "x86_64-pc-windows-gnu" {
        println!("cargo:rustc-link-lib=static=winmm");
    }
    let mut build = cc::Build::new();
    build
        .pic(false) // Needed for ARM target, windows build still works with it
        .define("projCOVERAGE_TEST", "0") // TODO: Still needed?
        //.static_flag(true)
        //.shared_flag(true)
        // Files related to port
        //.include("src/freertos/ports/win/")
        // TODO: This is the windows specific part that needs to be env specific
        // FreeRTOS.h and modules (Task, Queues, etc.)
        .include(freertos_src_path.join("include"))
        // portmacro.h
        .include(freertos_src_path.join("portable").join(port));
    // Tracing from Demo TODO: Get rid of this
    //.include(freertos_demo_path.join(demo).join("Trace_Recorder_Configuration"))
    //.include(freertos_plus_src_path.join("FreeRTOS-Plus-Trace/Include"))

    // TODO: find a better way to find the correct FreeRTOSConfig.h
    // FreeRTOSConfig.h
    if target.as_str() == "thumbv7m-none-eabi" {
        build.include("examples/stm32-cortex-m3");
    } else if target_os.as_str() == "win" {
        //.include(freertos_demo_path.join(demo))
        build.include("src/freertos/ports/win");
    } else {
        build.include("src/freertos/ports");
        //panic!("missing include dir for FreeRTOSconfig.h")
    }

    if target_os.as_str() == "windows" {
        println!("cargo:rerun-if-changed=src/freertos/ports/win/Run-time-stats-utils.c");
        println!("cargo:rerun-if-changed=src/freertos/ports/win/hooks.c");
        build.file("src/freertos/ports/win/Run-time-stats-utils.c");
        build.file("src/freertos/ports/win/hooks.c");
    } else {
        println!("cargo:rerun-if-changed=src/freertos/ports/arm/hooks.c");
        build.file("src/freertos/ports/arm/hooks.c");
    }

    println!("cargo:rerun-if-changed=src/freertos/shim.c");
    build.file("src/freertos/shim.c"); // TODO: make separate lib file for shim?

    // FreeRTOS Plus Trace is needed for windows Demo
    //.file(freertos_plus_src_path.join("FreeRTOS-Plus-Trace/trcKernelPort.c"))
    //.file(freertos_plus_src_path.join("FreeRTOS-Plus-Trace/trcSnapshotRecorder.c"))

    // FreeRTOS
    build.file(freertos_src_path.join("croutine.c"))
        .file(freertos_src_path.join("event_groups.c"))
        .file(freertos_src_path.join("portable/MemMang/heap_4.c"))
        .file(freertos_src_path.join("stream_buffer.c"))
        .file(freertos_src_path.join("timers.c"))
        .file(freertos_src_path.join("list.c"))
        .file(freertos_src_path.join("queue.c"))
        .file(freertos_src_path.join("tasks.c"))
        .file(freertos_src_path.join("portable").join(port).join("port.c"));

    if build_freertos {
        build.compile("freertos");
    }
}

fn build_linker_script(path: &str) {
    // Put the linker script somewhere the linker can find it
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    std::fs::copy(path, out.join("memory.x"))
        .expect("failed to copy linker script");
    println!("cargo:rustc-link-search={}", out.display());

    // Only re-run the build script when memory.x is changed,
    // instead of when any part of the source code changes.
    println!("cargo:rerun-if-changed={}", path);
}