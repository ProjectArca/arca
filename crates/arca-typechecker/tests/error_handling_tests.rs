use arca_typechecker::types::{PrimitiveType, Type};

#[test]
fn test_error_union_type_representation() {
    let io_err = Type::Primitive(PrimitiveType::I32);
    let parse_err = Type::Primitive(PrimitiveType::I64);

    let union_type = Type::ErrorUnion(vec![io_err.clone(), parse_err.clone()]);

    if let Type::ErrorUnion(variants) = union_type {
        assert_eq!(variants.len(), 2);
        assert_eq!(variants[0], io_err);
        assert_eq!(variants[1], parse_err);
    } else {
        panic!("Expected Type::ErrorUnion");
    }
}

#[test]
fn test_result_and_option_types() {
    let ok_type = Type::Primitive(PrimitiveType::String);
    let err_type = Type::Primitive(PrimitiveType::I32);

    let res_type = Type::Result(Box::new(ok_type.clone()), Box::new(err_type.clone()));
    let opt_type = Type::Option(Box::new(ok_type.clone()));

    if let Type::Result(ok, err) = res_type {
        assert_eq!(*ok, ok_type);
        assert_eq!(*err, err_type);
    } else {
        panic!("Expected Type::Result");
    }

    if let Type::Option(inner) = opt_type {
        assert_eq!(*inner, ok_type);
    } else {
        panic!("Expected Type::Option");
    }
}
