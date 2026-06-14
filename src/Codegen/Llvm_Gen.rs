use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::BasicType;
use inkwell::values::BasicValueEnum;
use std::collections::HashMap;

use crate::Parser::AST::*;

struct LoopContext<'ctx> {
    break_block: inkwell::basic_block::BasicBlock<'ctx>,
    continue_block: inkwell::basic_block::BasicBlock<'ctx>,
}

pub struct CodeGenerator<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    named_values: HashMap<String, BasicValueEnum<'ctx>>,
    loop_stack: Vec<LoopContext<'ctx>>,
}

impl<'ctx> CodeGenerator<'ctx> {
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        CodeGenerator {
            context,
            module,
            builder,
            named_values: HashMap::new(),
            loop_stack: Vec::new(),
        }
    }

    pub fn generate(&mut self, program: &Program) -> Result<(), String> {
        for var_decl in &program.global_vars {
            self.generate_global_var(var_decl)?;
        }
        for func in &program.functions {
            self.generate_function(func)?;
        }
        println!("\nGenerated LLVM IR: ");
        println!("{}", self.module.print_to_string().to_string());
        Ok(())
    }

    fn generate_global_var(&mut self, var_decl: &VarDecl) -> Result<(), String> {
        match &var_decl.value {
            Expr::Literal(Literal::Int(value)) => {
                let init_value = self.context.i64_type().const_int(*value as u64, false);
                let global = self.module.add_global(
                    self.context.i64_type(),
                    Some(inkwell::AddressSpace::default()),
                    &var_decl.name,
                );
                global.set_initializer(&init_value);
                self.named_values
                    .insert(var_decl.name.clone(), global.as_pointer_value().into());
                Ok(())
            }
            Expr::Literal(Literal::String(s)) => {
                let str_bytes = s.as_bytes();
                let str_type = self
                    .context
                    .i8_type()
                    .array_type(str_bytes.len() as u32 + 1);
                let global_str = self.module.add_global(
                    str_type,
                    Some(inkwell::AddressSpace::default()),
                    &var_decl.name,
                );
                let mut bytes_with_null = str_bytes.to_vec();
                bytes_with_null.push(0); // 添加 null 终止符
                global_str.set_initializer(&self.context.const_string(&bytes_with_null, false));
                self.named_values
                    .insert(var_decl.name.clone(), global_str.as_pointer_value().into());
                Ok(())
            }
            _ => Err("Global variable must be initialized with constant".to_string()),
        }
    }

    fn generate_function(&mut self, func: &Function) -> Result<(), String> {
        let param_types: Vec<inkwell::types::BasicMetadataTypeEnum<'ctx>> = func
            .params
            .iter()
            .map(|p| self.type_to_llvm_metadata(&p.param_type))
            .collect::<Result<Vec<_>, _>>()?;

        let function = if func.return_type == Some(Type::Void) || func.return_type.is_none() {
            let param_types_with_ret: Vec<inkwell::types::BasicMetadataTypeEnum<'ctx>> = func
                .params
                .iter()
                .map(|p| self.type_to_llvm_metadata(&p.param_type))
                .collect::<Result<Vec<_>, _>>()?;
            let fn_type = self
                .context
                .i64_type()
                .fn_type(&param_types_with_ret, false);
            self.module.add_function(&func.name, fn_type, None)
        } else {
            let return_ty = self.type_to_llvm_basic(&func.return_type.clone().unwrap())?;
            let fn_type = return_ty.fn_type(&param_types, false);
            self.module.add_function(&func.name, fn_type, None)
        };
        let entry_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry_block);
        for (i, param) in function.get_params().iter().enumerate() {
            if i < func.params.len() {
                param.set_name(&func.params[i].name);
                let ptr = self
                    .builder
                    .build_alloca(self.context.i64_type(), &func.params[i].name)
                    .unwrap();
                self.builder.build_store(ptr, *param).unwrap();
                self.named_values
                    .insert(func.params[i].name.clone(), ptr.into());
            }
        }
        for stmt in &func.body.statements {
            if self.current_block_terminated() {
                break;
            }
            self.generate_statement(stmt)?;
        }
        if func.return_type == Some(Type::Void) || func.return_type.is_none() {
            if !self.current_block_terminated() {
                let zero = self.context.i64_type().const_int(0, false);
                self.builder.build_return(Some(&zero)).unwrap();
            }
        } else if !self.current_block_terminated() {
            let zero = self.context.i64_type().const_int(0, false);
            self.builder.build_return(Some(&zero)).unwrap();
        }
        Ok(())
    }

    fn current_block_terminated(&self) -> bool {
        if let Some(block) = self.builder.get_insert_block() {
            block.get_terminator().is_some()
        } else {
            true
        }
    }

    fn generate_statement(&mut self, stmt: &Statement) -> Result<(), String> {
        match stmt {
            Statement::Return(value) => {
                if let Some(expr) = value {
                    let llvm_value = self.generate_expression(expr)?;
                    self.builder.build_return(Some(&llvm_value)).unwrap();
                } else {
                    self.builder.build_return(None).unwrap();
                }
                Ok(())
            }
            Statement::Expression(expr) => {
                self.generate_expression(expr)?;
                Ok(())
            }
            Statement::VarDecl(var_decl) => {
                let init_value = self.generate_expression(&var_decl.value)?;
                let ptr = self
                    .builder
                    .build_alloca(init_value.get_type(), &var_decl.name)
                    .unwrap();
                self.builder.build_store(ptr, init_value).unwrap();
                self.named_values.insert(var_decl.name.clone(), ptr.into());
                Ok(())
            }
            Statement::ShortDecl(short_decl) => {
                let init_value = self.generate_expression(&short_decl.value)?;
                let ptr = self
                    .builder
                    .build_alloca(init_value.get_type(), &short_decl.name)
                    .unwrap();
                self.builder.build_store(ptr, init_value).unwrap();
                self.named_values
                    .insert(short_decl.name.clone(), ptr.into());
                Ok(())
            }
            Statement::Assign(assign) => {
                let value = self.generate_expression(&assign.value)?;
                let ptr_val = self
                    .named_values
                    .get(&assign.name)
                    .ok_or(format!("Undefined variable: {}", assign.name))?;
                let ptr = ptr_val.into_pointer_value();
                self.builder.build_store(ptr, value).unwrap();
                Ok(())
            }
            Statement::If(if_stmt) => self.generate_if_stmt(if_stmt),
            Statement::For(for_stmt) => self.generate_for_stmt(for_stmt),
            Statement::Break => {
                let loop_ctx = self.loop_stack.last().ok_or("break outside of loop")?;
                let break_block = loop_ctx.break_block;
                self.builder
                    .build_unconditional_branch(break_block)
                    .unwrap();
                Ok(())
            }
            Statement::Continue => {
                let loop_ctx = self.loop_stack.last().ok_or("continue outside of loop")?;
                let continue_block = loop_ctx.continue_block;
                self.builder
                    .build_unconditional_branch(continue_block)
                    .unwrap();
                Ok(())
            }
        }
    }

    fn generate_if_stmt(&mut self, if_stmt: &IfStmt) -> Result<(), String> {
        let function = self
            .builder
            .get_insert_block()
            .ok_or("No insert block")?
            .get_parent()
            .ok_or("No parent function")?;
        let then_block = self.context.append_basic_block(function, "then");
        let else_block = self.context.append_basic_block(function, "else");
        let merge_block = self.context.append_basic_block(function, "merge");
        let cond_value = self.generate_expression(&if_stmt.condition)?;
        self.builder
            .build_conditional_branch(cond_value.into_int_value(), then_block, else_block)
            .unwrap();
        self.builder.position_at_end(then_block);
        for stmt in &if_stmt.then_block.statements {
            self.generate_statement(stmt)?;
        }
        if !self.current_block_terminated() {
            self.builder
                .build_unconditional_branch(merge_block)
                .unwrap();
        }
        self.builder.position_at_end(else_block);
        if let Some(else_body) = &if_stmt.else_block {
            if else_body.statements.len() == 1 {
                if let Statement::If(nested_if) = &else_body.statements[0] {
                    self.generate_else_if_chain(nested_if, merge_block)?;
                } else {
                    for stmt in &else_body.statements {
                        self.generate_statement(stmt)?;
                    }
                    if !self.current_block_terminated() {
                        self.builder
                            .build_unconditional_branch(merge_block)
                            .unwrap();
                    }
                }
            } else {
                for stmt in &else_body.statements {
                    self.generate_statement(stmt)?;
                }
                if !self.current_block_terminated() {
                    self.builder
                        .build_unconditional_branch(merge_block)
                        .unwrap();
                }
            }
        } else {
            self.builder
                .build_unconditional_branch(merge_block)
                .unwrap();
        }
        self.builder.position_at_end(merge_block);
        if !self.current_block_terminated() {
            self.builder.build_unreachable().unwrap();
        }
        Ok(())
    }

    fn generate_else_if_chain(
        &mut self,
        if_stmt: &IfStmt,
        merge_block: inkwell::basic_block::BasicBlock<'ctx>,
    ) -> Result<(), String> {
        let function = self
            .builder
            .get_insert_block()
            .ok_or("No insert block")?
            .get_parent()
            .ok_or("No parent function")?;
        let then_block = self.context.append_basic_block(function, "then");
        let else_block = self.context.append_basic_block(function, "else");
        let cond_value = self.generate_expression(&if_stmt.condition)?;
        self.builder
            .build_conditional_branch(cond_value.into_int_value(), then_block, else_block)
            .unwrap();
        self.builder.position_at_end(then_block);
        for stmt in &if_stmt.then_block.statements {
            if let Statement::If(nested) = stmt {
                self.generate_else_if_chain(nested, merge_block)?;
            } else {
                self.generate_statement(stmt)?;
            }
        }
        if !self.current_block_terminated() {
            self.builder
                .build_unconditional_branch(merge_block)
                .unwrap();
        }
        self.builder.position_at_end(else_block);
        if let Some(else_body) = &if_stmt.else_block {
            if else_body.statements.len() == 1 {
                if let Statement::If(nested_if) = &else_body.statements[0] {
                    self.generate_else_if_chain(nested_if, merge_block)?;
                } else {
                    for stmt in &else_body.statements {
                        if let Statement::If(nested) = stmt {
                            self.generate_else_if_chain(nested, merge_block)?;
                        } else {
                            self.generate_statement(stmt)?;
                        }
                    }
                    if !self.current_block_terminated() {
                        self.builder
                            .build_unconditional_branch(merge_block)
                            .unwrap();
                    }
                }
            } else {
                for stmt in &else_body.statements {
                    if let Statement::If(nested) = stmt {
                        self.generate_else_if_chain(nested, merge_block)?;
                    } else {
                        self.generate_statement(stmt)?;
                    }
                }
                if !self.current_block_terminated() {
                    self.builder
                        .build_unconditional_branch(merge_block)
                        .unwrap();
                }
            }
        } else {
            self.builder
                .build_unconditional_branch(merge_block)
                .unwrap();
        }
        Ok(())
    }

    fn generate_for_stmt(&mut self, for_stmt: &ForStmt) -> Result<(), String> {
        let function = self
            .builder
            .get_insert_block()
            .ok_or("No insert block")?
            .get_parent()
            .ok_or("No parent function")?;
        if let Some(init) = &for_stmt.init {
            self.generate_statement(init)?;
        }
        let cond_block = self.context.append_basic_block(function, "for.cond");
        let body_block = self.context.append_basic_block(function, "for.body");
        let step_block = self.context.append_basic_block(function, "for.step");
        let merge_block = self.context.append_basic_block(function, "for.merge");
        self.builder.build_unconditional_branch(cond_block).unwrap();
        self.builder.position_at_end(cond_block);
        if let Some(condition) = &for_stmt.condition {
            let cond_value = self.generate_expression(condition)?;
            self.builder
                .build_conditional_branch(cond_value.into_int_value(), body_block, merge_block)
                .unwrap();
        } else {
            self.builder.build_unconditional_branch(body_block).unwrap();
        }
        self.loop_stack.push(LoopContext {
            break_block: merge_block,
            continue_block: step_block,
        });
        self.builder.position_at_end(body_block);
        for stmt in &for_stmt.body.statements {
            self.generate_statement(stmt)?;
            if self.current_block_terminated() {
                break;
            }
        }
        if !self.current_block_terminated() {
            self.builder.build_unconditional_branch(step_block).unwrap();
        }
        self.loop_stack.pop();
        self.builder.position_at_end(step_block);
        if let Some(step) = &for_stmt.step {
            self.generate_statement(step)?;
        }
        if !self.current_block_terminated() {
            self.builder.build_unconditional_branch(cond_block).unwrap();
        }
        self.builder.position_at_end(merge_block);
        Ok(())
    }

    fn generate_expression(&mut self, expr: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        match expr {
            Expr::Literal(literal) => match literal {
                Literal::Int(value) => Ok(self
                    .context
                    .i64_type()
                    .const_int(*value as u64, false)
                    .into()),
                Literal::String(_) => Err("String literals not yet implemented".to_string()),
            },
            Expr::Identifier(name) => {
                if let Some(value) = self.named_values.get(name) {
                    let ptr = value.into_pointer_value();
                    Ok(self
                        .builder
                        .build_load(self.context.i64_type(), ptr, name)
                        .unwrap())
                } else {
                    Err(format!("Undefined variable: {}", name))
                }
            }
            Expr::UnaryOp(op, expr) => {
                let value = self.generate_expression(expr)?;
                let value_int = value.into_int_value();
                match op {
                    crate::Parser::AST::UnaryOperator::Negate => {
                        let zero = self.context.i64_type().const_int(0, false);
                        Ok(self
                            .builder
                            .build_int_sub(zero, value_int, "neg")
                            .unwrap()
                            .into())
                    }
                    crate::Parser::AST::UnaryOperator::Positive => Ok(value),
                    crate::Parser::AST::UnaryOperator::Not => {
                        Ok(self.builder.build_not(value_int, "not").unwrap().into())
                    }
                }
            }
            Expr::BinaryOp(left, op, right) => {
                let lhs = self.generate_expression(left)?;
                let rhs = self.generate_expression(right)?;
                let lhs_int = lhs.into_int_value();
                let rhs_int = rhs.into_int_value();
                let result = match op {
                    Operator::Add => self
                        .builder
                        .build_int_add(lhs_int, rhs_int, "sum")
                        .unwrap()
                        .into(),
                    Operator::Subtract => self
                        .builder
                        .build_int_sub(lhs_int, rhs_int, "diff")
                        .unwrap()
                        .into(),
                    Operator::Multiply => self
                        .builder
                        .build_int_mul(lhs_int, rhs_int, "prod")
                        .unwrap()
                        .into(),
                    Operator::Divide => self
                        .builder
                        .build_int_signed_div(lhs_int, rhs_int, "quot")
                        .unwrap()
                        .into(),
                    Operator::Greater => self
                        .builder
                        .build_int_compare(inkwell::IntPredicate::SGT, lhs_int, rhs_int, "cmp")
                        .unwrap()
                        .into(),
                    Operator::Less => self
                        .builder
                        .build_int_compare(inkwell::IntPredicate::SLT, lhs_int, rhs_int, "cmp")
                        .unwrap()
                        .into(),
                    Operator::GreaterEqual => self
                        .builder
                        .build_int_compare(inkwell::IntPredicate::SGE, lhs_int, rhs_int, "cmp")
                        .unwrap()
                        .into(),
                    Operator::LessEqual => self
                        .builder
                        .build_int_compare(inkwell::IntPredicate::SLE, lhs_int, rhs_int, "cmp")
                        .unwrap()
                        .into(),
                    Operator::Equal => self
                        .builder
                        .build_int_compare(inkwell::IntPredicate::EQ, lhs_int, rhs_int, "cmp")
                        .unwrap()
                        .into(),
                    Operator::NotEqual => self
                        .builder
                        .build_int_compare(inkwell::IntPredicate::NE, lhs_int, rhs_int, "cmp")
                        .unwrap()
                        .into(),
                };
                Ok(result)
            }
            Expr::Call(name, args) => {
                let function = self
                    .module
                    .get_function(name)
                    .ok_or(format!("Undefined function: {}", name))?;
                let arg_values: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> = args
                    .iter()
                    .map(|arg| self.generate_expression(arg).map(|v| v.into()))
                    .collect::<Result<Vec<_>, _>>()?;
                let call_site = self
                    .builder
                    .build_call(function, &arg_values, &format!("call_{}", name))
                    .unwrap();
                if function.get_type().get_return_type().is_some() {
                    use inkwell::values::AnyValue;
                    Ok(call_site
                        .as_any_value_enum()
                        .try_into()
                        .unwrap_or_else(|_| self.context.i64_type().const_int(0, false).into()))
                } else {
                    Ok(self.context.i64_type().const_int(0, false).into())
                }
            }
            Expr::ModuleCall(module, func, args) => {
                if module != "pio" {
                    return Err(format!("Unknown module: {}", module));
                }
                match func.as_str() {
                    "Println" => {
                        for (i, arg) in args.iter().enumerate() {
                            match arg {
                                Expr::Literal(Literal::String(msg)) => {
                                    let format_str = if i == args.len() - 1 {
                                        format!("{}\n\0", msg)
                                    } else {
                                        format!("{}\0", msg)
                                    };
                                    let format_type =
                                        self.context.i8_type().array_type(format_str.len() as u32);
                                    let global_format = self.module.add_global(
                                        format_type,
                                        Some(inkwell::AddressSpace::default()),
                                        &format!("println_fmt_{}", i),
                                    );
                                    global_format.set_initializer(
                                        &self.context.const_string(format_str.as_bytes(), false),
                                    );

                                    let printf_fn = self
                                        .module
                                        .get_function("printf")
                                        .ok_or("printf not found")?;
                                    let format_ptr = self
                                        .builder
                                        .build_pointer_cast(
                                            global_format.as_pointer_value(),
                                            self.context.ptr_type(inkwell::AddressSpace::default()),
                                            "fmt_ptr",
                                        )
                                        .unwrap();

                                    self.builder
                                        .build_call(printf_fn, &[format_ptr.into()], "printf_call")
                                        .unwrap();
                                }
                                _ => {
                                    let value = self.generate_expression(arg)?;
                                    let format_str = if i == args.len() - 1 {
                                        "%d\n\0"
                                    } else {
                                        "%d \0"
                                    };
                                    let format_type =
                                        self.context.i8_type().array_type(format_str.len() as u32);
                                    let global_format = self.module.add_global(
                                        format_type,
                                        Some(inkwell::AddressSpace::default()),
                                        &format!("print_int_fmt_{}", i),
                                    );
                                    global_format.set_initializer(
                                        &self.context.const_string(format_str.as_bytes(), false),
                                    );

                                    let printf_fn = self
                                        .module
                                        .get_function("printf")
                                        .ok_or("printf not found")?;
                                    let format_ptr = self
                                        .builder
                                        .build_pointer_cast(
                                            global_format.as_pointer_value(),
                                            self.context.ptr_type(inkwell::AddressSpace::default()),
                                            "fmt_ptr",
                                        )
                                        .unwrap();

                                    self.builder
                                        .build_call(
                                            printf_fn,
                                            &[format_ptr.into(), value.into()],
                                            "printf_call",
                                        )
                                        .unwrap();
                                }
                            }
                        }
                        Ok(self.context.i64_type().const_int(0, false).into())
                    }
                    "Print" => {
                        for (i, arg) in args.iter().enumerate() {
                            match arg {
                                Expr::Literal(Literal::String(msg)) => {
                                    let format_str = format!("{}\0", msg);
                                    let format_type =
                                        self.context.i8_type().array_type(format_str.len() as u32);
                                    let global_format = self.module.add_global(
                                        format_type,
                                        Some(inkwell::AddressSpace::default()),
                                        &format!("print_fmt_{}", i),
                                    );
                                    global_format.set_initializer(
                                        &self.context.const_string(format_str.as_bytes(), false),
                                    );

                                    let printf_fn = self
                                        .module
                                        .get_function("printf")
                                        .ok_or("printf not found")?;
                                    let format_ptr = self
                                        .builder
                                        .build_pointer_cast(
                                            global_format.as_pointer_value(),
                                            self.context.ptr_type(inkwell::AddressSpace::default()),
                                            "fmt_ptr",
                                        )
                                        .unwrap();
                                    self.builder
                                        .build_call(printf_fn, &[format_ptr.into()], "printf_call")
                                        .unwrap();
                                }
                                _ => {
                                    let value = self.generate_expression(arg)?;
                                    let format_str = if i == args.len() - 1 {
                                        "%d\n\0"
                                    } else {
                                        "%d \0"
                                    };

                                    let format_type =
                                        self.context.i8_type().array_type(format_str.len() as u32);
                                    let global_format = self.module.add_global(
                                        format_type,
                                        Some(inkwell::AddressSpace::default()),
                                        &format!("print_int_fmt_{}", i),
                                    );
                                    global_format.set_initializer(
                                        &self.context.const_string(format_str.as_bytes(), false),
                                    );
                                    let printf_fn = self
                                        .module
                                        .get_function("printf")
                                        .ok_or("printf not found")?;
                                    let format_ptr = self
                                        .builder
                                        .build_pointer_cast(
                                            global_format.as_pointer_value(),
                                            self.context.ptr_type(inkwell::AddressSpace::default()),
                                            "fmt_ptr",
                                        )
                                        .unwrap();
                                    self.builder
                                        .build_call(
                                            printf_fn,
                                            &[format_ptr.into(), value.into()],
                                            "printf_call",
                                        )
                                        .unwrap();
                                }
                            }
                        }
                        Ok(self.context.i64_type().const_int(0, false).into())
                    }
                    "Printf" => {
                        if args.is_empty() {
                            return Err("Printf requires at least format string".to_string());
                        }
                        match &args[0] {
                            Expr::Literal(Literal::String(fmt)) => {
                                let processed_fmt = fmt
                                    .replace("\\n", "\n")
                                    .replace("\\t", "\t")
                                    .replace("\\r", "\r");
                                let format_str = format!("{}\0", processed_fmt);
                                let format_type =
                                    self.context.i8_type().array_type(format_str.len() as u32);
                                let global_format = self.module.add_global(
                                    format_type,
                                    Some(inkwell::AddressSpace::default()),
                                    "printf_fmt",
                                );
                                global_format.set_initializer(
                                    &self.context.const_string(format_str.as_bytes(), false),
                                );
                                let printf_fn = self
                                    .module
                                    .get_function("printf")
                                    .ok_or("printf not found")?;
                                let format_ptr = self
                                    .builder
                                    .build_pointer_cast(
                                        global_format.as_pointer_value(),
                                        self.context.ptr_type(inkwell::AddressSpace::default()),
                                        "fmt_ptr",
                                    )
                                    .unwrap();
                                let mut call_args = vec![format_ptr.into()];
                                for arg in &args[1..] {
                                    let value = self.generate_expression(arg)?;
                                    call_args.push(value.into());
                                }
                                self.builder
                                    .build_call(printf_fn, &call_args, "printf_call")
                                    .unwrap();
                            }
                            _ => {
                                return Err(
                                    "Printf first argument must be a format string".to_string()
                                );
                            }
                        }
                        Ok(self.context.i64_type().const_int(0, false).into())
                    }
                    "Sprintf" => Ok(self.context.i64_type().const_int(0, false).into()),
                    "Scan" => {
                        if args.len() != 1 {
                            return Err("Scan requires exactly 1 argument".to_string());
                        }
                        let format_str = "%d\0";
                        let format_type =
                            self.context.i8_type().array_type(format_str.len() as u32);
                        let global_format = self.module.add_global(
                            format_type,
                            Some(inkwell::AddressSpace::default()),
                            "scan_fmt",
                        );
                        global_format.set_initializer(
                            &self.context.const_string(format_str.as_bytes(), false),
                        );
                        let scanf_fn =
                            self.module.get_function("scanf").ok_or("scanf not found")?;
                        let format_ptr = self
                            .builder
                            .build_pointer_cast(
                                global_format.as_pointer_value(),
                                self.context.ptr_type(inkwell::AddressSpace::default()),
                                "fmt_ptr",
                            )
                            .unwrap();
                        if let Expr::Identifier(var_name) = &args[0] {
                            let ptr_val = self
                                .named_values
                                .get(var_name)
                                .ok_or(format!("Undefined variable: {}", var_name))?;
                            let ptr = ptr_val.into_pointer_value();
                            self.builder
                                .build_call(
                                    scanf_fn,
                                    &[format_ptr.into(), ptr.into()],
                                    "scanf_call",
                                )
                                .unwrap();
                        } else {
                            return Err("Scan argument must be a variable".to_string());
                        }
                        Ok(self.context.i64_type().const_int(0, false).into())
                    }
                    "Scanln" => {
                        if args.len() != 1 {
                            return Err("Scanln requires exactly 1 argument".to_string());
                        }
                        let format_str = "%d\n\0";
                        let format_type =
                            self.context.i8_type().array_type(format_str.len() as u32);
                        let global_format = self.module.add_global(
                            format_type,
                            Some(inkwell::AddressSpace::default()),
                            "scanln_fmt",
                        );
                        global_format.set_initializer(
                            &self.context.const_string(format_str.as_bytes(), false),
                        );

                        let scanf_fn =
                            self.module.get_function("scanf").ok_or("scanf not found")?;
                        let format_ptr = self
                            .builder
                            .build_pointer_cast(
                                global_format.as_pointer_value(),
                                self.context.ptr_type(inkwell::AddressSpace::default()),
                                "fmt_ptr",
                            )
                            .unwrap();
                        if let Expr::Identifier(var_name) = &args[0] {
                            let ptr_val = self
                                .named_values
                                .get(var_name)
                                .ok_or(format!("Undefined variable: {}", var_name))?;
                            let ptr = ptr_val.into_pointer_value();
                            self.builder
                                .build_call(
                                    scanf_fn,
                                    &[format_ptr.into(), ptr.into()],
                                    "scanf_call",
                                )
                                .unwrap();
                        } else {
                            return Err("Scanln argument must be a variable".to_string());
                        }
                        Ok(self.context.i64_type().const_int(0, false).into())
                    }
                    _ => Err(format!("Unknown function: pio.{}", func)),
                }
            }
        }
    }

    fn type_to_llvm_basic(&self, ty: &Type) -> Result<inkwell::types::BasicTypeEnum<'ctx>, String> {
        match ty {
            Type::Int => Ok(self.context.i64_type().into()),
            Type::String => Err("String type not yet implemented".to_string()),
            Type::Void => Err("Void is not a basic type".to_string()),
        }
    }

    fn type_to_llvm_metadata(
        &self,
        ty: &Type,
    ) -> Result<inkwell::types::BasicMetadataTypeEnum<'ctx>, String> {
        match ty {
            Type::Int => Ok(self.context.i64_type().into()),
            Type::String => Err("String type not yet implemented".to_string()),
            Type::Void => Err("Void parameter not allowed".to_string()),
        }
    }

    pub fn verify_and_dump(&self) -> Result<(), String> {
        self.module
            .verify()
            .map_err(|e| format!("Module verification failed: {:?}", e))?;
        Ok(())
    }

    pub fn get_module(&self) -> &Module<'ctx> {
        &self.module
    }
}
