
use inkwell::context::Context;
use inkwell::OptimizationLevel;
use std::env;

use Coconut_Compiler::{Codegen, Filter, Lexer, Parser, StdLib};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file.coco>", args[0]);
        eprintln!("Example: {} test.coco", args[0]);
        return;
    }
    let filename = &args[1];
    if !Filter::Filter::is_coconut_file(filename) {
        eprintln!("✗ Error: '{}' is not a .coco file", filename);
        return;
    }
    let source = match Filter::Filter::read_coconut_file(filename) {
        Ok(content) => {
            println!("Loaded file: {}\n", filename);
            content
        }
        Err(e) => {
            eprintln!("✗ {}", e);
            return;
        }
    };
    println!("Source:\n{}\n", source);
    let mut lexer = Lexer::Lexer::Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(tokens) => {
            println!("Lexer: {} tokens\n", tokens.len());
            tokens
        }
        Err(e) => {
            eprintln!("Lexical error: {}", e);
            return;
        }
    };
    let mut parser = Parser::Parser::Parser::new(tokens);
    let program = match parser.parse() {
        Ok(program) => {
            println!("  Parser success!");
            println!("  Package: {}", program.package);
            println!("  Imports: {}", program.imports.len());
            println!("  Functions: {}\n", program.functions.len());
            program
        }
        Err(e) => {
            eprintln!("  Parse error: {}", e);
            return;
        }
    };
    let context = Context::create();
    let mut codegen = Codegen::Llvm_Gen::CodeGenerator::new(&context, "coconut_module");
    for import in &program.imports {
        if import.path == "pio" {
            StdLib::pio::init_pio_library(&context, codegen.get_module());
            println!("Imported standard library: pio");
        }
    }
    match codegen.generate(&program) {
        Ok(_) => {
            println!("\nCode generation successful!");
            println!("\nJIT Execution: ");
            println!();
            unsafe {
                inkwell::support::load_library_permanently(std::path::Path::new(""));

                let execution_engine = match codegen.get_module().create_jit_execution_engine(OptimizationLevel::None) {
                    Ok(ee) => ee,
                    Err(e) => {
                        eprintln!("JIT creation error: {:?}", e);
                        return;
                    }
                };
                if let Ok(main_fn) = execution_engine.get_function::<unsafe extern "C" fn() -> i64>("main") {
                    let result = main_fn.call();
                    println!("  main() returned: {}", result);
                } else {
                    println!("  No main() function found");
                }
            }
            println!("\nCompilation Complete!");
        }
        Err(e) => {
            eprintln!("Code generation error: {}", e);
        }
    }
}
