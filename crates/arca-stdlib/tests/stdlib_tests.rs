use arca_stdlib::StdLibResolver;

#[test]
fn test_stdlib_rust_resolver() {
    let resolver = StdLibResolver::new();

    assert!(resolver.is_stdlib_module("core"));
    assert!(resolver.is_stdlib_module("std/fs"));
    assert!(resolver.is_stdlib_module("std/net"));
    assert!(resolver.is_stdlib_module("std/crypto"));

    let fs_syms = resolver.get_module_symbols("std/fs").unwrap();
    assert_eq!(fs_syms[0].name, "File");
}
