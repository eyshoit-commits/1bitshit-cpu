use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = env::var("OUT_DIR").unwrap();
    let llama_path = Path::new(&out_dir).join("llama.cpp");

    // ═══════════════════════════════════════════════════════════════
    // PHASE 1: SOVEREIGN CLONE
    // ═══════════════════════════════════════════════════════════════
    if !llama_path.exists() {
        println!("cargo:warning=🔩 Cloning official ggml-org/llama.cpp source...");
        let status = Command::new("git")
            .args([
                "clone",
                "--depth",
                "1",
                "https://github.com/ggml-org/llama.cpp",
                llama_path.to_str().unwrap(),
            ])
            .status()
            .expect("Failed to clone llama.cpp");
        if !status.success() {
            panic!("Clone failed");
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // PHASE 2: INDUSTRIAL CMAKE BUILD
    // ═══════════════════════════════════════════════════════════════
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    let mut config = cmake::Config::new(&llama_path);

    config
        .define("LLAMA_BUILD_EXAMPLES", "OFF")
        .define("LLAMA_BUILD_TESTS", "OFF")
        .define("LLAMA_BUILD_SERVER", "OFF")
        // FIX 2: Disable llama-bench and all standalone tools.
        // llama-bench pulls in cpp-httplib which links OpenSSL.
        // On macOS x64, OpenSSL is not in the default linker path → LNK error.
        // We only need the static libraries (llama.a, ggml.a, etc.), not any binaries.
        .define("LLAMA_BUILD_TOOLS", "OFF")
        .define("LLAMA_BUILD_LLAMA_EXE", "OFF")
        .define("LLAMA_BUILD_BENCH", "OFF")
        .define("LLAMA_BUILD_IMATRIX", "OFF")
        .define("LLAMA_BUILD_PERF", "OFF")
        .define("LLAMA_BUILD_QUANT", "OFF")
        .define("LLAMA_STATIC", "ON")
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("CMAKE_MSVC_RUNTIME_LIBRARY", "MultiThreaded")
        .profile("Release");

    // ── Apple Platform Alignment ──────────────────────────────────────
    if target_os == "ios" {
        // Apple arch name is "arm64" NOT "aarch64".
        // CARGO_CFG_TARGET_ARCH returns "aarch64" (Rust/Linux naming),
        // but Apple's clang -arch flag only accepts "arm64".
        let apple_arch = if target_arch == "aarch64" {
            "arm64"
        } else {
            &target_arch
        };
        config.define("CMAKE_SYSTEM_NAME", "iOS");
        config.define("CMAKE_OSX_SYSROOT", "iphoneos");
        config.define("CMAKE_OSX_ARCHITECTURES", apple_arch);
        config.define("CMAKE_OSX_DEPLOYMENT_TARGET", "16.0");
    } else if target_os == "macos" {
        // Sync deployment target with Rust's default macOS linker flag.
        config.define("CMAKE_OSX_DEPLOYMENT_TARGET", "11.0");
    }

    // ── GPU Driver Logic (Sovereign Dispatch) ─────────────────────────
    let feature_cuda = env::var("CARGO_FEATURE_CUDA").is_ok();
    let feature_metal = env::var("CARGO_FEATURE_METAL").is_ok();
    let feature_vulkan = env::var("CARGO_FEATURE_VULKAN").is_ok();
    let feature_rocm = env::var("CARGO_FEATURE_ROCM").is_ok();
    let feature_openvino = env::var("CARGO_FEATURE_OPENVINO").is_ok();
    let feature_sycl = env::var("CARGO_FEATURE_SYCL").is_ok();
    let feature_qnn = env::var("CARGO_FEATURE_QNN").is_ok();
    let feature_cann = env::var("CARGO_FEATURE_CANN").is_ok();

    if feature_cuda {
        config.define("GGML_CUDA", "ON");
    } else if feature_metal || target_os == "macos" || target_os == "ios" {
        config.define("GGML_METAL", "ON");
    } else if feature_vulkan {
        config.define("GGML_VULKAN", "ON");
    } else if feature_rocm {
        config.define("GGML_HIPBLAS", "ON");
    } else if feature_openvino {
        config.define("GGML_OPENVINO", "ON");
        if let Ok(v) = env::var("OpenCL_INCLUDE_DIR") {
            config.define("OpenCL_INCLUDE_DIR", &v);
        }
        if let Ok(v) = env::var("OpenCL_LIBRARY") {
            config.define("OpenCL_LIBRARY", &v);
            if let Some(parent) = Path::new(&v).parent() {
                println!("cargo:rustc-link-search=native={}", parent.display());
            }
        }
        if let Ok(v) = env::var("OpenVINO_DIR") {
            config.define("OpenVINO_DIR", &v);
        }
    } else if feature_sycl {
        config.define("GGML_SYCL", "ON");
        if let Ok(v) = env::var("DPCPP_CXX") {
            config.define("CMAKE_CXX_COMPILER", &v);
        }
        if let Ok(v) = env::var("DPCPP_CC") {
            config.define("CMAKE_C_COMPILER", &v);
        }
        config.cxxflag("/EHsc");
    } else if feature_qnn {
        config.define("GGML_QNN", "ON");
    } else if feature_cann {
        config.define("GGML_CANN", "ON");
        config.define("SOC_TYPE", "ascend910b");
    }

    let dst = config.build();

    // ═══════════════════════════════════════════════════════════════
    // PHASE 3: INDUSTRIAL LINKAGE
    // ═══════════════════════════════════════════════════════════════
    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-search=native={}/lib64", dst.display());
    find_and_link_search_paths(&dst);

    // ── Core libs (always present) ────────────────────────────────────
    println!("cargo:rustc-link-lib=static=llama");
    println!("cargo:rustc-link-lib=static=ggml");
    println!("cargo:rustc-link-lib=static=ggml-base");
    println!("cargo:rustc-link-lib=static=ggml-cpu");

    // ── Windows: explicit backend static libs ─────────────────────────
    // On Windows DLL builds, ALL symbols must resolve at link time.
    // ggml.lib's backend-registry calls backend-specific functions which
    // live in their own libs. Without explicit linking → LNK2019.
    // On Linux/macOS, the dynamic linker resolves these at runtime.
    if target_os == "windows" {
        if feature_cuda {
            println!("cargo:rustc-link-lib=static=ggml-cuda");
        }
        if feature_vulkan {
            println!("cargo:rustc-link-lib=static=ggml-vulkan");
            if let Ok(vulkan_sdk) = env::var("VULKAN_SDK") {
                println!("cargo:rustc-link-search=native={}/Lib", vulkan_sdk);
            }
            println!("cargo:rustc-link-lib=dylib=vulkan-1");
        }
        if feature_rocm {
            println!("cargo:rustc-link-lib=static=ggml-hip");
        }
        if feature_openvino {
            println!("cargo:rustc-link-lib=static=ggml-openvino");
            println!("cargo:rustc-link-lib=dylib=OpenCL");
        }
    }

    if target_os == "macos" || target_os == "ios" {
        println!("cargo:rustc-link-lib=static=ggml-metal");
        println!("cargo:rustc-link-lib=static=ggml-blas");
        println!("cargo:rustc-link-lib=framework=Accelerate");
    }

    // ── OS-specific system libraries ──────────────────────────────────
    if target_os == "windows" {
        println!("cargo:rustc-link-lib=dylib=advapi32");
        println!("cargo:rustc-link-lib=dylib=user32");
        println!("cargo:rustc-link-lib=dylib=ws2_32");

        // FIX 1: CUDA Runtime + cuBLAS import libraries.
        //
        // These are NOT statically embedded inside ggml-cuda.lib.
        // ggml-cuda.lib calls cudaLaunchCooperativeKernel, cudaFuncSetAttribute,
        // __cudaRegisterFatBinary, cublasCreate_v2 etc. which live in:
        //   cudart.lib  → cudart64_*.dll  (CUDA Runtime API)
        //   cublas.lib  → cublas64_*.dll  (cuBLAS)
        //   cublasLt.lib→ cublasLt64_*.dll
        //   cuda.lib    → nvcuda.dll      (CUDA Driver API: cuDeviceGet, cuMemCreate...)
        //
        // jimver/cuda-toolkit GHA action sets CUDA_PATH to the toolkit root.
        // e.g. "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.8"
        // Import libs are at $CUDA_PATH\lib\x64\*.lib
        if feature_cuda {
            if let Ok(cuda_path) = env::var("CUDA_PATH") {
                println!("cargo:rustc-link-search=native={}/lib/x64", cuda_path);
            }
            println!("cargo:rustc-link-lib=dylib=cudart");
            println!("cargo:rustc-link-lib=dylib=cublas");
            println!("cargo:rustc-link-lib=dylib=cublasLt");
            println!("cargo:rustc-link-lib=dylib=cuda");
        }
    } else if target_os == "macos" {
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=Metal");
        println!("cargo:rustc-link-lib=framework=MetalKit");
        println!("cargo:rustc-link-lib=framework=Accelerate");
        println!("cargo:rustc-link-lib=dylib=c++");
    } else if target_os == "ios" {
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=Metal");
        println!("cargo:rustc-link-lib=framework=UIKit");
        println!("cargo:rustc-link-lib=framework=Accelerate");
        println!("cargo:rustc-link-lib=dylib=c++");
    } else {
        // Linux / Android
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }

    println!("cargo:warning=🧿 [Llama-Engine] Industrial CMake Build Complete.");
}

fn find_and_link_search_paths(dir: &Path) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut has_lib = false;
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                find_and_link_search_paths(&path);
            } else if let Some(ext) = path.extension() {
                if ext == "a" || ext == "lib" {
                    has_lib = true;
                }
            }
        }
        if has_lib {
            println!("cargo:rustc-link-search=native={}", dir.display());
        }
    }
}
