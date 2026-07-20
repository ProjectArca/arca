use arca_capabilities::{CapabilityDef, CapabilityRegistry, ImplBlock};
use arca_typechecker::{FnType, PrimitiveType, Type, TypeEnv};
use std::collections::HashMap;

#[test]
fn test_capability_impl_and_vtable_generation() {
    let mut registry = CapabilityRegistry::new();
    let env = TypeEnv::new();

    let mut reader_methods = HashMap::new();
    reader_methods.insert(
        "read".to_string(),
        FnType {
            params: vec![Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I32)),
        },
    );

    registry.register_capability(CapabilityDef {
        name: "Reader".to_string(),
        methods: reader_methods.clone(),
    });

    let impl_block = ImplBlock {
        capability_name: "Reader".to_string(),
        target_type: "File".to_string(),
        methods: reader_methods,
    };

    let vtable_res = registry.register_impl(impl_block, &env);
    assert!(vtable_res.is_ok());

    let vtable = vtable_res.unwrap();
    assert_eq!(vtable.capability_name, "Reader");
    assert_eq!(vtable.target_type, "File");
    assert_eq!(vtable.slots.len(), 1);
    assert_eq!(vtable.slots[0].method_name, "read");
    assert_eq!(vtable.slots[0].index, 0);
}

#[test]
fn test_missing_capability_method_diagnostic() {
    let mut registry = CapabilityRegistry::new();
    let env = TypeEnv::new();

    let mut reader_methods = HashMap::new();
    reader_methods.insert(
        "read".to_string(),
        FnType {
            params: vec![Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I32)),
        },
    );

    registry.register_capability(CapabilityDef {
        name: "Reader".to_string(),
        methods: reader_methods,
    });

    let incomplete_impl = ImplBlock {
        capability_name: "Reader".to_string(),
        target_type: "File".to_string(),
        methods: HashMap::new(), // Missing read()
    };

    let res = registry.register_impl(incomplete_impl, &env);
    assert!(res.is_err());

    let diags = res.unwrap_err();
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("Missing method 'read' required by capability 'Reader'"));
}
