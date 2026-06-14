use std::env;
use std::process;

fn cococ_help() {
    println!("Coconut_Compiler is a tool for managing Coconut source code.");
    println!();
    println!("Usage: ");
    println!("    cococ <command> [arguments]");
    println!();
    println!("The commands are:");
    println!(
        "    run/cococ run                   <file.coconut> Compile and run a Coconut source file"
    );
    println!("    help/cococ help                 Show this help message");
    println!("    version/cococ --version         Show compiler version");
    println!();
}

fn cococ_version() {
    let os = match env::consts::OS {
        "linux" => "Linux",
        "macos" => "MacOS",
        "windows" => "Windows",
        other => other,
    };

    let arch = match env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        "x86" => "386",
        "arm" => "arm32",
        other => other,
    };
    println!("Coconut_Compiler version Coconut0.0.1 ({}/{})", os, arch);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() >= 2 {
        let full_command = args[1..].join(" ");
        match full_command.as_str() {
            cmd if cmd.starts_with("run ") => {
                let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
                if parts.len() < 2 || parts[1].is_empty() {
                    eprintln!("Error: No input file specified");
                    eprintln!("Usage: cococ run <file.coco>");
                    process::exit(1);
                }
                compile_and_run(parts[1]);
            }
            "help" => {
                cococ_help();
            }
            "--version" | "-ver" | "version" => {
                cococ_version();
            }
            "exit" | "quit" => {
                println!("呃啊~!");
                process::exit(0);
            }
            _ => {
                eprintln!("Error: Unknown command '{}'", full_command);
                eprintln!("Run 'cococ help' for available commands");
                process::exit(1);
            }
        }
        return;
    }
    println!("Coconut Compiler Terminal");
    println!("Type 'help' for available commands, 'exit' to quit\n");

    loop {
        print!("Command> ");
        use std::io::{self, Write};
        io::stdout().flush().unwrap();
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let input = input.trim();
                if input.is_empty() {
                    continue;
                }
                let command_str = if input.starts_with("cococ ") {
                    &input[6..]
                } else {
                    input
                };
                let parts: Vec<&str> = command_str.split_whitespace().collect();
                let command = parts[0];
                match command {
                    "run" => {
                        if parts.len() < 2 {
                            eprintln!("Error: No input file specified");
                            eprintln!("Usage: run <file.coconut>");
                        } else {
                            compile_and_run(parts[1]);
                        }
                    }
                    "help" => {
                        cococ_help();
                    }
                    "version" | "--version" | "-ver" => {
                        cococ_version();
                    }
                    "exit" | "quit" => {
                        println!("屑~!");
                        break;
                    }
                    _ => {
                        eprintln!("Error: Unknown command '{}'", command);
                        eprintln!("Type 'help' for available commands");
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                break;
            }
        }
    }
}

fn compile_and_run(filename: &str) {
    use Coconut_Compiler::Codegen::Llvm_Gen::CodeGenerator;
    use Coconut_Compiler::Lexer::Lexer::Lexer;
    use Coconut_Compiler::Parser::Parser::Parser;
    use Coconut_Compiler::StdLib::pio;
    use inkwell::OptimizationLevel;
    use inkwell::context::Context;
    if !filename.ends_with(".coconut") {
        eprintln!("Error: '{}' is not a .coconut file", filename);
        process::exit(1);
    }
    let source = match std::fs::read_to_string(filename) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", filename, e);
            process::exit(1);
        }
    };
    println!("Source:\n{}\n", source);
    let mut lexer = Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(tokens) => {
            println!("Lexer: {} tokens\n", tokens.len());
            tokens
        }
        Err(e) => {
            eprintln!("Lexical error: {}", e);
            process::exit(1);
        }
    };
    let mut parser = Parser::new(tokens);
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
            process::exit(1);
        }
    };
    let context = Context::create();
    let module = context.create_module("coconut_module");
    pio::init_pio_library(&context, &module);
    let mut codegen = CodeGenerator::new(&context, "coconut_module");
    match codegen.generate(&program) {
        Ok(_) => {
            println!("\nCode generation successful!");
            println!("\nJIT Execution:");
            println!();
            unsafe {
                inkwell::support::load_library_permanently(std::path::Path::new(""));
                let execution_engine = match codegen
                    .get_module()
                    .create_jit_execution_engine(OptimizationLevel::None)
                {
                    Ok(ee) => ee,
                    Err(e) => {
                        eprintln!("JIT creation error: {:?}", e);
                        process::exit(1);
                    }
                };
                if let Ok(main_fn) =
                    execution_engine.get_function::<unsafe extern "C" fn() -> i64>("main")
                {
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
            process::exit(1);
        }
    }
}
