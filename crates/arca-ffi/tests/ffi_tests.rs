use arca_ffi::{CPrimitive, CStructLayout, FfiResolver};

#[test]
fn test_c_primitive_sizes() {
    assert_eq!(CPrimitive::Char.size_and_align(), (1, 1));
    assert_eq!(CPrimitive::Int.size_and_align(), (4, 4));
    assert_eq!(CPrimitive::Double.size_and_align(), (8, 8));
    assert_eq!(CPrimitive::VoidPtr.size_and_align(), (8, 8));
}

#[test]
fn test_c_struct_alignment_and_padding() {
    // struct Point { char id; int x; int y; };
    // id at 0 (size 1, pad 3) -> x at 4 (size 4) -> y at 8 (size 4) -> total size 12
    let layout = CStructLayout::compute(
        "Point",
        vec![
            ("id", CPrimitive::Char),
            ("x", CPrimitive::Int),
            ("y", CPrimitive::Int),
        ],
    );

    assert_eq!(layout.alignment, 4);
    assert_eq!(layout.total_size, 12);
    assert_eq!(layout.fields[0].offset, 0);
    assert_eq!(layout.fields[1].offset, 4);
    assert_eq!(layout.fields[2].offset, 8);
}

#[test]
fn test_ffi_resolver_stdlib_c() {
    let resolver = FfiResolver::new();
    assert!(resolver.extern_fns.contains_key("malloc"));
    assert!(resolver.extern_fns.contains_key("free"));
    assert!(resolver.extern_fns.contains_key("printf"));
}
