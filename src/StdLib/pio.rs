use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::builder::Builder;
use inkwell::values::BasicValueEnum;

pub fn init_pio_library<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
    let printf_type = context.i32_type().fn_type(&[context.ptr_type(inkwell::AddressSpace::default()).into()], true);
    module.add_function("printf", printf_type, None);
    let scanf_type = context.i32_type().fn_type(&[context.ptr_type(inkwell::AddressSpace::default()).into()], true);
    module.add_function("scanf", scanf_type, None);
}
pub fn generate_println<'ctx>(
    context: &'ctx Context,
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    message: &str,
) -> Result<(), String> {
    let format_str = format!("{}\n\0", message);
    let format_type = context.i8_type().array_type(format_str.len() as u32);
    let global_format = module.add_global(format_type, Some(inkwell::AddressSpace::default()), "println_fmt");
    global_format.set_initializer(&context.const_string(format_str.as_bytes(), false));
    let printf_fn = module.get_function("printf")
        .ok_or("printf function not found")?;
    let format_ptr = builder.build_pointer_cast(
        global_format.as_pointer_value(),
        context.ptr_type(inkwell::AddressSpace::default()),
        "fmt_ptr"
    ).unwrap();
    builder.build_call(
        printf_fn,
        &[format_ptr.into()],
        "printf_call"
    ).unwrap();
    Ok(())
}
pub fn generate_print_int<'ctx>(
    context: &'ctx Context,
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    value: BasicValueEnum<'ctx>,
) -> Result<(), String> {
    let format_str = "%d\n\0";
    let format_type = context.i8_type().array_type(format_str.len() as u32);
    let global_format = module.add_global(format_type, Some(inkwell::AddressSpace::default()), "print_int_fmt");
    global_format.set_initializer(&context.const_string(format_str.as_bytes(), false));
    let printf_fn = module.get_function("printf")
        .ok_or("printf function not found")?;
    let format_ptr = builder.build_pointer_cast(
        global_format.as_pointer_value(),
        context.ptr_type(inkwell::AddressSpace::default()),
        "fmt_ptr"
    ).unwrap();
    builder.build_call(
        printf_fn,
        &[format_ptr.into(), value.into()],
        "printf_call"
    ).unwrap();
    Ok(())
}
