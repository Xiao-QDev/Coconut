use inkwell::OptimizationLevel;
use inkwell::context::Context;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use std::ffi::CString;
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use crate::Codegen::llvm_gen::CodeGenerator;
use crate::Filter::filter;
use crate::Lexer::lexer::Lexer;
use crate::Parser::AST::Program;
use crate::Parser::parser::Parser;

const VERSION: &str = "0.1.0";

fn read_source(filename: &str) -> Result<String, String> {
    if !filter::is_coconut_file(filename) {
        return Err(format!("'{}' is not a .cocl file", filename));
    }
    filter::read_coconut_file(filename).map_err(|e| e.to_string())
}

fn emit_object_file(codegen: &CodeGenerator, out_path: &Path) -> Result<(), String> {
    Target::initialize_native(&InitializationConfig::default())
        .map_err(|e| format!("Failed to initialize native target: {}", e))?;

    let triple = TargetMachine::get_default_triple();
    let target =
        Target::from_triple(&triple).map_err(|e| format!("Failed to get target: {}", e))?;

    let target_machine = target
        .create_target_machine(
            &triple,
            "generic",
            "",
            OptimizationLevel::Default,
            RelocMode::PIC,
            CodeModel::Default,
        )
        .ok_or("Failed to create target machine")?;

    let module = codegen.get_module();
    module.set_triple(&triple);
    module.set_data_layout(&target_machine.get_target_data().get_data_layout());

    target_machine
        .write_to_file(module, FileType::Object, out_path)
        .map_err(|e| format!("Failed to emit object file: {}", e))
}

unsafe extern "C" {
    #[link_name = "coconut_lld_elf_link"]
    fn lld_elf_link(args: *const *const c_char, argc: usize) -> bool;
}

fn get_link_config() -> Result<LinkConfig, String> {
    Target::initialize_native(&InitializationConfig::default())
        .map_err(|e| format!("Failed to initialize native target: {}", e))?;

    let triple = TargetMachine::get_default_triple();
    let triple_str = triple.as_str().to_string_lossy().to_string();
    let arch = triple_str.split('-').next().unwrap_or("x86_64").to_string();

    let (gcc_tuple, dynamic_linker) = match arch.as_str() {
        "x86_64" => (
            format!("{}-linux-gnu", arch),
            "/lib64/ld-linux-x86-64.so.2".to_string(),
        ),
        "aarch64" => (
            format!("{}-linux-gnu", arch),
            "/lib/ld-linux-aarch64.so.1".to_string(),
        ),
        "riscv64" | "riscv64gc" => (
            format!("{}-linux-gnu", arch),
            "/lib/ld-linux-riscv64-lp64d.so.1".to_string(),
        ),
        "arm" => (
            "arm-linux-gnueabihf".to_string(),
            "/lib/ld-linux-armhf.so.3".to_string(),
        ),
        _ => (
            format!("{}-linux-gnu", arch),
            format!("/lib/ld-linux-{}.so.2", arch),
        ),
    };

    Ok(LinkConfig {
        multiarch_dir: format!("/usr/lib/{}", gcc_tuple),
        lib_multiarch_dir: format!("/lib/{}", gcc_tuple),
        gcc_tuple,
        dynamic_linker,
    })
}

struct LinkConfig {
    multiarch_dir: String,
    lib_multiarch_dir: String,
    gcc_tuple: String,
    dynamic_linker: String,
}

fn find_crt(name: &str, cfg: &LinkConfig) -> String {
    let search_dirs = [
        cfg.multiarch_dir.as_str(),
        cfg.lib_multiarch_dir.as_str(),
        "/usr/lib64",
        "/usr/lib",
        "/lib64",
    ];
    for dir in &search_dirs {
        let full = format!("{}/{}", dir, name);
        if Path::new(&full).exists() {
            return full;
        }
    }
    let gcc_base = format!("/usr/lib/gcc/{}", cfg.gcc_tuple);
    if let Ok(entries) = std::fs::read_dir(&gcc_base) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let full = format!(
                    "{}/{}/{}",
                    gcc_base,
                    entry.file_name().to_string_lossy(),
                    name
                );
                if Path::new(&full).exists() {
                    return full;
                }
            }
        }
    }
    name.to_string()
}

fn link_executable(obj_path: &str, out_path: &str) -> Result<(), String> {
    let cfg = get_link_config()?;
    let crt1 = find_crt("crt1.o", &cfg);
    let crti = find_crt("crti.o", &cfg);
    let crtbegin = find_crt("crtbegin.o", &cfg);
    let crtend = find_crt("crtend.o", &cfg);
    let crtn = find_crt("crtn.o", &cfg);

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let runtime_lib = format!("{}/runtime/libstr_helpers.a", manifest_dir);

    let lib_flag_1 = format!("-L{}", cfg.multiarch_dir);
    let lib_flag_2 = format!("-L{}", cfg.lib_multiarch_dir);

    let raw_args = [
        "ld.lld",
        "-o",
        out_path,
        &lib_flag_1,
        &lib_flag_2,
        "-dynamic-linker",
        &cfg.dynamic_linker,
        &crt1,
        &crti,
        &crtbegin,
        obj_path,
        &runtime_lib,
        &crtend,
        &crtn,
        "-lc",
        "-lm",
    ];

    let c_args: Vec<CString> = raw_args.iter().map(|a| CString::new(*a).unwrap()).collect();
    let c_ptrs: Vec<*const c_char> = c_args.iter().map(|a| a.as_ptr()).collect();

    let success = unsafe { lld_elf_link(c_ptrs.as_ptr(), c_ptrs.len()) };

    if !success {
        return Err("LLD linking failed".to_string());
    }

    Ok(())
}

fn lex_and_parse(source: &str) -> Result<Program, String> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer
        .tokenize()
        .map_err(|e| format!("Lexical error: {}", e))?;
    let mut parser = Parser::new(tokens);
    parser.parse().map_err(|e| format!("Parse error: {}", e))
}

pub fn compile_source(filename: &str) -> Result<PathBuf, String> {
    let path = Path::new(filename);
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let obj_path = format!("{}.o", stem);
    let out_path = stem.to_string();

    let source = read_source(filename)?;
    println!("Loaded file: {}\n", filename);
    println!("Source:\n{}\n", source);

    let program = lex_and_parse(&source)?;
    println!("  Parser success!");
    println!("  Package: {}", program.package);
    println!("  Imports: {}", program.imports.len());
    println!("  Functions: {}\n", program.functions.len());

    let context = Context::create();
    let codegen = CodeGenerator::new(&context, "coconut_module");
    codegen.generate(&program)?;
    println!("\nCode generation successful!");

    emit_object_file(&codegen, Path::new(&obj_path))?;
    println!("  Emitted object file: {}", obj_path);

    link_executable(&obj_path, &out_path)?;
    println!("  Linked (LLD): {}", out_path);
    println!("\nBuild complete: {}", out_path);

    Ok(PathBuf::from(out_path))
}

pub fn compile_all() -> Result<PathBuf, String> {
    let src_dir = Path::new("src");
    if !src_dir.exists() {
        return Err("No src/ directory found in current working directory".to_string());
    }

    let main_file = src_dir.join("main.cocl");
    if !main_file.exists() {
        return Err("No src/main.cocl found".to_string());
    }

    let main_source =
        filter::read_coconut_file(main_file.to_str().unwrap()).map_err(|e| e.to_string())?;
    println!("Loaded: src/main.cocl");

    let mut main_program =
        lex_and_parse(&main_source).map_err(|e| format!("src/main.cocl: {}", e))?;
    println!("  Package: {}", main_program.package);
    println!("  Imports: {}", main_program.imports.len());
    println!("  Functions: {}", main_program.functions.len());

    for import in &main_program.imports {
        let import_path = &import.path;
        let dir_path = src_dir.join(import_path);

        let import_files: Vec<PathBuf> = if dir_path.is_dir() {
            let mut files: Vec<PathBuf> = std::fs::read_dir(&dir_path)
                .map_err(|e| format!("Cannot read directory {}: {}", dir_path.display(), e))?
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("cocl") {
                        Some(path)
                    } else {
                        None
                    }
                })
                .collect();
            if files.is_empty() {
                return Err(format!("No .cocl files found in {}", dir_path.display()));
            }
            files.sort();
            files
        } else {
            let file = if import_path.ends_with(".cocl") {
                src_dir.join(import_path)
            } else {
                src_dir.join(format!("{}.cocl", import_path))
            };
            if !file.exists() {
                return Err(format!(
                    "Import '{}' not found: expected {}",
                    import.path,
                    file.display()
                ));
            }
            vec![file]
        };

        for import_file in &import_files {
            let source = filter::read_coconut_file(import_file.to_str().unwrap())
                .map_err(|e| e.to_string())?;
            println!("Loaded: {}", import_file.display());

            let lib_program =
                lex_and_parse(&source).map_err(|e| format!("{}: {}", import_file.display(), e))?;
            println!("  Functions: {}", lib_program.functions.len());

            for func in &lib_program.functions {
                if main_program.functions.iter().any(|f| f.name == func.name) {
                    return Err(format!(
                        "Duplicate function '{}' in {}",
                        func.name,
                        import_file.display()
                    ));
                }
            }
            main_program.functions.extend(lib_program.functions);
            main_program.structs.extend(lib_program.structs);
            main_program.global_vars.extend(lib_program.global_vars);
        }
    }
    let context = Context::create();
    let codegen = CodeGenerator::new(&context, "coconut_module");
    codegen.generate(&main_program)?;
    println!("Code generation successful!");

    let out_name = src_dir
        .parent()
        .and_then(|p| p.file_name())
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let out_name = if out_name.is_empty() {
        "main"
    } else {
        &out_name
    };
    let obj_path = format!("{}.o", out_name);
    let out_path = out_name.to_string();

    emit_object_file(&codegen, Path::new(&obj_path))?;
    println!("  Emitted object file: {}", obj_path);

    link_executable(&obj_path, &out_path)?;
    println!("  Linked (LLD): {}", out_path);
    println!("\nBuild complete: {}", out_path);

    Ok(PathBuf::from(out_path))
}

pub fn cmd_build(args: &[String]) -> ExitCode {
    if args.len() >= 3 && args[2] == "-all" {
        return match compile_all() {
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("✗ Build failed: {}", e);
                ExitCode::FAILURE
            }
        };
    }
    if args.len() < 3 {
        eprintln!("Usage: cococ build <file.cocl>");
        eprintln!("       cococ build -all   (builds src/main.cocl + imports)");
        return ExitCode::FAILURE;
    }
    match compile_source(&args[2]) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("✗ Build failed: {}", e);
            ExitCode::FAILURE
        }
    }
}

pub fn cmd_run(args: &[String]) -> ExitCode {
    if args.len() >= 3 && args[2] == "-all" {
        let exe_path = match compile_all() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("✗ Build failed: {}", e);
                return ExitCode::FAILURE;
            }
        };
        println!("\nRunning: ./{}\n", exe_path.display());
        let status = Command::new(format!("./{}", exe_path.display()))
            .status()
            .expect("Failed to execute binary");
        return ExitCode::from(status.code().unwrap_or(1) as u8);
    }
    if args.len() < 3 {
        eprintln!("Usage: cococ run <file.cocl>");
        eprintln!("       cococ run -all   (builds and runs src/main.cocl + imports)");
        return ExitCode::FAILURE;
    }
    let exe_path = match compile_source(&args[2]) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("✗ Build failed: {}", e);
            return ExitCode::FAILURE;
        }
    };
    println!("\nRunning: ./{}\n", exe_path.display());
    let status = Command::new(format!("./{}", exe_path.display()))
        .status()
        .expect("Failed to execute binary");
    ExitCode::from(status.code().unwrap_or(1) as u8)
}
pub fn cmd_ver() -> ExitCode {
    println!("Coconut Compiler v0.0.1");
    println!("LLVM backend:  inkwell 0.9.0 / LLVM 22.1.0");
    println!("Object emit:   LLVM TargetMachine");
    println!("Linker:        LLD (ld.lld)");
    ExitCode::SUCCESS
}

pub fn print_usage() {
    println!("Coconut Compiler v{}", VERSION);
    println!();
    println!("Usage:");
    println!("  cococ build <file.cocl>   Compile single file to executable");
    println!("  cococ build -all          Build src/main.cocl with all imports");
    println!("  cococ run   <file.cocl>   Compile and run single file");
    println!("  cococ run   -all          Build and run src/main.cocl + imports");
    println!("  cococ ver                 Show version information");
    println!();
    println!("Project layout for -all:");
    println!("  src/");
    println!("    main.cocl       <- entry point (package main, fn main)");
    println!("    utils.cocl      <- import \"utils\"");
    println!("    math.cocl       <- import \"math\"");
}
