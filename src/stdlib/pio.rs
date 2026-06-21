use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::AnyValue;
use inkwell::values::BasicValueEnum;
use std::cell::Cell;

pub struct Pio<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    counter: Cell<u32>,
}

impl<'ctx> Pio<'ctx> {
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        let printf_type = context.i32_type().fn_type(
            &[context.ptr_type(inkwell::AddressSpace::default()).into()],
            true,
        );
        module.add_function("printf", printf_type, None);
        let scanf_type = context.i32_type().fn_type(
            &[context.ptr_type(inkwell::AddressSpace::default()).into()],
            true,
        );
        module.add_function("scanf", scanf_type, None);
        let sprintf_type = context.i32_type().fn_type(
            &[context.ptr_type(inkwell::AddressSpace::default()).into()],
            true,
        );
        module.add_function("sprintf", sprintf_type, None);
        let ptr_ty = context.ptr_type(inkwell::AddressSpace::default());
        let strlen_type = context.i64_type().fn_type(&[ptr_ty.into()], false);
        module.add_function("strlen", strlen_type, None);
        let atoi_type = context.i32_type().fn_type(&[ptr_ty.into()], false);
        module.add_function("atoi", atoi_type, None);

        let read_file_type = ptr_ty.fn_type(&[ptr_ty.into()], false);
        module.add_function("coconut_read_file", read_file_type, None);
        let write_file_type = context
            .i32_type()
            .fn_type(&[ptr_ty.into(), ptr_ty.into()], false);
        module.add_function("coconut_write_file", write_file_type, None);
        let substring_type = ptr_ty.fn_type(
            &[
                ptr_ty.into(),
                context.i32_type().into(),
                context.i32_type().into(),
            ],
            false,
        );
        module.add_function("coconut_substring", substring_type, None);
        let append_type = ptr_ty.fn_type(&[ptr_ty.into(), ptr_ty.into()], false);
        module.add_function("coconut_append", append_type, None);
        let str_equals_type = context
            .i32_type()
            .fn_type(&[ptr_ty.into(), ptr_ty.into()], false);
        module.add_function("coconut_str_equals", str_equals_type, None);
        let char_fn_type = context
            .i32_type()
            .fn_type(&[context.i32_type().into()], false);
        module.add_function("coconut_is_digit", char_fn_type, None);
        module.add_function("coconut_is_alpha", char_fn_type, None);
        module.add_function("coconut_is_space", char_fn_type, None);
        module.add_function("coconut_is_digit", char_fn_type, None);
        module.add_function("coconut_is_alpha", char_fn_type, None);
        module.add_function("coconut_is_space", char_fn_type, None);

        let handle_new_type = ptr_ty.fn_type(&[], false);
        module.add_function("coconut_dynarr_new", handle_new_type, None);
        module.add_function("coconut_strarr_new", handle_new_type, None);
        let dynarr_push_type = context
            .void_type()
            .fn_type(&[ptr_ty.into(), context.i64_type().into()], false);
        module.add_function("coconut_dynarr_push", dynarr_push_type, None);
        let dynarr_get_type = context
            .i64_type()
            .fn_type(&[ptr_ty.into(), context.i32_type().into()], false);
        module.add_function("coconut_dynarr_get", dynarr_get_type, None);
        let dynarr_set_type = context.void_type().fn_type(
            &[
                ptr_ty.into(),
                context.i32_type().into(),
                context.i64_type().into(),
            ],
            false,
        );
        module.add_function("coconut_dynarr_set", dynarr_set_type, None);
        let dynarr_len_type = context.i32_type().fn_type(&[ptr_ty.into()], false);
        module.add_function("coconut_dynarr_len", dynarr_len_type, None);
        module.add_function("coconut_strarr_len", dynarr_len_type, None);
        let strarr_push_type = context
            .void_type()
            .fn_type(&[ptr_ty.into(), ptr_ty.into()], false);
        module.add_function("coconut_strarr_push", strarr_push_type, None);
        let strarr_get_type = ptr_ty.fn_type(&[ptr_ty.into(), context.i32_type().into()], false);
        module.add_function("coconut_strarr_get", strarr_get_type, None);

        let hm_new_type = ptr_ty.fn_type(&[], false);
        module.add_function("coconut_hashmap_new", hm_new_type, None);
        let hm_set_type = context
            .void_type()
            .fn_type(&[ptr_ty.into(), ptr_ty.into(), ptr_ty.into()], false);
        module.add_function("coconut_hashmap_set", hm_set_type, None);
        let hm_get_type = ptr_ty.fn_type(&[ptr_ty.into(), ptr_ty.into()], false);
        module.add_function("coconut_hashmap_get", hm_get_type, None);
        let hm_has_type = context
            .i32_type()
            .fn_type(&[ptr_ty.into(), ptr_ty.into()], false);
        module.add_function("coconut_hashmap_has", hm_has_type, None);

        let hm_del_type = context
            .void_type()
            .fn_type(&[ptr_ty.into(), ptr_ty.into()], false);
        module.add_function("coconut_hashmap_del", hm_del_type, None);

        let hm_len_type = context.i32_type().fn_type(&[ptr_ty.into()], false);
        module.add_function("coconut_hashmap_len", hm_len_type, None);

        let hm_keys_type = ptr_ty.fn_type(&[ptr_ty.into()], false);
        module.add_function("coconut_hashmap_keys", hm_keys_type, None);

        let hm_clear_type = context.void_type().fn_type(&[ptr_ty.into()], false);
        module.add_function("coconut_hashmap_clear", hm_clear_type, None);

        let split_type = ptr_ty.fn_type(&[ptr_ty.into(), ptr_ty.into()], false);
        module.add_function("coconut_split", split_type, None);

        let join_type = ptr_ty.fn_type(&[ptr_ty.into(), ptr_ty.into()], false);
        module.add_function("coconut_join", join_type, None);

        let format_type = ptr_ty.fn_type(&[ptr_ty.into()], true);
        module.add_function("coconut_format", format_type, None);

        Pio {
            context,
            module,
            builder,
            counter: Cell::new(0),
        }
    }

    fn next_id(&self) -> u32 {
        let id = self.counter.get();
        self.counter.set(id + 1);
        id
    }

    pub fn pub_next_id(&self) -> u32 {
        self.next_id()
    }

    pub fn context(&self) -> &'ctx Context {
        self.context
    }
    pub fn module(&self) -> &Module<'ctx> {
        &self.module
    }
    pub fn builder(&self) -> &Builder<'ctx> {
        &self.builder
    }

    fn build_format_global(
        &self,
        format_str: &str,
        prefix: &str,
    ) -> inkwell::values::PointerValue<'ctx> {
        let id = self.next_id();
        let full_str = format!("{}\0", format_str);
        let format_type = self.context.i8_type().array_type(full_str.len() as u32);
        let global = self.module.add_global(
            format_type,
            Some(inkwell::AddressSpace::default()),
            &format!("{}_{}", prefix, id),
        );
        global.set_initializer(&self.context.const_string(full_str.as_bytes(), false));
        self.builder
            .build_pointer_cast(
                global.as_pointer_value(),
                self.context.ptr_type(inkwell::AddressSpace::default()),
                "fmt_ptr",
            )
            .unwrap()
    }

    fn get_read_file(&self) -> Result<inkwell::values::FunctionValue<'ctx>, String> {
        self.module
            .get_function("coconut_read_file")
            .ok_or("coconut_read_file not found".to_string())
    }
    fn get_write_file(&self) -> Result<inkwell::values::FunctionValue<'ctx>, String> {
        self.module
            .get_function("coconut_write_file")
            .ok_or("coconut_write_file not found".to_string())
    }
    fn get_substring(&self) -> Result<inkwell::values::FunctionValue<'ctx>, String> {
        self.module
            .get_function("coconut_substring")
            .ok_or("coconut_substring not found".to_string())
    }
    fn get_append(&self) -> Result<inkwell::values::FunctionValue<'ctx>, String> {
        self.module
            .get_function("coconut_append")
            .ok_or("coconut_append not found".to_string())
    }
    fn get_str_equals(&self) -> Result<inkwell::values::FunctionValue<'ctx>, String> {
        self.module
            .get_function("coconut_str_equals")
            .ok_or("coconut_str_equals not found".to_string())
    }
    fn get_is_digit(&self) -> Result<inkwell::values::FunctionValue<'ctx>, String> {
        self.module
            .get_function("coconut_is_digit")
            .ok_or("coconut_is_digit not found".to_string())
    }
    fn get_is_alpha(&self) -> Result<inkwell::values::FunctionValue<'ctx>, String> {
        self.module
            .get_function("coconut_is_alpha")
            .ok_or("coconut_is_alpha not found".to_string())
    }
    fn get_is_space(&self) -> Result<inkwell::values::FunctionValue<'ctx>, String> {
        self.module
            .get_function("coconut_is_space")
            .ok_or("coconut_is_space not found".to_string())
    }

    pub fn read_file(&self, path: BasicValueEnum<'ctx>) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self.get_read_file()?;
        let call_site = self
            .builder
            .build_call(fn_val, &[path.into()], "read_file_call")
            .unwrap();
        Ok(call_site.as_any_value_enum().try_into().unwrap())
    }

    pub fn write_file(
        &self,
        path: BasicValueEnum<'ctx>,
        content: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self.get_write_file()?;
        let call_site = self
            .builder
            .build_call(fn_val, &[path.into(), content.into()], "write_file_call")
            .unwrap();
        let i32_val: inkwell::values::IntValue = call_site.as_any_value_enum().try_into().unwrap();
        Ok(self
            .builder
            .build_int_s_extend(i32_val, self.context.i64_type(), "wf_ext")
            .unwrap()
            .into())
    }

    pub fn substring(
        &self,
        s: BasicValueEnum<'ctx>,
        start: BasicValueEnum<'ctx>,
        len: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self.get_substring()?;
        let start_i32 = self
            .builder
            .build_int_truncate(start.into_int_value(), self.context.i32_type(), "start32")
            .unwrap();
        let len_i32 = self
            .builder
            .build_int_truncate(len.into_int_value(), self.context.i32_type(), "len32")
            .unwrap();
        let call_site = self
            .builder
            .build_call(
                fn_val,
                &[s.into(), start_i32.into(), len_i32.into()],
                "substring_call",
            )
            .unwrap();
        Ok(call_site.as_any_value_enum().try_into().unwrap())
    }

    pub fn append(
        &self,
        a: BasicValueEnum<'ctx>,
        b: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self.get_append()?;
        let call_site = self
            .builder
            .build_call(fn_val, &[a.into(), b.into()], "append_call")
            .unwrap();
        Ok(call_site.as_any_value_enum().try_into().unwrap())
    }

    pub fn str_equals(
        &self,
        a: BasicValueEnum<'ctx>,
        b: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self.get_str_equals()?;
        let call_site = self
            .builder
            .build_call(fn_val, &[a.into(), b.into()], "str_eq_call")
            .unwrap();
        let i32_val: inkwell::values::IntValue = call_site.as_any_value_enum().try_into().unwrap();
        Ok(self
            .builder
            .build_int_z_extend(i32_val, self.context.i64_type(), "eq_ext")
            .unwrap()
            .into())
    }

    pub fn char_at(
        &self,
        s: BasicValueEnum<'ctx>,
        index: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let ptr = unsafe {
            self.builder
                .build_in_bounds_gep(
                    self.context.i8_type(),
                    s.into_pointer_value(),
                    &[index.into_int_value()],
                    "char_ptr",
                )
                .unwrap()
        };
        let ch = self
            .builder
            .build_load(self.context.i8_type(), ptr, "char_val")
            .unwrap();
        Ok(self
            .builder
            .build_int_s_extend(ch.into_int_value(), self.context.i64_type(), "ch_ext")
            .unwrap()
            .into())
    }

    pub fn is_digit(&self, c: BasicValueEnum<'ctx>) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self.get_is_digit()?;
        let c_i32 = self
            .builder
            .build_int_truncate(c.into_int_value(), self.context.i32_type(), "c32")
            .unwrap();
        let call_site = self
            .builder
            .build_call(fn_val, &[c_i32.into()], "is_digit_call")
            .unwrap();
        let i32_val: inkwell::values::IntValue = call_site.as_any_value_enum().try_into().unwrap();
        Ok(self
            .builder
            .build_int_z_extend(i32_val, self.context.i64_type(), "digit_ext")
            .unwrap()
            .into())
    }

    pub fn is_alpha(&self, c: BasicValueEnum<'ctx>) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self.get_is_alpha()?;
        let c_i32 = self
            .builder
            .build_int_truncate(c.into_int_value(), self.context.i32_type(), "c32")
            .unwrap();
        let call_site = self
            .builder
            .build_call(fn_val, &[c_i32.into()], "is_alpha_call")
            .unwrap();
        let i32_val: inkwell::values::IntValue = call_site.as_any_value_enum().try_into().unwrap();
        Ok(self
            .builder
            .build_int_z_extend(i32_val, self.context.i64_type(), "alpha_ext")
            .unwrap()
            .into())
    }

    pub fn is_space(&self, c: BasicValueEnum<'ctx>) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self.get_is_space()?;
        let c_i32 = self
            .builder
            .build_int_truncate(c.into_int_value(), self.context.i32_type(), "c32")
            .unwrap();
        let call_site = self
            .builder
            .build_call(fn_val, &[c_i32.into()], "is_space_call")
            .unwrap();
        let i32_val: inkwell::values::IntValue = call_site.as_any_value_enum().try_into().unwrap();
        Ok(self
            .builder
            .build_int_z_extend(i32_val, self.context.i64_type(), "space_ext")
            .unwrap()
            .into())
    }

    fn get_printf(&self) -> Result<inkwell::values::FunctionValue<'ctx>, String> {
        self.module
            .get_function("printf")
            .ok_or("printf not found".to_string())
    }

    fn get_scanf(&self) -> Result<inkwell::values::FunctionValue<'ctx>, String> {
        self.module
            .get_function("scanf")
            .ok_or("scanf not found".to_string())
    }

    fn get_sprintf(&self) -> Result<inkwell::values::FunctionValue<'ctx>, String> {
        self.module
            .get_function("sprintf")
            .ok_or("sprintf not found".to_string())
    }

    fn get_strlen(&self) -> Result<inkwell::values::FunctionValue<'ctx>, String> {
        self.module
            .get_function("strlen")
            .ok_or("strlen not found".to_string())
    }

    fn get_atoi(&self) -> Result<inkwell::values::FunctionValue<'ctx>, String> {
        self.module
            .get_function("atoi")
            .ok_or("atoi not found".to_string())
    }

    pub fn print_string(&self, msg: &str) -> Result<(), String> {
        let fmt_ptr = self.build_format_global(msg, "print_str");
        let printf_fn = self.get_printf()?;
        self.builder
            .build_call(printf_fn, &[fmt_ptr.into()], "printf_call")
            .unwrap();
        Ok(())
    }

    pub fn println_string(&self, msg: &str) -> Result<(), String> {
        let fmt_ptr = self.build_format_global(&format!("{}\n", msg), "println_str");
        let printf_fn = self.get_printf()?;
        self.builder
            .build_call(printf_fn, &[fmt_ptr.into()], "printf_call")
            .unwrap();
        Ok(())
    }

    pub fn print_int(&self, value: BasicValueEnum<'ctx>) -> Result<(), String> {
        let fmt_ptr = self.build_format_global("%ld", "print_int");
        let printf_fn = self.get_printf()?;
        self.builder
            .build_call(printf_fn, &[fmt_ptr.into(), value.into()], "printf_call")
            .unwrap();
        Ok(())
    }

    pub fn println_int(&self, value: BasicValueEnum<'ctx>) -> Result<(), String> {
        let fmt_ptr = self.build_format_global("%ld\n", "println_int");
        let printf_fn = self.get_printf()?;
        self.builder
            .build_call(printf_fn, &[fmt_ptr.into(), value.into()], "printf_call")
            .unwrap();
        Ok(())
    }

    pub fn print_int_sp(&self, value: BasicValueEnum<'ctx>) -> Result<(), String> {
        let fmt_ptr = self.build_format_global("%ld ", "print_int_sp");
        let printf_fn = self.get_printf()?;
        self.builder
            .build_call(printf_fn, &[fmt_ptr.into(), value.into()], "printf_call")
            .unwrap();
        Ok(())
    }

    pub fn print_float(&self, value: BasicValueEnum<'ctx>) -> Result<(), String> {
        let fmt_ptr = self.build_format_global("%f", "print_float");
        let printf_fn = self.get_printf()?;
        self.builder
            .build_call(printf_fn, &[fmt_ptr.into(), value.into()], "printf_call")
            .unwrap();
        Ok(())
    }

    pub fn println_float(&self, value: BasicValueEnum<'ctx>) -> Result<(), String> {
        let fmt_ptr = self.build_format_global("%f\n", "println_float");
        let printf_fn = self.get_printf()?;
        self.builder
            .build_call(printf_fn, &[fmt_ptr.into(), value.into()], "printf_call")
            .unwrap();
        Ok(())
    }

    pub fn print_float_sp(&self, value: BasicValueEnum<'ctx>) -> Result<(), String> {
        let fmt_ptr = self.build_format_global("%f ", "print_float_sp");
        let printf_fn = self.get_printf()?;
        self.builder
            .build_call(printf_fn, &[fmt_ptr.into(), value.into()], "printf_call")
            .unwrap();
        Ok(())
    }

    pub fn print_string_val(&self, value: BasicValueEnum<'ctx>) -> Result<(), String> {
        let fmt_ptr = self.build_format_global("%s", "print_str_val");
        let printf_fn = self.get_printf()?;
        self.builder
            .build_call(printf_fn, &[fmt_ptr.into(), value.into()], "printf_call")
            .unwrap();
        Ok(())
    }

    pub fn print_string_val_sp(&self, value: BasicValueEnum<'ctx>) -> Result<(), String> {
        let fmt_ptr = self.build_format_global("%s ", "print_str_val_sp");
        let printf_fn = self.get_printf()?;
        self.builder
            .build_call(printf_fn, &[fmt_ptr.into(), value.into()], "printf_call")
            .unwrap();
        Ok(())
    }

    pub fn println_string_val(&self, value: BasicValueEnum<'ctx>) -> Result<(), String> {
        let fmt_ptr = self.build_format_global("%s\n", "println_str_val");
        let printf_fn = self.get_printf()?;
        self.builder
            .build_call(printf_fn, &[fmt_ptr.into(), value.into()], "printf_call")
            .unwrap();
        Ok(())
    }

    pub fn printf(&self, fmt: &str, args: &[BasicValueEnum<'ctx>]) -> Result<(), String> {
        let processed = fmt
            .replace("\\n", "\n")
            .replace("\\t", "\t")
            .replace("\\r", "\r");
        let fmt_ptr = self.build_format_global(&processed, "printf_fmt");
        let printf_fn = self.get_printf()?;
        let mut call_args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> =
            vec![fmt_ptr.into()];
        for arg in args {
            call_args.push((*arg).into());
        }
        self.builder
            .build_call(printf_fn, &call_args, "printf_call")
            .unwrap();
        Ok(())
    }

    pub fn scan_int(&self, ptr: inkwell::values::PointerValue<'ctx>) -> Result<(), String> {
        let fmt_ptr = self.build_format_global("%ld", "scan_int");
        let scanf_fn = self.get_scanf()?;
        self.builder
            .build_call(scanf_fn, &[fmt_ptr.into(), ptr.into()], "scanf_call")
            .unwrap();
        Ok(())
    }

    pub fn scan_string(&self, ptr: inkwell::values::PointerValue<'ctx>) -> Result<(), String> {
        let fmt_ptr = self.build_format_global("%255s", "scan_str");
        let scanf_fn = self.get_scanf()?;
        self.builder
            .build_call(scanf_fn, &[fmt_ptr.into(), ptr.into()], "scanf_call")
            .unwrap();
        Ok(())
    }

    pub fn scan_float(&self, ptr: inkwell::values::PointerValue<'ctx>) -> Result<(), String> {
        let fmt_ptr = self.build_format_global("%lf", "scan_float");
        let scanf_fn = self.get_scanf()?;
        self.builder
            .build_call(scanf_fn, &[fmt_ptr.into(), ptr.into()], "scanf_call")
            .unwrap();
        Ok(())
    }

    pub fn scanln_int(&self, ptr: inkwell::values::PointerValue<'ctx>) -> Result<(), String> {
        let fmt_ptr = self.build_format_global("%ld", "scanln_int");
        let scanf_fn = self.get_scanf()?;
        self.builder
            .build_call(scanf_fn, &[fmt_ptr.into(), ptr.into()], "scanf_call")
            .unwrap();
        Ok(())
    }

    pub fn scanln_float(&self, ptr: inkwell::values::PointerValue<'ctx>) -> Result<(), String> {
        let fmt_ptr = self.build_format_global("%lf", "scanln_float");
        let scanf_fn = self.get_scanf()?;
        self.builder
            .build_call(scanf_fn, &[fmt_ptr.into(), ptr.into()], "scanf_call")
            .unwrap();
        Ok(())
    }

    pub fn sprintf(
        &self,
        fmt: &str,
        args: &[BasicValueEnum<'ctx>],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let processed = fmt
            .replace("\\n", "\n")
            .replace("\\t", "\t")
            .replace("\\r", "\r");
        let fmt_ptr = self.build_format_global(&processed, "sprintf_fmt");
        let buf_type = self.context.i8_type().array_type(256);
        let buf = self.builder.build_alloca(buf_type, "sprintf_buf").unwrap();
        let sprintf_fn = self.get_sprintf()?;
        let mut call_args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> =
            vec![buf.into(), fmt_ptr.into()];
        for arg in args {
            call_args.push((*arg).into());
        }
        self.builder
            .build_call(sprintf_fn, &call_args, "sprintf_call")
            .unwrap();
        Ok(buf.into())
    }

    pub fn len_string(&self, value: BasicValueEnum<'ctx>) -> Result<BasicValueEnum<'ctx>, String> {
        let strlen_fn = self.get_strlen()?;
        let call_site = self
            .builder
            .build_call(strlen_fn, &[value.into()], "strlen_call")
            .unwrap();
        Ok(call_site.as_any_value_enum().try_into().unwrap())
    }

    pub fn atoi_string(&self, value: BasicValueEnum<'ctx>) -> Result<BasicValueEnum<'ctx>, String> {
        let atoi_fn = self.get_atoi()?;
        let call_site = self
            .builder
            .build_call(atoi_fn, &[value.into()], "atoi_call")
            .unwrap();
        let i32_val: inkwell::values::IntValue = call_site.as_any_value_enum().try_into().unwrap();
        Ok(self
            .builder
            .build_int_s_extend(i32_val, self.context.i64_type(), "atoi_i64")
            .unwrap()
            .into())
    }

    pub fn itoa_string(&self, value: BasicValueEnum<'ctx>) -> Result<BasicValueEnum<'ctx>, String> {
        let fmt_ptr = self.build_format_global("%ld", "itoa_fmt");
        let buf_type = self.context.i8_type().array_type(32);
        let buf = self.builder.build_alloca(buf_type, "itoa_buf").unwrap();
        let sprintf_fn = self.get_sprintf()?;
        self.builder
            .build_call(
                sprintf_fn,
                &[buf.into(), fmt_ptr.into(), value.into()],
                "sprintf_call",
            )
            .unwrap();
        Ok(buf.into())
    }

    fn i64_to_ptr(&self, handle: BasicValueEnum<'ctx>) -> inkwell::values::PointerValue<'ctx> {
        self.builder
            .build_int_to_ptr(
                handle.into_int_value(),
                self.context.ptr_type(inkwell::AddressSpace::default()),
                "handle_ptr",
            )
            .unwrap()
    }

    fn ptr_to_i64(&self, ptr: inkwell::values::PointerValue<'ctx>) -> BasicValueEnum<'ctx> {
        self.builder
            .build_ptr_to_int(ptr, self.context.i64_type(), "ptr_handle")
            .unwrap()
            .into()
    }

    pub fn dynarr_new(&self) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self
            .module
            .get_function("coconut_dynarr_new")
            .ok_or("coconut_dynarr_new not found".to_string())?;
        let call_site = self
            .builder
            .build_call(fn_val, &[], "dynarr_new_call")
            .unwrap();
        Ok(self.ptr_to_i64(call_site.as_any_value_enum().try_into().unwrap()))
    }

    pub fn dynarr_push(
        &self,
        handle: BasicValueEnum<'ctx>,
        val: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        let fn_val = self
            .module
            .get_function("coconut_dynarr_push")
            .ok_or("coconut_dynarr_push not found".to_string())?;
        self.builder
            .build_call(
                fn_val,
                &[self.i64_to_ptr(handle).into(), val.into()],
                "dynarr_push_call",
            )
            .unwrap();
        Ok(())
    }

    pub fn dynarr_get(
        &self,
        handle: BasicValueEnum<'ctx>,
        idx: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self
            .module
            .get_function("coconut_dynarr_get")
            .ok_or("coconut_dynarr_get not found".to_string())?;
        let idx_i32 = self
            .builder
            .build_int_truncate(idx.into_int_value(), self.context.i32_type(), "idx32")
            .unwrap();
        let call_site = self
            .builder
            .build_call(
                fn_val,
                &[self.i64_to_ptr(handle).into(), idx_i32.into()],
                "dynarr_get_call",
            )
            .unwrap();
        Ok(call_site.as_any_value_enum().try_into().unwrap())
    }

    pub fn dynarr_set(
        &self,
        handle: BasicValueEnum<'ctx>,
        idx: BasicValueEnum<'ctx>,
        val: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        let fn_val = self
            .module
            .get_function("coconut_dynarr_set")
            .ok_or("coconut_dynarr_set not found".to_string())?;
        let idx_i32 = self
            .builder
            .build_int_truncate(idx.into_int_value(), self.context.i32_type(), "idx32")
            .unwrap();
        self.builder
            .build_call(
                fn_val,
                &[self.i64_to_ptr(handle).into(), idx_i32.into(), val.into()],
                "dynarr_set_call",
            )
            .unwrap();
        Ok(())
    }

    pub fn dynarr_len(&self, handle: BasicValueEnum<'ctx>) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self
            .module
            .get_function("coconut_dynarr_len")
            .ok_or("coconut_dynarr_len not found".to_string())?;
        let call_site = self
            .builder
            .build_call(fn_val, &[self.i64_to_ptr(handle).into()], "dynarr_len_call")
            .unwrap();
        let i32_val: inkwell::values::IntValue = call_site.as_any_value_enum().try_into().unwrap();
        Ok(self
            .builder
            .build_int_s_extend(i32_val, self.context.i64_type(), "len_ext")
            .unwrap()
            .into())
    }

    pub fn strarr_new(&self) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self
            .module
            .get_function("coconut_strarr_new")
            .ok_or("coconut_strarr_new not found".to_string())?;
        let call_site = self
            .builder
            .build_call(fn_val, &[], "strarr_new_call")
            .unwrap();
        Ok(self.ptr_to_i64(call_site.as_any_value_enum().try_into().unwrap()))
    }

    pub fn strarr_push(
        &self,
        handle: BasicValueEnum<'ctx>,
        s: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        let fn_val = self
            .module
            .get_function("coconut_strarr_push")
            .ok_or("coconut_strarr_push not found".to_string())?;
        self.builder
            .build_call(
                fn_val,
                &[self.i64_to_ptr(handle).into(), s.into()],
                "strarr_push_call",
            )
            .unwrap();
        Ok(())
    }

    pub fn strarr_get(
        &self,
        handle: BasicValueEnum<'ctx>,
        idx: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self
            .module
            .get_function("coconut_strarr_get")
            .ok_or("coconut_strarr_get not found".to_string())?;
        let idx_i32 = self
            .builder
            .build_int_truncate(idx.into_int_value(), self.context.i32_type(), "idx32")
            .unwrap();
        let call_site = self
            .builder
            .build_call(
                fn_val,
                &[self.i64_to_ptr(handle).into(), idx_i32.into()],
                "strarr_get_call",
            )
            .unwrap();
        Ok(call_site.as_any_value_enum().try_into().unwrap())
    }

    pub fn strarr_len(&self, handle: BasicValueEnum<'ctx>) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self
            .module
            .get_function("coconut_strarr_len")
            .ok_or("coconut_strarr_len not found".to_string())?;
        let call_site = self
            .builder
            .build_call(fn_val, &[self.i64_to_ptr(handle).into()], "strarr_len_call")
            .unwrap();
        let i32_val: inkwell::values::IntValue = call_site.as_any_value_enum().try_into().unwrap();
        Ok(self
            .builder
            .build_int_s_extend(i32_val, self.context.i64_type(), "len_ext")
            .unwrap()
            .into())
    }

    pub fn hashmap_new(&self) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self
            .module
            .get_function("coconut_hashmap_new")
            .ok_or("coconut_hashmap_new not found".to_string())?;
        let call_site = self.builder.build_call(fn_val, &[], "hm_new_call").unwrap();
        Ok(self.ptr_to_i64(call_site.as_any_value_enum().try_into().unwrap()))
    }

    pub fn hashmap_set(
        &self,
        handle: BasicValueEnum<'ctx>,
        key: BasicValueEnum<'ctx>,
        val: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        let fn_val = self
            .module
            .get_function("coconut_hashmap_set")
            .ok_or("coconut_hashmap_set not found".to_string())?;
        self.builder
            .build_call(
                fn_val,
                &[self.i64_to_ptr(handle).into(), key.into(), val.into()],
                "hm_set_call",
            )
            .unwrap();
        Ok(())
    }

    pub fn hashmap_get(
        &self,
        handle: BasicValueEnum<'ctx>,
        key: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self
            .module
            .get_function("coconut_hashmap_get")
            .ok_or("coconut_hashmap_get not found".to_string())?;
        let call_site = self
            .builder
            .build_call(
                fn_val,
                &[self.i64_to_ptr(handle).into(), key.into()],
                "hm_get_call",
            )
            .unwrap();
        Ok(call_site.as_any_value_enum().try_into().unwrap())
    }

    pub fn hashmap_has(
        &self,
        handle: BasicValueEnum<'ctx>,
        key: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self
            .module
            .get_function("coconut_hashmap_has")
            .ok_or("coconut_hashmap_has not found".to_string())?;
        let call_site = self
            .builder
            .build_call(
                fn_val,
                &[self.i64_to_ptr(handle).into(), key.into()],
                "hm_has_call",
            )
            .unwrap();
        let i32_val: inkwell::values::IntValue = call_site.as_any_value_enum().try_into().unwrap();
        Ok(self
            .builder
            .build_int_z_extend(i32_val, self.context.i64_type(), "hm_has_ext")
            .unwrap()
            .into())
    }

    pub fn hashmap_del(
        &self,
        handle: BasicValueEnum<'ctx>,
        key: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        let fn_val = self
            .module
            .get_function("coconut_hashmap_del")
            .ok_or("coconut_hashmap_del not found".to_string())?;
        self.builder
            .build_call(
                fn_val,
                &[self.i64_to_ptr(handle).into(), key.into()],
                "hm_del_call",
            )
            .unwrap();
        Ok(())
    }

    pub fn hashmap_len(
        &self,
        handle: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self
            .module
            .get_function("coconut_hashmap_len")
            .ok_or("coconut_hashmap_len not found".to_string())?;
        let call_site = self
            .builder
            .build_call(fn_val, &[self.i64_to_ptr(handle).into()], "hm_len_call")
            .unwrap();
        let i32_val: inkwell::values::IntValue = call_site.as_any_value_enum().try_into().unwrap();
        Ok(self
            .builder
            .build_int_s_extend(i32_val, self.context.i64_type(), "hm_len_ext")
            .unwrap()
            .into())
    }

    pub fn hashmap_keys(
        &self,
        handle: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self
            .module
            .get_function("coconut_hashmap_keys")
            .ok_or("coconut_hashmap_keys not found".to_string())?;
        let call_site = self
            .builder
            .build_call(fn_val, &[self.i64_to_ptr(handle).into()], "hm_keys_call")
            .unwrap();
        Ok(self.ptr_to_i64(call_site.as_any_value_enum().try_into().unwrap()))
    }

    pub fn hashmap_clear(&self, handle: BasicValueEnum<'ctx>) -> Result<(), String> {
        let fn_val = self
            .module
            .get_function("coconut_hashmap_clear")
            .ok_or("coconut_hashmap_clear not found".to_string())?;
        self.builder
            .build_call(fn_val, &[self.i64_to_ptr(handle).into()], "hm_clear_call")
            .unwrap();
        Ok(())
    }

    pub fn split(
        &self,
        str: BasicValueEnum<'ctx>,
        delim: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self
            .module
            .get_function("coconut_split")
            .ok_or("coconut_split not found".to_string())?;
        let call_site = self
            .builder
            .build_call(fn_val, &[str.into(), delim.into()], "split_call")
            .unwrap();
        Ok(self.ptr_to_i64(call_site.as_any_value_enum().try_into().unwrap()))
    }

    pub fn join(
        &self,
        arr: BasicValueEnum<'ctx>,
        delim: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = self
            .module
            .get_function("coconut_join")
            .ok_or("coconut_join not found".to_string())?;
        let call_site = self
            .builder
            .build_call(
                fn_val,
                &[self.i64_to_ptr(arr).into(), delim.into()],
                "join_call",
            )
            .unwrap();
        Ok(call_site.as_any_value_enum().try_into().unwrap())
    }

    pub fn format(
        &self,
        fmt: &str,
        args: &[BasicValueEnum<'ctx>],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let processed = fmt
            .replace("\\n", "\n")
            .replace("\\t", "\t")
            .replace("\\r", "\r");
        let fmt_ptr = self.build_format_global(&processed, "format_fmt");
        let fn_val = self
            .module
            .get_function("coconut_format")
            .ok_or("coconut_format not found".to_string())?;

        let mut call_args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> =
            vec![fmt_ptr.into()];
        for arg in args {
            call_args.push((*arg).into());
        }

        let call_site = self
            .builder
            .build_call(fn_val, &call_args, "format_call")
            .unwrap();
        Ok(call_site.as_any_value_enum().try_into().unwrap())
    }
}
