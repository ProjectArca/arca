use arca_air::AirModule;
use arca_backend::{BackendKind, CodeGenerator, TargetArch};

#[test]
fn test_c_backend_generation() {
    let module = AirModule::new("test_mod".into());
    let gen = CodeGenerator::new(BackendKind::C, TargetArch::C);

    let c_code = gen.generate_c(&module);
    assert!(c_code.contains("#include <stdio.h>"));
    assert!(c_code.contains("Hello from Arca Portable C Code Generator"));
}

#[test]
fn test_llvm_backend_generation() {
    let module = AirModule::new("test_mod".into());
    let gen = CodeGenerator::new(BackendKind::Llvm, TargetArch::Arm64);

    let llvm_ir = gen.generate_llvm(&module);
    assert!(llvm_ir.contains("target triple = \"aarch64-apple-darwin\""));
}

#[test]
fn test_native_anb_object_emission() {
    let module = AirModule::new("test_mod".into());
    let gen = CodeGenerator::new(BackendKind::Native, TargetArch::Arm64);

    let obj_bytes = gen.generate_native_object(&module);
    assert_eq!(&obj_bytes[0..4], &[0xCF, 0xFA, 0xED, 0xFE]);
}
