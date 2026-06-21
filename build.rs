fn main() {
    // LLD C++ 包装器（统一放在 runtime/）
    println!("cargo:rustc-link-search=native=/home/xiaoq/Projects/Coconut/runtime");
    println!("cargo:rustc-link-lib=static=lld_wrapper");
    // whole-archive 强制链接所有 C runtime 符号（JIT 需要解析这些函数）
    println!("cargo:rustc-link-lib=static:+whole-archive=str_helpers");
    // 系统 LLVM 22 静态库
    println!("cargo:rustc-link-search=native=/home/xiaoq/LLVM-22.1.0-Linux-X64/lib");
    println!("cargo:rustc-link-lib=static=LLVMCore");
    println!("cargo:rustc-link-lib=static=LLVMX86CodeGen");
    println!("cargo:rustc-link-lib=static=LLVMX86Desc");
    println!("cargo:rustc-link-lib=static=LLVMX86Info");
    println!("cargo:rustc-link-lib=static=LLVMX86AsmParser");
    println!("cargo:rustc-link-lib=static=LLVMX86Disassembler");
    println!("cargo:rustc-link-lib=static=LLVMCodeGen");
    println!("cargo:rustc-link-lib=static=LLVMTarget");
    println!("cargo:rustc-link-lib=static=LLVMAsmPrinter");
    println!("cargo:rustc-link-lib=static=LLVMMC");
    println!("cargo:rustc-link-lib=static=LLVMMCParser");
    println!("cargo:rustc-link-lib=static=LLVMSelectionDAG");
    println!("cargo:rustc-link-lib=static=LLVMGlobalISel");
    println!("cargo:rustc-link-lib=static=LLVMBitWriter");
    println!("cargo:rustc-link-lib=static=LLVMBitReader");
    println!("cargo:rustc-link-lib=static=LLVMIRReader");
    println!("cargo:rustc-link-lib=static=LLVMAsmParser");
    println!("cargo:rustc-link-lib=static=LLVMAnalysis");
    println!("cargo:rustc-link-lib=static=LLVMTransformUtils");
    println!("cargo:rustc-link-lib=static=LLVMScalarOpts");
    println!("cargo:rustc-link-lib=static=LLVMInstCombine");
    println!("cargo:rustc-link-lib=static=LLVMAggressiveInstCombine");
    println!("cargo:rustc-link-lib=static=LLVMipo");
    println!("cargo:rustc-link-lib=static=LLVMVectorize");
    println!("cargo:rustc-link-lib=static=LLVMPasses");
    println!("cargo:rustc-link-lib=static=LLVMSupport");
    println!("cargo:rustc-link-lib=static=LLVMBinaryFormat");
    println!("cargo:rustc-link-lib=static=LLVMTargetParser");
    println!("cargo:rustc-link-lib=static=LLVMExecutionEngine");
    println!("cargo:rustc-link-lib=static=LLVMMCJIT");
    println!("cargo:rustc-link-lib=static=LLVMOrcJIT");
    println!("cargo:rustc-link-lib=static=LLVMOrcShared");
    println!("cargo:rustc-link-lib=static=LLVMOrcTargetProcess");
    println!("cargo:rustc-link-lib=static=LLVMRuntimeDyld");
    println!("cargo:rustc-link-lib=static=LLVMJITLink");
    println!("cargo:rustc-link-lib=static=LLVMObject");
    println!("cargo:rustc-link-lib=static=LLVMDemangle");
    println!("cargo:rustc-link-lib=static=LLVMRemarks");
    println!("cargo:rustc-link-lib=static=LLVMProfileData");
    println!("cargo:rustc-link-lib=static=LLVMDebugInfoDWARF");
    println!("cargo:rustc-link-lib=static=LLVMTextAPI");
    println!("cargo:rustc-link-lib=static=LLVMCoroutines");
    println!("cargo:rustc-link-lib=static=LLVMCFGuard");
    println!("cargo:rustc-link-lib=static=LLVMCodeGenTypes");
    println!("cargo:rustc-link-lib=static=LLVMLinker");

    // LLD 链接器库
    println!("cargo:rustc-link-lib=static=lldELF");
    println!("cargo:rustc-link-lib=static=lldCommon");

    // 系统依赖
    println!("cargo:rustc-link-lib=z");
    println!("cargo:rustc-link-lib=stdc++");
    println!("cargo:rustc-link-lib=pthread");
    println!("cargo:rustc-link-lib=dl");
    println!("cargo:rustc-link-lib=m");

    // 导出所有符号，使 JIT 能解析 C runtime 函数
    println!("cargo:rustc-link-arg=-rdynamic");
}
