use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::{AnyValue, BasicValueEnum};
use std::cell::RefCell;
use std::collections::HashMap;

use crate::parser::ast::*;
use crate::stdlib::pio::Pio;

#[derive(Clone)]
struct LoopContext<'ctx> {
    break_block: inkwell::basic_block::BasicBlock<'ctx>,
    continue_block: inkwell::basic_block::BasicBlock<'ctx>,
}

pub struct CodeGenerator<'ctx> {
    pio: Pio<'ctx>,
    named_values: RefCell<HashMap<String, BasicValueEnum<'ctx>>>,
    var_types: RefCell<HashMap<String, Type>>,
    loop_stack: RefCell<Vec<LoopContext<'ctx>>>,
    struct_defs: RefCell<HashMap<String, StructDef>>,
}

impl<'ctx> CodeGenerator<'ctx> {
    fn llvm<T>(result: Result<T, inkwell::builder::BuilderError>) -> Result<T, String> {
        result.map_err(|e| format!("LLVM error: {}", e))
    }
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let pio = Pio::new(context, module_name);
        CodeGenerator {
            pio,
            named_values: RefCell::new(HashMap::new()),
            var_types: RefCell::new(HashMap::new()),
            loop_stack: RefCell::new(Vec::new()),
            struct_defs: RefCell::new(HashMap::new()),
        }
    }

    fn context(&self) -> &'ctx Context {
        self.pio.context()
    }

    fn module(&self) -> &Module<'ctx> {
        self.pio.module()
    }

    fn builder(&self) -> &Builder<'ctx> {
        self.pio.builder()
    }

    fn set_var_type(&self, name: &str, ty: Type) {
        self.var_types.borrow_mut().insert(name.to_string(), ty);
    }

    fn get_var_type(&self, name: &str) -> Type {
        self.var_types
            .borrow()
            .get(name)
            .cloned()
            .unwrap_or(Type::Int)
    }

    pub fn generate(&mut self, program: &Program) -> Result<(), String> {
        for struct_def in &program.structs {
            self.struct_defs
                .borrow_mut()
                .insert(struct_def.name.clone(), struct_def.clone());
        }
        for var_decl in &program.global_vars {
            self.generate_global_var(var_decl)?;
        }
        for func in &program.functions {
            self.declare_function(func)?;
        }
        for func in &program.functions {
            self.generate_function(func)?;
        }
        Ok(())
    }

    fn declare_function(&mut self, func: &Function) -> Result<(), String> {
        let mut param_types: Vec<inkwell::types::BasicMetadataTypeEnum<'ctx>> = Vec::new();
        for param in &func.params {
            if param.is_ref {
                param_types.push(
                    self.context()
                        .ptr_type(inkwell::AddressSpace::default())
                        .into(),
                );
            } else {
                param_types.push(self.type_to_llvm_basic(&param.param_type)?.into());
            }
        }
        let fn_type = match &func.return_type {
            Some(Type::Void) | None => self.context().i64_type().fn_type(&param_types, false),
            Some(ret_type) => {
                let llvm_ret = self.type_to_llvm_basic(ret_type)?;
                match llvm_ret {
                    inkwell::types::BasicTypeEnum::IntType(t) => t.fn_type(&param_types, false),
                    inkwell::types::BasicTypeEnum::FloatType(t) => t.fn_type(&param_types, false),
                    inkwell::types::BasicTypeEnum::PointerType(t) => t.fn_type(&param_types, false),
                    inkwell::types::BasicTypeEnum::ArrayType(t) => t.fn_type(&param_types, false),
                    inkwell::types::BasicTypeEnum::StructType(t) => t.fn_type(&param_types, false),
                    inkwell::types::BasicTypeEnum::VectorType(t) => t.fn_type(&param_types, false),
                    inkwell::types::BasicTypeEnum::ScalableVectorType(t) => {
                        t.fn_type(&param_types, false)
                    }
                }
            }
        };
        self.module().add_function(&func.name, fn_type, None);
        Ok(())
    }

    fn generate_global_var(&self, var_decl: &VarDecl) -> Result<(), String> {
        self.set_var_type(&var_decl.name, var_decl.var_type.clone());
        match &var_decl.value {
            Expr::Literal(Literal::Int(value)) => {
                let init_value = self.context().i64_type().const_int(*value as u64, false);
                let global = self.module().add_global(
                    self.context().i64_type(),
                    Some(inkwell::AddressSpace::default()),
                    &var_decl.name,
                );
                global.set_initializer(&init_value);
                self.named_values
                    .borrow_mut()
                    .insert(var_decl.name.clone(), global.as_pointer_value().into());
                Ok(())
            }
            Expr::Literal(Literal::Float(value)) => {
                let init_value = self.context().f64_type().const_float(*value);
                let global = self.module().add_global(
                    self.context().f64_type(),
                    Some(inkwell::AddressSpace::default()),
                    &var_decl.name,
                );
                global.set_initializer(&init_value);
                self.named_values
                    .borrow_mut()
                    .insert(var_decl.name.clone(), global.as_pointer_value().into());
                Ok(())
            }
            Expr::Literal(Literal::String(s)) => {
                let str_bytes = s.as_bytes();
                let str_type = self
                    .context()
                    .i8_type()
                    .array_type(str_bytes.len() as u32 + 1);
                let global_str = self.module().add_global(
                    str_type,
                    Some(inkwell::AddressSpace::default()),
                    &var_decl.name,
                );
                let mut bytes_with_null = str_bytes.to_vec();
                bytes_with_null.push(0);
                global_str.set_initializer(&self.context().const_string(&bytes_with_null, false));
                self.named_values
                    .borrow_mut()
                    .insert(var_decl.name.clone(), global_str.as_pointer_value().into());
                Ok(())
            }
            _ => Err("Global variable must be initialized with constant".to_string()),
        }
    }

    fn generate_function(&mut self, func: &Function) -> Result<(), String> {
        let llvm_func = self
            .module()
            .get_function(&func.name)
            .ok_or(format!("Function {} not declared", func.name))?;
        let entry = self.context().append_basic_block(llvm_func, "entry");
        self.builder().position_at_end(entry);

        for (i, param) in func.params.iter().enumerate() {
            let param_val = llvm_func.get_nth_param(i as u32).unwrap();
            if param.is_ref {
                let ptr = param_val.into_pointer_value();
                self.named_values
                    .borrow_mut()
                    .insert(param.name.clone(), ptr.into());
                let mut ptype = param.param_type.clone();
                if let Type::Array(elem, 0) = &ptype {
                    ptype = Type::Array(elem.clone(), 1);
                }
                self.set_var_type(&param.name, ptype);
            } else {
                let param_type = self.type_to_llvm_basic(&param.param_type)?;
                let param_ptr = self
                    .builder()
                    .build_alloca(param_type, &param.name)
                    .unwrap();
                self.builder().build_store(param_ptr, param_val).unwrap();
                self.named_values
                    .borrow_mut()
                    .insert(param.name.clone(), param_ptr.into());
                self.set_var_type(&param.name, param.param_type.clone());
            }
        }

        for stmt in &func.body.statements {
            if self.current_block_terminated() {
                break;
            }
            self.generate_statement(stmt)?;
        }

        if !self.current_block_terminated() {
            if func.return_type == Some(Type::Void) || func.return_type.is_none() {
                let zero = self.context().i64_type().const_int(0, false);
                self.builder().build_return(Some(&zero)).unwrap();
            } else {
                let default_val =
                    self.default_value_for_type(&func.return_type.as_ref().unwrap())?;
                self.builder().build_return(Some(&default_val)).unwrap();
            }
        }
        Ok(())
    }

    fn default_value_for_type(&self, ty: &Type) -> Result<BasicValueEnum<'ctx>, String> {
        match ty {
            Type::Int => Ok(self.context().i64_type().const_int(0, false).into()),
            Type::Bool => Ok(self.context().bool_type().const_int(0, false).into()),
            Type::Float => Ok(self.context().f64_type().const_float(0.0).into()),
            Type::String => {
                let empty = self.build_string_literal("");
                Ok(empty.into())
            }
            Type::Array(_, _) => Err("Array default value not yet supported".to_string()),
            Type::Struct(name) => {
                let struct_def = self
                    .struct_defs
                    .borrow()
                    .get(name)
                    .cloned()
                    .ok_or(format!("Undefined struct: {}", name))?;
                let struct_type = self.get_struct_type(&struct_def)?;
                Ok(struct_type.const_zero().into())
            }
            Type::Void => Err("Cannot create default value for Void".to_string()),
        }
    }

    fn current_block_terminated(&self) -> bool {
        if let Some(block) = self.builder().get_insert_block() {
            block.get_terminator().is_some()
        } else {
            true
        }
    }

    fn to_bool_i1(
        &self,
        value: BasicValueEnum<'ctx>,
    ) -> Result<inkwell::values::IntValue<'ctx>, String> {
        let int_val = value.into_int_value();
        if int_val.get_type().get_bit_width() == 1 {
            return Ok(int_val);
        }
        let zero = int_val.get_type().const_zero();
        let is_nonzero = self
            .builder()
            .build_int_compare(inkwell::IntPredicate::NE, int_val, zero, "to_bool")
            .unwrap();
        Ok(is_nonzero)
    }

    fn generate_statement(&self, stmt: &Statement) -> Result<(), String> {
        match stmt {
            Statement::Return(value) => {
                if let Some(expr) = value {
                    let llvm_value = self.generate_expression(expr)?;
                    self.builder().build_return(Some(&llvm_value)).unwrap();
                } else {
                    self.builder().build_return(None).unwrap();
                }
                Ok(())
            }
            Statement::Expression(expr) => {
                self.generate_expression(expr)?;
                Ok(())
            }
            Statement::VarDecl(var_decl) => {
                self.set_var_type(&var_decl.name, var_decl.var_type.clone());
                let var_type = &var_decl.var_type;
                if matches!(var_type, Type::Array(_, _) | Type::Struct(_)) {
                    let llvm_type = self.type_to_llvm_basic(var_type)?;
                    let ptr = self
                        .builder()
                        .build_alloca(llvm_type, &var_decl.name)
                        .unwrap();
                    if let Expr::ArrayLiteral(ref elements) = var_decl.value {
                        if let Type::Array(_, size) = var_type {
                            for (i, elem_expr) in elements.iter().enumerate() {
                                let elem_val = self.generate_expression(elem_expr)?;
                                let elem_ptr = unsafe {
                                    self.builder()
                                        .build_in_bounds_gep(
                                            llvm_type,
                                            ptr,
                                            &[
                                                self.context().i64_type().const_int(0, false),
                                                self.context()
                                                    .i64_type()
                                                    .const_int(i as u64, false),
                                            ],
                                            "arr_init",
                                        )
                                        .unwrap()
                                };
                                self.builder().build_store(elem_ptr, elem_val).unwrap();
                            }
                            let _ = size;
                        }
                    } else {
                        let init_value = self.generate_expression(&var_decl.value)?;
                        self.builder().build_store(ptr, init_value).unwrap();
                    }
                    self.named_values
                        .borrow_mut()
                        .insert(var_decl.name.clone(), ptr.into());
                }
                Ok(())
            }
            Statement::ShortDecl(short_decl) => {
                let init_value = self.generate_expression(&short_decl.value)?;
                let inferred_type = self.infer_type_from_value(&init_value);
                self.set_var_type(&short_decl.name, inferred_type);
                let ptr = self
                    .builder()
                    .build_alloca(init_value.get_type(), &short_decl.name)
                    .unwrap();
                self.builder().build_store(ptr, init_value).unwrap();
                self.named_values
                    .borrow_mut()
                    .insert(short_decl.name.clone(), ptr.into());
                Ok(())
            }
            Statement::Assign(assign) => {
                let value = self.generate_expression(&assign.value)?;
                match &assign.target {
                    Expr::Identifier(name) => {
                        let ptr_val = self
                            .named_values
                            .borrow()
                            .get(name)
                            .copied()
                            .ok_or(format!("Undefined variable: {}", name))?;
                        let ptr = ptr_val.into_pointer_value();
                        self.builder().build_store(ptr, value).unwrap();
                    }
                    Expr::ArrayAccess(arr_expr, idx_expr) => {
                        let arr_ptr = self.generate_lvalue(arr_expr)?;
                        let index_val = self.generate_expression(idx_expr)?;
                        let var_name = match arr_expr.as_ref() {
                            Expr::Identifier(n) => n.clone(),
                            _ => return Err("Complex array assignment not supported".to_string()),
                        };
                        let var_type = self.get_var_type(&var_name);
                        let arr_type = self.type_to_llvm_basic(&var_type)?;
                        let elem_ptr = unsafe {
                            self.builder()
                                .build_in_bounds_gep(
                                    arr_type,
                                    arr_ptr,
                                    &[
                                        self.context().i64_type().const_int(0, false),
                                        index_val.into_int_value(),
                                    ],
                                    "elem_ptr",
                                )
                                .unwrap()
                        };
                        self.builder().build_store(elem_ptr, value).unwrap();
                    }
                    Expr::FieldAccess(struct_expr, field_name) => {
                        let struct_ptr = self.generate_lvalue(struct_expr)?;
                        let struct_type_name = match struct_expr.as_ref() {
                            Expr::Identifier(n) => n.clone(),
                            _ => return Err("Complex field assignment not supported".to_string()),
                        };
                        let var_type = self.get_var_type(&struct_type_name);
                        if let Type::Struct(type_name) = &var_type {
                            let struct_def = self
                                .struct_defs
                                .borrow()
                                .get(type_name)
                                .cloned()
                                .ok_or(format!("Undefined struct: {}", type_name))?;
                            let llvm_struct_type = self.get_struct_type(&struct_def)?;
                            let field_idx = struct_def
                                .fields
                                .iter()
                                .position(|f| &f.name == field_name)
                                .ok_or(format!("Unknown field: {}.{}", type_name, field_name))?;
                            let field_ptr = self
                                .builder()
                                .build_struct_gep(
                                    llvm_struct_type,
                                    struct_ptr,
                                    field_idx as u32,
                                    &format!("{}_{}_ptr", type_name, field_name),
                                )
                                .unwrap();
                            self.builder().build_store(field_ptr, value).unwrap();
                        } else {
                            return Err("Field access on non-struct type".to_string());
                        }
                    }
                    _ => return Err("Invalid assignment target".to_string()),
                }
                Ok(())
            }
            Statement::If(if_stmt) => self.generate_if_stmt(if_stmt),
            Statement::For(for_stmt) => self.generate_for_stmt(for_stmt),
            Statement::While(while_stmt) => self.generate_while_stmt(while_stmt),
            Statement::Switch(switch_stmt) => self.generate_switch_stmt(switch_stmt),
            Statement::MultiVarDecl(mvd) => self.generate_multi_var_decl(mvd),
            Statement::MultiShortDecl(msd) => self.generate_multi_short_decl(msd),
            Statement::IncDec(target, is_increment) => {
                let one = self.context().i64_type().const_int(1, false);
                match target {
                    Expr::Identifier(name) => {
                        let ptr_val = self
                            .named_values
                            .borrow()
                            .get(name)
                            .copied()
                            .ok_or(format!("Undefined variable: {}", name))?;
                        let ptr = ptr_val.into_pointer_value();
                        let current = self
                            .builder()
                            .build_load(self.context().i64_type(), ptr, "inc_load")
                            .unwrap();
                        let new_val = if *is_increment {
                            self.builder()
                                .build_int_add(current.into_int_value(), one, "inc_add")
                                .unwrap()
                        } else {
                            self.builder()
                                .build_int_sub(current.into_int_value(), one, "dec_sub")
                                .unwrap()
                        };
                        self.builder().build_store(ptr, new_val).unwrap();
                    }
                    Expr::ArrayAccess(arr_expr, idx_expr) => {
                        let arr_ptr = self.generate_lvalue(arr_expr)?;
                        let index_val = self.generate_expression(idx_expr)?;
                        let var_name = match arr_expr.as_ref() {
                            Expr::Identifier(n) => n.clone(),
                            _ => return Err("Complex array inc/dec not supported".to_string()),
                        };
                        let var_type = self.get_var_type(&var_name);
                        let arr_type = self.type_to_llvm_basic(&var_type)?;
                        let elem_ptr = unsafe {
                            self.builder()
                                .build_in_bounds_gep(
                                    arr_type,
                                    arr_ptr,
                                    &[
                                        self.context().i64_type().const_int(0, false),
                                        index_val.into_int_value(),
                                    ],
                                    "elem_ptr",
                                )
                                .unwrap()
                        };
                        let current = self
                            .builder()
                            .build_load(self.context().i64_type(), elem_ptr, "inc_load")
                            .unwrap();
                        let new_val = if *is_increment {
                            self.builder()
                                .build_int_add(current.into_int_value(), one, "inc_add")
                                .unwrap()
                        } else {
                            self.builder()
                                .build_int_sub(current.into_int_value(), one, "dec_sub")
                                .unwrap()
                        };
                        self.builder().build_store(elem_ptr, new_val).unwrap();
                    }
                    Expr::FieldAccess(struct_expr, field_name) => {
                        let struct_ptr = self.generate_lvalue(struct_expr)?;
                        let struct_type_name = match struct_expr.as_ref() {
                            Expr::Identifier(n) => n.clone(),
                            _ => return Err("Complex field inc/dec not supported".to_string()),
                        };
                        let var_type = self.get_var_type(&struct_type_name);
                        if let Type::Struct(type_name) = &var_type {
                            let struct_def = self
                                .struct_defs
                                .borrow()
                                .get(type_name)
                                .cloned()
                                .ok_or(format!("Undefined struct: {}", type_name))?;
                            let llvm_struct_type = self.get_struct_type(&struct_def)?;
                            let field_idx = struct_def
                                .fields
                                .iter()
                                .position(|f| &f.name == field_name)
                                .ok_or(format!("Unknown field: {}.{}", type_name, field_name))?;
                            let field_ptr = self
                                .builder()
                                .build_struct_gep(
                                    llvm_struct_type,
                                    struct_ptr,
                                    field_idx as u32,
                                    &format!("{}_{}_ptr", type_name, field_name),
                                )
                                .unwrap();
                            let current = self
                                .builder()
                                .build_load(self.context().i64_type(), field_ptr, "inc_load")
                                .unwrap();
                            let new_val = if *is_increment {
                                self.builder()
                                    .build_int_add(current.into_int_value(), one, "inc_add")
                                    .unwrap()
                            } else {
                                self.builder()
                                    .build_int_sub(current.into_int_value(), one, "dec_sub")
                                    .unwrap()
                            };
                            self.builder().build_store(field_ptr, new_val).unwrap();
                        } else {
                            return Err("Field inc/dec on non-struct type".to_string());
                        }
                    }
                    _ => return Err("Invalid inc/dec target".to_string()),
                }
                Ok(())
            }
            Statement::Break => {
                let loop_ctx = self
                    .loop_stack
                    .borrow()
                    .last()
                    .ok_or("break outside of loop")?
                    .clone();
                if !self.current_block_terminated() {
                    self.builder()
                        .build_unconditional_branch(loop_ctx.break_block)
                        .unwrap();
                }
                Ok(())
            }
            Statement::Continue => {
                let loop_ctx = self
                    .loop_stack
                    .borrow()
                    .last()
                    .ok_or("continue outside of loop")?
                    .clone();
                if !self.current_block_terminated() {
                    self.builder()
                        .build_unconditional_branch(loop_ctx.continue_block)
                        .unwrap();
                }
                Ok(())
            }
        }
    }

    fn infer_type_from_value(&self, value: &BasicValueEnum<'ctx>) -> Type {
        if value.is_float_value() {
            Type::Float
        } else if value.is_pointer_value() {
            Type::String
        } else if value.is_int_value() {
            let int_val = value.into_int_value();
            if int_val.get_type().get_bit_width() == 1 {
                Type::Bool
            } else {
                Type::Int
            }
        } else {
            Type::Int
        }
    }

    fn generate_if_stmt(&self, if_stmt: &IfStmt) -> Result<(), String> {
        let function = self
            .builder()
            .get_insert_block()
            .ok_or("No insert block")?
            .get_parent()
            .ok_or("No parent function")?;
        let then_block = self.context().append_basic_block(function, "then");
        let else_block = self.context().append_basic_block(function, "else");
        let merge_block = self.context().append_basic_block(function, "merge");
        let cond_value = self.generate_expression(&if_stmt.condition)?;
        let cond_i1 = self.to_bool_i1(cond_value)?;
        self.builder()
            .build_conditional_branch(cond_i1, then_block, else_block)
            .unwrap();
        self.builder().position_at_end(then_block);
        for stmt in &if_stmt.then_block.statements {
            self.generate_statement(stmt)?;
            if self.current_block_terminated() {
                break;
            }
        }
        if !self.current_block_terminated() {
            self.builder()
                .build_unconditional_branch(merge_block)
                .unwrap();
        }
        self.builder().position_at_end(else_block);
        self.generate_else_body(&if_stmt.else_block, merge_block)?;
        self.builder().position_at_end(merge_block);
        Ok(())
    }

    fn generate_else_if_chain(
        &self,
        if_stmt: &IfStmt,
        merge_block: inkwell::basic_block::BasicBlock<'ctx>,
    ) -> Result<(), String> {
        let function = self
            .builder()
            .get_insert_block()
            .ok_or("No insert block")?
            .get_parent()
            .ok_or("No parent function")?;
        let then_block = self.context().append_basic_block(function, "then");
        let else_block = self.context().append_basic_block(function, "else");
        let cond_value = self.generate_expression(&if_stmt.condition)?;
        let cond_i1 = self.to_bool_i1(cond_value)?;
        self.builder()
            .build_conditional_branch(cond_i1, then_block, else_block)
            .unwrap();
        self.builder().position_at_end(then_block);
        for stmt in &if_stmt.then_block.statements {
            self.generate_statement(stmt)?;
            if self.current_block_terminated() {
                break;
            }
        }
        if !self.current_block_terminated() {
            self.builder()
                .build_unconditional_branch(merge_block)
                .unwrap();
        }
        self.builder().position_at_end(else_block);
        self.generate_else_body(&if_stmt.else_block, merge_block)?;
        Ok(())
    }

    fn generate_else_body(
        &self,
        else_block: &Option<Block>,
        merge_block: inkwell::basic_block::BasicBlock<'ctx>,
    ) -> Result<(), String> {
        if let Some(else_body) = else_block {
            if else_body.statements.len() == 1 {
                if let Statement::If(nested_if) = &else_body.statements[0] {
                    self.generate_else_if_chain(nested_if, merge_block)?;
                } else {
                    for stmt in &else_body.statements {
                        self.generate_statement(stmt)?;
                    }
                    if !self.current_block_terminated() {
                        self.builder()
                            .build_unconditional_branch(merge_block)
                            .unwrap();
                    }
                }
            } else {
                for stmt in &else_body.statements {
                    self.generate_statement(stmt)?;
                }
                if !self.current_block_terminated() {
                    self.builder()
                        .build_unconditional_branch(merge_block)
                        .unwrap();
                }
            }
        } else {
            self.builder()
                .build_unconditional_branch(merge_block)
                .unwrap();
        }
        Ok(())
    }

    fn generate_for_stmt(&self, for_stmt: &ForStmt) -> Result<(), String> {
        let function = self
            .builder()
            .get_insert_block()
            .ok_or("No insert block")?
            .get_parent()
            .ok_or("No parent function")?;
        if let Some(init) = &for_stmt.init {
            self.generate_statement(init)?;
        }
        let cond_block = self.context().append_basic_block(function, "for.cond");
        let body_block = self.context().append_basic_block(function, "for.body");
        let step_block = self.context().append_basic_block(function, "for.step");
        let merge_block = self.context().append_basic_block(function, "for.merge");
        self.builder()
            .build_unconditional_branch(cond_block)
            .unwrap();
        self.builder().position_at_end(cond_block);
        if let Some(condition) = &for_stmt.condition {
            let cond_value = self.generate_expression(condition)?;
            let cond_i1 = self.to_bool_i1(cond_value)?;
            self.builder()
                .build_conditional_branch(cond_i1, body_block, merge_block)
                .unwrap();
        } else {
            self.builder()
                .build_unconditional_branch(body_block)
                .unwrap();
        }
        self.loop_stack.borrow_mut().push(LoopContext {
            break_block: merge_block,
            continue_block: step_block,
        });
        self.builder().position_at_end(body_block);
        for stmt in &for_stmt.body.statements {
            self.generate_statement(stmt)?;
            if self.current_block_terminated() {
                break;
            }
        }
        if !self.current_block_terminated() {
            self.builder()
                .build_unconditional_branch(step_block)
                .unwrap();
        }
        self.loop_stack.borrow_mut().pop();
        self.builder().position_at_end(step_block);
        if let Some(step) = &for_stmt.step {
            self.generate_statement(step)?;
        }
        if !self.current_block_terminated() {
            self.builder()
                .build_unconditional_branch(cond_block)
                .unwrap();
        }
        self.builder().position_at_end(merge_block);
        Ok(())
    }

    fn generate_while_stmt(&self, while_stmt: &WhileStmt) -> Result<(), String> {
        let function = self
            .builder()
            .get_insert_block()
            .ok_or("No insert block")?
            .get_parent()
            .ok_or("No parent function")?;
        let cond_block = self.context().append_basic_block(function, "while.cond");
        let body_block = self.context().append_basic_block(function, "while.body");
        let merge_block = self.context().append_basic_block(function, "while.merge");
        self.builder()
            .build_unconditional_branch(cond_block)
            .unwrap();
        self.builder().position_at_end(cond_block);
        let cond_value = self.generate_expression(&while_stmt.condition)?;
        let cond_i1 = self.to_bool_i1(cond_value)?;
        self.builder()
            .build_conditional_branch(cond_i1, body_block, merge_block)
            .unwrap();
        self.loop_stack.borrow_mut().push(LoopContext {
            break_block: merge_block,
            continue_block: cond_block,
        });
        self.builder().position_at_end(body_block);
        for stmt in &while_stmt.body.statements {
            self.generate_statement(stmt)?;
            if self.current_block_terminated() {
                break;
            }
        }
        if !self.current_block_terminated() {
            self.builder()
                .build_unconditional_branch(cond_block)
                .unwrap();
        }
        self.loop_stack.borrow_mut().pop();
        self.builder().position_at_end(merge_block);
        Ok(())
    }

    fn generate_switch_stmt(&self, switch_stmt: &SwitchStmt) -> Result<(), String> {
        let function = self
            .builder()
            .get_insert_block()
            .ok_or("No insert block")?
            .get_parent()
            .ok_or("No parent function")?;
        let switch_val = self.generate_expression(&switch_stmt.expression)?;
        let switch_int = switch_val.into_int_value();
        let merge_block = self.context().append_basic_block(function, "switch.merge");
        let default_block = self
            .context()
            .append_basic_block(function, "switch.default");

        let mut case_blocks: Vec<(
            inkwell::values::IntValue<'ctx>,
            inkwell::basic_block::BasicBlock<'ctx>,
        )> = Vec::new();
        for (i, case) in switch_stmt.cases.iter().enumerate() {
            let case_block = self
                .context()
                .append_basic_block(function, &format!("switch.case.{}", i));
            let case_val = if let Some(cond) = &case.condition {
                self.generate_expression(cond)?
            } else {
                self.context().i64_type().const_int(0, false).into()
            };
            case_blocks.push((case_val.into_int_value(), case_block));
        }

        let dest = if switch_stmt.default_block.is_some() {
            default_block
        } else {
            merge_block
        };
        let switch_inst = self
            .builder()
            .build_switch(switch_int, dest, &case_blocks)
            .unwrap();
        let _ = switch_inst;

        for (i, case) in switch_stmt.cases.iter().enumerate() {
            let case_block = case_blocks[i].1;
            self.builder().position_at_end(case_block);
            for stmt in &case.block.statements {
                self.generate_statement(stmt)?;
                if self.current_block_terminated() {
                    break;
                }
            }
            if !self.current_block_terminated() {
                self.builder()
                    .build_unconditional_branch(merge_block)
                    .unwrap();
            }
        }

        self.builder().position_at_end(default_block);
        if let Some(default_body) = &switch_stmt.default_block {
            for stmt in &default_body.statements {
                self.generate_statement(stmt)?;
                if self.current_block_terminated() {
                    break;
                }
            }
        }
        if !self.current_block_terminated() {
            self.builder()
                .build_unconditional_branch(merge_block)
                .unwrap();
        }

        self.builder().position_at_end(merge_block);
        Ok(())
    }

    fn generate_multi_var_decl(&self, mvd: &MultiVarDecl) -> Result<(), String> {
        for (name, expr) in mvd.names.iter().zip(mvd.values.iter()) {
            let value = self.generate_expression(expr)?;
            self.set_var_type(name, mvd.var_type.clone());
            let llvm_type = self.type_to_llvm_basic(&mvd.var_type)?;
            let ptr = self.builder().build_alloca(llvm_type, name).unwrap();
            self.builder().build_store(ptr, value).unwrap();
            self.named_values
                .borrow_mut()
                .insert(name.clone(), ptr.into());
        }
        Ok(())
    }

    fn generate_multi_short_decl(&self, msd: &MultiShortDecl) -> Result<(), String> {
        for (name, expr) in msd.names.iter().zip(msd.values.iter()) {
            let value = self.generate_expression(expr)?;
            let inferred_type = self.infer_type_from_value(&value);
            self.set_var_type(name, inferred_type);
            let ptr = self.builder().build_alloca(value.get_type(), name).unwrap();
            self.builder().build_store(ptr, value).unwrap();
            self.named_values
                .borrow_mut()
                .insert(name.clone(), ptr.into());
        }
        Ok(())
    }

    fn generate_expression(&self, expr: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        match expr {
            Expr::Literal(literal) => match literal {
                Literal::Int(value) => Ok(self
                    .context()
                    .i64_type()
                    .const_int(*value as u64, false)
                    .into()),
                Literal::Float(value) => Ok(self.context().f64_type().const_float(*value).into()),
                Literal::String(s) => Ok(self.build_string_literal(s).into()),
                Literal::Bool(value) => Ok(self
                    .context()
                    .bool_type()
                    .const_int(if *value { 1 } else { 0 }, false)
                    .into()),
            },
            Expr::Identifier(name) => {
                let ptr_val = self
                    .named_values
                    .borrow()
                    .get(name)
                    .copied()
                    .ok_or(format!("Undefined variable: {}", name))?;
                let ptr = ptr_val.into_pointer_value();
                let var_type = self.get_var_type(name);
                let llvm_type = self.type_to_llvm_basic(&var_type)?;
                Ok(self.builder().build_load(llvm_type, ptr, name).unwrap())
            }
            Expr::UnaryOp(op, expr) => {
                let value = self.generate_expression(expr)?;
                match op {
                    UnaryOperator::Negate => {
                        if value.is_float_value() {
                            let zero = self.context().f64_type().const_float(0.0);
                            Ok(self
                                .builder()
                                .build_float_sub(zero, value.into_float_value(), "neg")
                                .unwrap()
                                .into())
                        } else {
                            let zero = self.context().i64_type().const_int(0, false);
                            Ok(self
                                .builder()
                                .build_int_sub(zero, value.into_int_value(), "neg")
                                .unwrap()
                                .into())
                        }
                    }
                    UnaryOperator::Positive => Ok(value),
                    UnaryOperator::Not => {
                        let value_int = value.into_int_value();
                        if value_int.get_type().get_bit_width() == 1 {
                            let zero = self.context().bool_type().const_int(0, false);
                            let is_zero = self
                                .builder()
                                .build_int_compare(
                                    inkwell::IntPredicate::EQ,
                                    value_int,
                                    zero,
                                    "is_zero",
                                )
                                .unwrap();
                            Ok(is_zero.into())
                        } else {
                            let is_zero = self
                                .builder()
                                .build_int_compare(
                                    inkwell::IntPredicate::EQ,
                                    value_int,
                                    self.context().i64_type().const_int(0, false),
                                    "is_zero",
                                )
                                .unwrap();
                            Ok(self
                                .builder()
                                .build_int_z_extend(
                                    is_zero,
                                    self.context().i64_type(),
                                    "not_result",
                                )
                                .unwrap()
                                .into())
                        }
                    }
                    UnaryOperator::BitwiseNot => {
                        let value_int = value.into_int_value();
                        Ok(self.builder().build_not(value_int, "bnot").unwrap().into())
                    }
                }
            }
            Expr::BinaryOp(left, op, right) => match op {
                Operator::LogicalAnd => {
                    let lhs = self.generate_expression(left)?;
                    let function = self
                        .builder()
                        .get_insert_block()
                        .unwrap()
                        .get_parent()
                        .unwrap();
                    let id = self.pio.pub_next_id();
                    let rhs_block = self
                        .context()
                        .append_basic_block(function, &format!("and_rhs_{}", id));
                    let merge_block = self
                        .context()
                        .append_basic_block(function, &format!("and_merge_{}", id));
                    let lhs_i1 = self.to_bool_i1(lhs)?;
                    let lhs_block = self.builder().get_insert_block().unwrap();
                    self.builder()
                        .build_conditional_branch(lhs_i1, rhs_block, merge_block)
                        .unwrap();
                    self.builder().position_at_end(rhs_block);
                    let rhs = self.generate_expression(right)?;
                    let rhs_i1 = self.to_bool_i1(rhs)?;
                    let rhs_end_block = self.builder().get_insert_block().unwrap();
                    self.builder()
                        .build_unconditional_branch(merge_block)
                        .unwrap();
                    self.builder().position_at_end(merge_block);
                    let phi = self
                        .builder()
                        .build_phi(self.context().bool_type(), "and_phi")
                        .unwrap();
                    phi.add_incoming(&[
                        (&self.context().bool_type().const_int(0, false), lhs_block),
                        (&rhs_i1, rhs_end_block),
                    ]);
                    let result = self
                        .builder()
                        .build_int_z_extend(
                            phi.as_basic_value().into_int_value(),
                            self.context().i64_type(),
                            "and_result",
                        )
                        .unwrap();
                    Ok(result.into())
                }
                Operator::LogicalOr => {
                    let lhs = self.generate_expression(left)?;
                    let function = self
                        .builder()
                        .get_insert_block()
                        .unwrap()
                        .get_parent()
                        .unwrap();
                    let id = self.pio.pub_next_id();
                    let rhs_block = self
                        .context()
                        .append_basic_block(function, &format!("or_rhs_{}", id));
                    let merge_block = self
                        .context()
                        .append_basic_block(function, &format!("or_merge_{}", id));
                    let lhs_i1 = self.to_bool_i1(lhs)?;
                    let lhs_block = self.builder().get_insert_block().unwrap();
                    self.builder()
                        .build_conditional_branch(lhs_i1, merge_block, rhs_block)
                        .unwrap();
                    self.builder().position_at_end(rhs_block);
                    let rhs = self.generate_expression(right)?;
                    let rhs_i1 = self.to_bool_i1(rhs)?;
                    let rhs_end_block = self.builder().get_insert_block().unwrap();
                    self.builder()
                        .build_unconditional_branch(merge_block)
                        .unwrap();
                    self.builder().position_at_end(merge_block);
                    let phi = self
                        .builder()
                        .build_phi(self.context().bool_type(), "or_phi")
                        .unwrap();
                    phi.add_incoming(&[
                        (&self.context().bool_type().const_int(1, false), lhs_block),
                        (&rhs_i1, rhs_end_block),
                    ]);
                    let result = self
                        .builder()
                        .build_int_z_extend(
                            phi.as_basic_value().into_int_value(),
                            self.context().i64_type(),
                            "or_result",
                        )
                        .unwrap();
                    Ok(result.into())
                }
                _ => {
                    let lhs = self.generate_expression(left)?;
                    let rhs = self.generate_expression(right)?;
                    if lhs.is_float_value() || rhs.is_float_value() {
                        let lhs_f = if lhs.is_float_value() {
                            lhs.into_float_value()
                        } else {
                            self.builder()
                                .build_signed_int_to_float(
                                    lhs.into_int_value(),
                                    self.context().f64_type(),
                                    "int_to_float",
                                )
                                .unwrap()
                        };
                        let rhs_f = if rhs.is_float_value() {
                            rhs.into_float_value()
                        } else {
                            self.builder()
                                .build_signed_int_to_float(
                                    rhs.into_int_value(),
                                    self.context().f64_type(),
                                    "int_to_float",
                                )
                                .unwrap()
                        };
                        let result = match op {
                            Operator::Add => self
                                .builder()
                                .build_float_add(lhs_f, rhs_f, "fadd")
                                .unwrap()
                                .into(),
                            Operator::Subtract => self
                                .builder()
                                .build_float_sub(lhs_f, rhs_f, "fsub")
                                .unwrap()
                                .into(),
                            Operator::Multiply => self
                                .builder()
                                .build_float_mul(lhs_f, rhs_f, "fmul")
                                .unwrap()
                                .into(),
                            Operator::Divide => self
                                .builder()
                                .build_float_div(lhs_f, rhs_f, "fdiv")
                                .unwrap()
                                .into(),
                            Operator::Greater
                            | Operator::Less
                            | Operator::GreaterEqual
                            | Operator::LessEqual
                            | Operator::Equal
                            | Operator::NotEqual => {
                                let pred = match op {
                                    Operator::Greater => inkwell::FloatPredicate::OGT,
                                    Operator::Less => inkwell::FloatPredicate::OLT,
                                    Operator::GreaterEqual => inkwell::FloatPredicate::OGE,
                                    Operator::LessEqual => inkwell::FloatPredicate::OLE,
                                    Operator::Equal => inkwell::FloatPredicate::OEQ,
                                    Operator::NotEqual => inkwell::FloatPredicate::ONE,
                                    _ => unreachable!(),
                                };
                                let cmp = self
                                    .builder()
                                    .build_float_compare(pred, lhs_f, rhs_f, "cmp")
                                    .unwrap();
                                self.builder()
                                    .build_int_z_extend(cmp, self.context().i64_type(), "cmp_ext")
                                    .unwrap()
                                    .into()
                            }
                            _ => return Err("Unsupported float operator".to_string()),
                        };
                        Ok(result)
                    } else {
                        let lhs_int = lhs.into_int_value();
                        let rhs_int = rhs.into_int_value();
                        let result = match op {
                            Operator::Add => self
                                .builder()
                                .build_int_add(lhs_int, rhs_int, "sum")
                                .unwrap()
                                .into(),
                            Operator::Subtract => self
                                .builder()
                                .build_int_sub(lhs_int, rhs_int, "diff")
                                .unwrap()
                                .into(),
                            Operator::Multiply => self
                                .builder()
                                .build_int_mul(lhs_int, rhs_int, "prod")
                                .unwrap()
                                .into(),
                            Operator::Divide => self
                                .builder()
                                .build_int_signed_div(lhs_int, rhs_int, "quot")
                                .unwrap()
                                .into(),
                            Operator::Modulo => self
                                .builder()
                                .build_int_signed_rem(lhs_int, rhs_int, "rem")
                                .unwrap()
                                .into(),
                            Operator::BitwiseAnd => self
                                .builder()
                                .build_and(lhs_int, rhs_int, "band")
                                .unwrap()
                                .into(),
                            Operator::BitwiseOr => self
                                .builder()
                                .build_or(lhs_int, rhs_int, "bor")
                                .unwrap()
                                .into(),
                            Operator::BitwiseXor => self
                                .builder()
                                .build_xor(lhs_int, rhs_int, "bxor")
                                .unwrap()
                                .into(),
                            Operator::Greater
                            | Operator::Less
                            | Operator::GreaterEqual
                            | Operator::LessEqual
                            | Operator::Equal
                            | Operator::NotEqual => {
                                let pred = match op {
                                    Operator::Greater => inkwell::IntPredicate::SGT,
                                    Operator::Less => inkwell::IntPredicate::SLT,
                                    Operator::GreaterEqual => inkwell::IntPredicate::SGE,
                                    Operator::LessEqual => inkwell::IntPredicate::SLE,
                                    Operator::Equal => inkwell::IntPredicate::EQ,
                                    Operator::NotEqual => inkwell::IntPredicate::NE,
                                    _ => unreachable!(),
                                };
                                let cmp = self
                                    .builder()
                                    .build_int_compare(pred, lhs_int, rhs_int, "cmp")
                                    .unwrap();
                                self.builder()
                                    .build_int_z_extend(cmp, self.context().i64_type(), "cmp_ext")
                                    .unwrap()
                                    .into()
                            }
                            _ => return Err("Unsupported int operator".to_string()),
                        };
                        Ok(result)
                    }
                }
            },
            Expr::Call(name, args) => {
                let function = self
                    .module()
                    .get_function(name)
                    .ok_or(format!("Undefined function: {}", name))?;
                let llvm_params = function.get_params();
                let mut arg_values: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> = Vec::new();

                for (i, arg) in args.iter().enumerate() {
                    let param = llvm_params.get(i);
                    let is_ptr_param = param.map_or(false, |p| p.is_pointer_value());
                    if is_ptr_param {
                        if let Expr::Identifier(var_name) = arg {
                            let ptr_val = self
                                .named_values
                                .borrow()
                                .get(var_name)
                                .copied()
                                .ok_or(format!("Undefined variable: {}", var_name))?;
                            arg_values.push(ptr_val.into());
                        } else {
                            return Err("ref parameter must be a variable".to_string());
                        }
                    } else {
                        let val = self.generate_expression(arg)?;
                        arg_values.push(val.into());
                    }
                }
                let call_site = self
                    .builder()
                    .build_call(function, &arg_values, &format!("call_{}", name))
                    .unwrap();
                if function.get_type().get_return_type().is_some() {
                    Ok(call_site
                        .as_any_value_enum()
                        .try_into()
                        .unwrap_or_else(|_| self.context().i64_type().const_int(0, false).into()))
                } else {
                    Ok(self.context().i64_type().const_int(0, false).into())
                }
            }
            Expr::ModuleCall(module, func, args) => {
                if module != "pio" {
                    return Err(format!("Unknown module: {}", module));
                }
                self.generate_pio_call(func, args)
            }
            Expr::ArrayAccess(array_expr, index_expr) => {
                let arr_ptr = self.generate_lvalue(array_expr)?;
                let index_val = self.generate_expression(index_expr)?;
                let var_name = match array_expr.as_ref() {
                    Expr::Identifier(n) => n.clone(),
                    _ => return Err("Complex array access not supported".to_string()),
                };
                let var_type = self.get_var_type(&var_name);
                let arr_type = self.type_to_llvm_basic(&var_type)?;
                let elem_type = self.type_to_llvm_basic(&self.get_element_type(&var_type))?;
                let elem_ptr = unsafe {
                    self.builder()
                        .build_in_bounds_gep(
                            arr_type,
                            arr_ptr,
                            &[
                                self.context().i64_type().const_int(0, false),
                                index_val.into_int_value(),
                            ],
                            "elem_ptr",
                        )
                        .unwrap()
                };
                Ok(self
                    .builder()
                    .build_load(elem_type, elem_ptr, "elem")
                    .unwrap())
            }
            Expr::StructLiteral(name, fields) => {
                let struct_def = self
                    .struct_defs
                    .borrow()
                    .get(name)
                    .cloned()
                    .ok_or(format!("Undefined struct: {}", name))?;
                let struct_type = self.get_struct_type(&struct_def)?;
                let alloc = self
                    .builder()
                    .build_alloca(struct_type, &format!("{}_lit", name))
                    .unwrap();
                for (field_name, expr) in fields {
                    let field_idx = struct_def
                        .fields
                        .iter()
                        .position(|f| &f.name == field_name)
                        .ok_or(format!("Unknown field: {}.{}", name, field_name))?;
                    let field_ptr = self
                        .builder()
                        .build_struct_gep(
                            struct_type,
                            alloc,
                            field_idx as u32,
                            &format!("{}_{}", name, field_name),
                        )
                        .unwrap();
                    let value = self.generate_expression(expr)?;
                    self.builder().build_store(field_ptr, value).unwrap();
                }
                let struct_val = self
                    .builder()
                    .build_load(struct_type, alloc, &format!("{}_val", name))
                    .unwrap();
                Ok(struct_val.into())
            }
            Expr::ArrayLiteral(elements) => {
                let elem_type = self.context().i64_type();
                let arr_type = elem_type.array_type(elements.len() as u32);
                let alloc = self.builder().build_alloca(arr_type, "arr_lit").unwrap();
                for (i, elem_expr) in elements.iter().enumerate() {
                    let elem_val = self.generate_expression(elem_expr)?;
                    let elem_ptr = unsafe {
                        self.builder()
                            .build_in_bounds_gep(
                                arr_type,
                                alloc,
                                &[
                                    self.context().i64_type().const_int(0, false),
                                    self.context().i64_type().const_int(i as u64, false),
                                ],
                                "arr_lit_elem",
                            )
                            .unwrap()
                    };
                    self.builder().build_store(elem_ptr, elem_val).unwrap();
                }
                Ok(alloc.into())
            }
            Expr::FieldAccess(struct_expr, field_name) => {
                let struct_ptr = self.generate_lvalue(struct_expr)?;
                let struct_type_name = match struct_expr.as_ref() {
                    Expr::Identifier(n) => n.clone(),
                    _ => return Err("Complex field access not supported".to_string()),
                };
                let var_type = self.get_var_type(&struct_type_name);
                if let Type::Struct(type_name) = &var_type {
                    let struct_def = self
                        .struct_defs
                        .borrow()
                        .get(type_name)
                        .cloned()
                        .ok_or(format!("Undefined struct: {}", type_name))?;
                    let llvm_struct_type = self.get_struct_type(&struct_def)?;
                    let field_idx = struct_def
                        .fields
                        .iter()
                        .position(|f| &f.name == field_name)
                        .ok_or(format!("Unknown field: {}.{}", type_name, field_name))?;
                    let field_type =
                        self.type_to_llvm_basic(&struct_def.fields[field_idx].field_type)?;
                    let field_ptr = self
                        .builder()
                        .build_struct_gep(
                            llvm_struct_type,
                            struct_ptr,
                            field_idx as u32,
                            &format!("{}_{}", type_name, field_name),
                        )
                        .unwrap();
                    Ok(self
                        .builder()
                        .build_load(field_type, field_ptr, field_name)
                        .unwrap())
                } else {
                    Err("Field access on non-struct type".to_string())
                }
            }
        }
    }

    fn generate_lvalue(&self, expr: &Expr) -> Result<inkwell::values::PointerValue<'ctx>, String> {
        match expr {
            Expr::Identifier(name) => {
                let ptr_val = self
                    .named_values
                    .borrow()
                    .get(name)
                    .copied()
                    .ok_or(format!("Undefined variable: {}", name))?;
                Ok(ptr_val.into_pointer_value())
            }
            _ => Err("Invalid lvalue expression".to_string()),
        }
    }

    fn get_element_type(&self, ty: &Type) -> Type {
        match ty {
            Type::Array(elem, _) => *elem.clone(),
            _ => Type::Int,
        }
    }

    fn generate_pio_call(&self, func: &str, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        let zero = || self.context().i64_type().const_int(0, false).into();
        match func {
            "Println" => {
                for (i, arg) in args.iter().enumerate() {
                    let is_last = i == args.len() - 1;
                    match arg {
                        Expr::Literal(Literal::String(msg)) => {
                            if is_last {
                                self.pio.println_string(msg)?;
                            } else {
                                self.pio.print_string(msg)?;
                            }
                        }
                        Expr::Identifier(name)
                            if matches!(self.get_var_type(name), Type::Array(_, _)) =>
                        {
                            let var_type = self.get_var_type(name);
                            if let Type::Array(_, size) = &var_type {
                                let arr_ptr = self
                                    .named_values
                                    .borrow()
                                    .get(name)
                                    .copied()
                                    .ok_or(format!("Undefined variable: {}", name))?;
                                let arr_type = self.type_to_llvm_basic(&var_type)?;
                                let elem_type =
                                    self.type_to_llvm_basic(&self.get_element_type(&var_type))?;
                                for k in 0..*size {
                                    let elem_ptr = unsafe {
                                        self.builder()
                                            .build_in_bounds_gep(
                                                arr_type,
                                                arr_ptr.into_pointer_value(),
                                                &[
                                                    self.context().i64_type().const_int(0, false),
                                                    self.context()
                                                        .i64_type()
                                                        .const_int(k as u64, false),
                                                ],
                                                "print_elem",
                                            )
                                            .unwrap()
                                    };
                                    let elem_val = self
                                        .builder()
                                        .build_load(elem_type, elem_ptr, "elem")
                                        .unwrap();
                                    if k < *size - 1 || !is_last {
                                        self.pio.print_int_sp(elem_val.into())?;
                                    } else {
                                        self.pio.println_int(elem_val.into())?;
                                    }
                                }
                            }
                        }
                        _ => {
                            let value = self.generate_expression(arg)?;
                            if value.is_float_value() {
                                if is_last {
                                    self.pio.println_float(value)?;
                                } else {
                                    self.pio.print_float_sp(value)?;
                                }
                            } else if value.is_pointer_value() {
                                if is_last {
                                    self.pio.println_string_val(value)?;
                                } else {
                                    self.pio.print_string_val(value)?;
                                }
                            } else if value.is_int_value()
                                && value.into_int_value().get_type().get_bit_width() == 1
                            {
                                let extended = self
                                    .builder()
                                    .build_int_z_extend(
                                        value.into_int_value(),
                                        self.context().i64_type(),
                                        "bool_ext",
                                    )
                                    .unwrap();
                                if is_last {
                                    self.pio.println_int(extended.into())?;
                                } else {
                                    self.pio.print_int_sp(extended.into())?;
                                }
                            } else {
                                if is_last {
                                    self.pio.println_int(value)?;
                                } else {
                                    self.pio.print_int_sp(value)?;
                                }
                            }
                        }
                    }
                }
                Ok(zero())
            }
            "Print" => {
                for (i, arg) in args.iter().enumerate() {
                    let is_last = i == args.len() - 1;
                    match arg {
                        Expr::Literal(Literal::String(msg)) => {
                            if is_last {
                                self.pio.print_string(msg)?;
                            } else {
                                self.pio.print_string(&format!("{} ", msg))?;
                            }
                        }

                        _ => {
                            let value = self.generate_expression(arg)?;
                            if value.is_float_value() {
                                if is_last {
                                    self.pio.print_float(value)?;
                                } else {
                                    self.pio.print_float_sp(value)?;
                                }
                            } else if value.is_pointer_value() {
                                if is_last {
                                    self.pio.print_string_val(value)?;
                                } else {
                                    self.pio.print_string_val_sp(value)?;
                                }
                            } else if value.is_int_value()
                                && value.into_int_value().get_type().get_bit_width() == 1
                            {
                                let extended = self
                                    .builder()
                                    .build_int_z_extend(
                                        value.into_int_value(),
                                        self.context().i64_type(),
                                        "bool_ext",
                                    )
                                    .unwrap();
                                if is_last {
                                    self.pio.print_int(extended.into())?;
                                } else {
                                    self.pio.print_int_sp(extended.into())?;
                                }
                            } else {
                                if is_last {
                                    self.pio.print_int(value)?;
                                } else {
                                    self.pio.print_int_sp(value)?;
                                }
                            }
                        }
                    }
                }
                Ok(zero())
            }
            "Printf" => {
                if args.is_empty() {
                    return Err("Printf requires at least format string".to_string());
                }
                match &args[0] {
                    Expr::Literal(Literal::String(fmt)) => {
                        let mut evaluated_args = Vec::new();
                        for arg in &args[1..] {
                            evaluated_args.push(self.generate_expression(arg)?);
                        }
                        self.pio.printf(fmt, &evaluated_args)?;
                    }
                    _ => {
                        return Err("Printf first argument must be a format string".to_string());
                    }
                }
                Ok(zero())
            }
            "Sprintf" => {
                if args.is_empty() {
                    return Err("Sprintf requires at least format string".to_string());
                }
                match &args[0] {
                    Expr::Literal(Literal::String(fmt)) => {
                        let mut evaluated_args = Vec::new();
                        for arg in &args[1..] {
                            evaluated_args.push(self.generate_expression(arg)?);
                        }
                        self.pio.sprintf(fmt, &evaluated_args)
                    }
                    _ => Err("Sprintf first argument must be a format string".to_string()),
                }
            }
            "Scan" => {
                if args.len() != 1 {
                    return Err("Scan requires exactly 1 argument".to_string());
                }
                if let Expr::Identifier(var_name) = &args[0] {
                    let ptr_val = self
                        .named_values
                        .borrow()
                        .get(var_name)
                        .copied()
                        .ok_or(format!("Undefined variable: {}", var_name))?;
                    let var_type = self.get_var_type(var_name);
                    match var_type {
                        Type::Float => {
                            self.pio.scan_float(ptr_val.into_pointer_value())?;
                        }
                        Type::String => {
                            self.pio.scan_string(ptr_val.into_pointer_value())?;
                        }
                        _ => {
                            self.pio.scan_int(ptr_val.into_pointer_value())?;
                        }
                    }
                } else {
                    return Err("Scan argument must be a variable".to_string());
                }
                Ok(zero())
            }
            "Scanln" => {
                if args.len() != 1 {
                    return Err("Scanln requires exactly 1 argument".to_string());
                }
                if let Expr::Identifier(var_name) = &args[0] {
                    let ptr_val = self
                        .named_values
                        .borrow()
                        .get(var_name)
                        .copied()
                        .ok_or(format!("Undefined variable: {}", var_name))?;
                    let var_type = self.get_var_type(var_name);
                    match var_type {
                        Type::Float => {
                            self.pio.scanln_float(ptr_val.into_pointer_value())?;
                        }
                        Type::String => {
                            self.pio.scan_string(ptr_val.into_pointer_value())?;
                        }
                        _ => {
                            self.pio.scanln_int(ptr_val.into_pointer_value())?;
                        }
                    }
                } else {
                    return Err("Scanln argument must be a variable".to_string());
                }
                Ok(zero())
            }
            "Len" => {
                if args.len() != 1 {
                    return Err("Len requires exactly 1 argument".to_string());
                }
                let value = self.generate_expression(&args[0])?;
                self.pio.len_string(value)
            }
            "Atoi" => {
                if args.len() != 1 {
                    return Err("Atoi requires exactly 1 argument".to_string());
                }
                let value = self.generate_expression(&args[0])?;
                self.pio.atoi_string(value)
            }
            "Itoa" => {
                if args.len() != 1 {
                    return Err("Itoa requires exactly 1 argument".to_string());
                }
                let value = self.generate_expression(&args[0])?;
                self.pio.itoa_string(value)
            }
            "ReadFile" => {
                if args.len() != 1 {
                    return Err("ReadFile requires exactly 1 argument".to_string());
                }
                let path = self.generate_expression(&args[0])?;
                self.pio.read_file(path)
            }
            "WriteFile" => {
                if args.len() != 2 {
                    return Err("WriteFile requires exactly 2 arguments".to_string());
                }
                let path = self.generate_expression(&args[0])?;
                let content = self.generate_expression(&args[1])?;
                self.pio.write_file(path, content)
            }
            "Substring" => {
                if args.len() != 3 {
                    return Err("Substring requires 3 arguments: string, start, len".to_string());
                }
                let s = self.generate_expression(&args[0])?;
                let start = self.generate_expression(&args[1])?;
                let len = self.generate_expression(&args[2])?;
                self.pio.substring(s, start, len)
            }
            "Append" => {
                if args.len() != 2 {
                    return Err("Append requires exactly 2 arguments".to_string());
                }
                let a = self.generate_expression(&args[0])?;
                let b = self.generate_expression(&args[1])?;
                self.pio.append(a, b)
            }
            "StrEquals" => {
                if args.len() != 2 {
                    return Err("StrEquals requires exactly 2 arguments".to_string());
                }
                let a = self.generate_expression(&args[0])?;
                let b = self.generate_expression(&args[1])?;
                self.pio.str_equals(a, b)
            }
            "CharAt" => {
                if args.len() != 2 {
                    return Err("CharAt requires exactly 2 arguments".to_string());
                }
                let s = self.generate_expression(&args[0])?;
                let idx = self.generate_expression(&args[1])?;
                self.pio.char_at(s, idx)
            }
            "IsDigit" => {
                if args.len() != 1 {
                    return Err("IsDigit requires exactly 1 argument".to_string());
                }
                let c = self.generate_expression(&args[0])?;
                self.pio.is_digit(c)
            }
            "IsAlpha" => {
                if args.len() != 1 {
                    return Err("IsAlpha requires exactly 1 argument".to_string());
                }
                let c = self.generate_expression(&args[0])?;
                self.pio.is_alpha(c)
            }
            "IsSpace" => {
                if args.len() != 1 {
                    return Err("IsSpace requires exactly 1 argument".to_string());
                }
                let c = self.generate_expression(&args[0])?;
                self.pio.is_space(c)
            }
            "DynArrayNew" => self.pio.dynarr_new(),
            "DynArrayPush" => {
                if args.len() != 2 {
                    return Err("DynArrayPush requires 2 arguments".to_string());
                }
                let h = self.generate_expression(&args[0])?;
                let v = self.generate_expression(&args[1])?;
                self.pio.dynarr_push(h, v)?;
                Ok(self.context().i64_type().const_zero().into())
            }
            "DynArrayGet" => {
                if args.len() != 2 {
                    return Err("DynArrayGet requires 2 arguments".to_string());
                }
                let h = self.generate_expression(&args[0])?;
                let idx = self.generate_expression(&args[1])?;
                self.pio.dynarr_get(h, idx)
            }
            "DynArraySet" => {
                if args.len() != 3 {
                    return Err("DynArraySet requires 3 arguments".to_string());
                }
                let h = self.generate_expression(&args[0])?;
                let idx = self.generate_expression(&args[1])?;
                let v = self.generate_expression(&args[2])?;
                self.pio.dynarr_set(h, idx, v)?;
                Ok(self.context().i64_type().const_zero().into())
            }
            "DynArrayLen" => {
                if args.len() != 1 {
                    return Err("DynArrayLen requires 1 argument".to_string());
                }
                let h = self.generate_expression(&args[0])?;
                self.pio.dynarr_len(h)
            }
            "StrArrayNew" => self.pio.strarr_new(),
            "StrArrayPush" => {
                if args.len() != 2 {
                    return Err("StrArrayPush requires 2 arguments".to_string());
                }
                let h = self.generate_expression(&args[0])?;
                let s = self.generate_expression(&args[1])?;
                self.pio.strarr_push(h, s)?;
                Ok(self.context().i64_type().const_zero().into())
            }
            "StrArrayGet" => {
                if args.len() != 2 {
                    return Err("StrArrayGet requires 2 arguments".to_string());
                }
                let h = self.generate_expression(&args[0])?;
                let idx = self.generate_expression(&args[1])?;
                self.pio.strarr_get(h, idx)
            }
            "StrArrayLen" => {
                if args.len() != 1 {
                    return Err("StrArrayLen requires 1 argument".to_string());
                }
                let h = self.generate_expression(&args[0])?;
                self.pio.strarr_len(h)
            }
            "HashMapNew" => self.pio.hashmap_new(),
            "HashMapSet" => {
                if args.len() != 3 {
                    return Err("HashMapSet requires 3 arguments: map, key, value".to_string());
                }
                let h = self.generate_expression(&args[0])?;
                let k = self.generate_expression(&args[1])?;
                let v = self.generate_expression(&args[2])?;
                self.pio.hashmap_set(h, k, v)?;
                Ok(self.context().i64_type().const_zero().into())
            }
            "HashMapGet" => {
                if args.len() != 2 {
                    return Err("HashMapGet requires 2 arguments: map, key".to_string());
                }
                let h = self.generate_expression(&args[0])?;
                let k = self.generate_expression(&args[1])?;
                self.pio.hashmap_get(h, k)
            }
            "HashMapHas" => {
                if args.len() != 2 {
                    return Err("HashMapHas requires 2 arguments: map, key".to_string());
                }
                let h = self.generate_expression(&args[0])?;
                let k = self.generate_expression(&args[1])?;
                self.pio.hashmap_has(h, k)
            }
            "HashMapDel" => {
                if args.len() != 2 {
                    return Err("HashMapDel requires 2 arguments: map, key".to_string());
                }
                let h = self.generate_expression(&args[0])?;
                let k = self.generate_expression(&args[1])?;
                self.pio.hashmap_del(h, k)?;
                Ok(self.context().i64_type().const_zero().into())
            }
            "HashMapLen" => {
                if args.len() != 1 {
                    return Err("HashMapLen requires 1 argument".to_string());
                }
                let h = self.generate_expression(&args[0])?;
                self.pio.hashmap_len(h)
            }
            "HashMapKeys" => {
                if args.len() != 1 {
                    return Err("HashMapKeys requires 1 argument".to_string());
                }
                let h = self.generate_expression(&args[0])?;
                self.pio.hashmap_keys(h)
            }
            "HashMapClear" => {
                if args.len() != 1 {
                    return Err("HashMapClear requires 1 argument".to_string());
                }
                let h = self.generate_expression(&args[0])?;
                self.pio.hashmap_clear(h)?;
                Ok(self.context().i64_type().const_zero().into())
            }
            "Split" => {
                if args.len() != 2 {
                    return Err("Split requires 2 arguments: str, delim".to_string());
                }
                let s = self.generate_expression(&args[0])?;
                let d = self.generate_expression(&args[1])?;
                self.pio.split(s, d)
            }
            "Join" => {
                if args.len() != 2 {
                    return Err("Join requires 2 arguments: array, delim".to_string());
                }
                let arr = self.generate_expression(&args[0])?;
                let d = self.generate_expression(&args[1])?;
                self.pio.join(arr, d)
            }
            "Format" => {
                if args.is_empty() {
                    return Err("Format requires at least 1 argument".to_string());
                }
                if let Expr::Literal(Literal::String(fmt)) = &args[0] {
                    let mut eval_args = Vec::new();
                    for arg in &args[1..] {
                        eval_args.push(self.generate_expression(arg)?);
                    }
                    self.pio.format(fmt, &eval_args)
                } else {
                    Err("Format first argument must be a string literal".to_string())
                }
            }
            _ => Err(format!("Unknown pio function: {}", func)),
        }
    }

    fn build_string_literal(&self, s: &str) -> inkwell::values::PointerValue<'ctx> {
        let str_bytes = s.as_bytes();
        let mut bytes_with_null = str_bytes.to_vec();
        bytes_with_null.push(0);
        let str_type = self
            .context()
            .i8_type()
            .array_type(bytes_with_null.len() as u32);
        let global_str =
            self.module()
                .add_global(str_type, Some(inkwell::AddressSpace::default()), "str_lit");
        global_str.set_initializer(&self.context().const_string(&bytes_with_null, false));
        unsafe {
            self.builder()
                .build_in_bounds_gep(
                    str_type,
                    global_str.as_pointer_value(),
                    &[
                        self.context().i32_type().const_int(0, false),
                        self.context().i32_type().const_int(0, false),
                    ],
                    "str_ptr",
                )
                .unwrap()
        }
    }

    fn get_struct_type(
        &self,
        struct_def: &StructDef,
    ) -> Result<inkwell::types::StructType<'ctx>, String> {
        let field_types: Vec<inkwell::types::BasicTypeEnum<'ctx>> = struct_def
            .fields
            .iter()
            .map(|f| self.type_to_llvm_basic(&f.field_type))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self.context().struct_type(&field_types, false))
    }

    fn type_to_llvm_basic(&self, ty: &Type) -> Result<inkwell::types::BasicTypeEnum<'ctx>, String> {
        match ty {
            Type::Int => Ok(self.context().i64_type().into()),
            Type::Bool => Ok(self.context().bool_type().into()),
            Type::Float => Ok(self.context().f64_type().into()),
            Type::String => Ok(self
                .context()
                .ptr_type(inkwell::AddressSpace::default())
                .into()),
            Type::Array(elem_ty, size) => {
                let llvm_elem = self.type_to_llvm_basic(elem_ty)?;
                Ok(self.build_array_type(llvm_elem, *size as u32))
            }
            Type::Struct(name) => {
                let struct_def = self
                    .struct_defs
                    .borrow()
                    .get(name)
                    .cloned()
                    .ok_or(format!("Undefined struct: {}", name))?;
                Ok(self.get_struct_type(&struct_def)?.into())
            }
            Type::Void => Err("Void is not a basic type".to_string()),
        }
    }

    fn type_to_llvm_metadata(
        &self,
        ty: &Type,
    ) -> Result<inkwell::types::BasicMetadataTypeEnum<'ctx>, String> {
        match ty {
            Type::Int => Ok(self.context().i64_type().into()),
            Type::Bool => Ok(self.context().bool_type().into()),
            Type::Float => Ok(self.context().f64_type().into()),
            Type::String => Ok(self
                .context()
                .ptr_type(inkwell::AddressSpace::default())
                .into()),
            Type::Array(elem_ty, size) => {
                let llvm_elem = self.type_to_llvm_basic(elem_ty)?;
                Ok(self.build_array_type(llvm_elem, *size as u32).into())
            }
            Type::Struct(name) => {
                let struct_def = self
                    .struct_defs
                    .borrow()
                    .get(name)
                    .cloned()
                    .ok_or(format!("Undefined struct: {}", name))?;
                Ok(self.get_struct_type(&struct_def)?.into())
            }
            Type::Void => Err("Void parameter not allowed".to_string()),
        }
    }

    fn build_array_type(
        &self,
        elem: inkwell::types::BasicTypeEnum<'ctx>,
        size: u32,
    ) -> inkwell::types::BasicTypeEnum<'ctx> {
        use inkwell::types::BasicTypeEnum;
        match elem {
            BasicTypeEnum::IntType(t) => t.array_type(size).into(),
            BasicTypeEnum::FloatType(t) => t.array_type(size).into(),
            BasicTypeEnum::PointerType(t) => t.array_type(size).into(),
            BasicTypeEnum::ArrayType(t) => t.array_type(size).into(),
            BasicTypeEnum::StructType(t) => t.array_type(size).into(),
            BasicTypeEnum::VectorType(t) => t.array_type(size).into(),
            BasicTypeEnum::ScalableVectorType(t) => t.array_type(size).into(),
        }
    }

    pub fn verify_and_dump(&self) -> Result<(), String> {
        self.module()
            .verify()
            .map_err(|e| format!("Module verification failed: {:?}", e))?;
        Ok(())
    }

    pub fn get_module(&self) -> &Module<'ctx> {
        self.module()
    }
}
