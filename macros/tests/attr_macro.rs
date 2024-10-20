// tests/attribute_macro.rs

#[allow(unused)]
use loco_macros::*;

// macro converts struct S to struct H

#[test]
//#[test_request]
fn test_macro() {
    #[allow(clippy::assertions_on_constants)]
    {
        assert!(true);
    }
}
