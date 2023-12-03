// lib.rs
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/*

let mut settings = insta::Settings::clone_current();
settings.set_prepend_module_to_snapshot(false);
settings.set_snapshot_suffix("auth_request");
let _guard = settings.bind_to_scope();

*/
#[proc_macro_attribute]
pub fn loco_test(_metadata: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemFn);
    let block = input.block;

    let modified_body = quote! {
        {
            let mut settings = insta::Settings::clone_current();
            settings.set_prepend_module_to_snapshot(false);
            let _guard = settings.bind_to_scope();
            #block
        }
    };
    input.block = syn::parse(modified_body.into()).expect("Failed to parse modified body");

    quote! {
        #input
    }
    .into()
}
