use inkwell::OptimizationLevel;
use inkwell::context::Context;
use std::env;
use std::process::ExitCode;

use cococ::command::commands;
use cococ::{codegen, filter, lexer, parser};
fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        commands::print_usage();
        return ExitCode::SUCCESS;
    }

    match args[1].as_str() {
        "build" => commands::cmd_build(&args),
        "run" => commands::cmd_run(&args),
        "ver" => commands::cmd_ver(),
        _ => {
            let filename = &args[1];
            if !filter::filter::is_coconut_file(filename) {
                eprintln!("✗ Error: '{}' is not a .cocl file", filename);
                return ExitCode::FAILURE;
            }
            jit_run(filename)
        }
    }
}

fn jit_run(filename: &str) -> ExitCode {
    let source = match filter::filter::read_coconut_file(filename) {
        Ok(content) => {
            println!("Loaded file: {}\n", filename);
            content
        }
        Err(e) => {
            eprintln!("✗ {}", e);
            return ExitCode::FAILURE;
        }
    };
    println!("Source:\n{}\n", source);

    let mut lexer = lexer::lexer::Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(tokens) => {
            println!("lexer: {} tokens\n", tokens.len());
            tokens
        }
        Err(e) => {
            eprintln!("Lexical error: {}", e);
            return ExitCode::FAILURE;
        }
    };

    let mut parser = parser::parser::Parser::new(tokens);
    let program = match parser.parse() {
        Ok(program) => {
            println!("  parser success!");
            println!("  Package: {}", program.package);
            println!("  Imports: {}", program.imports.len());
            println!("  Functions: {}\n", program.functions.len());
            program
        }
        Err(e) => {
            eprintln!("{}: {}", filename, e);
            return ExitCode::FAILURE;
        }
    };

    let context = Context::create();
    let mut codegen = codegen::llvm_gen::CodeGenerator::new(&context, "coconut_module");
    match codegen.generate(&program) {
        Ok(_) => {
            println!("\nCode generation successful!");
            println!("\nJIT Execution: \n");
            unsafe {
                inkwell::support::load_visible_symbols();
                let execution_engine = match codegen
                    .get_module()
                    .create_jit_execution_engine(OptimizationLevel::None)
                {
                    Ok(ee) => ee,
                    Err(e) => {
                        eprintln!("JIT creation error: {:?}", e);
                        return ExitCode::FAILURE;
                    }
                };
                if let Ok(main_fn) =
                    execution_engine.get_function::<unsafe extern "C" fn() -> i64>("main")
                {
                    let result = main_fn.call();
                    ExitCode::from(result as u8)
                } else {
                    println!("  No main() function found");
                    ExitCode::FAILURE
                }
            }
        }
        Err(e) => {
            eprintln!("Code generation error: {}", e);
            ExitCode::FAILURE
        }
    }
}
