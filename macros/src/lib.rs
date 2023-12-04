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
pub fn test_macro(_metadata: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemFn);
    let block = input.block;

    let modified_body = quote! {
        {
            let mut settings = insta::Settings::clone_current();
            settings.set_prepend_module_to_snapshot(false);
            let _guard = settings.bind_to_scope();
            testing::request::<App, Migrator, _, _>(|request, ctx| async move {

                #block

            }).await;
        }
    };
    input.block = syn::parse(modified_body.into()).expect("Failed to parse modified body");

    quote! {
        #input
    }
    .into()
}

#[proc_macro_attribute]
pub fn test_request(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as ItemFn);

    // Check if the function has the expected signature

    // Extract the original function name
    let original_function_name = &input.sig.ident;

    // Preserve the original function attributes in the wrapper function
    let original_attributes = &input.attrs;
    let mut wrapper_attributes = Vec::new();
    for attr in original_attributes {
        wrapper_attributes.push(attr.clone());
    }
    let block = input.block;

    //#original_function_name( &request, &ctx).await;
    // Generate the expanded code using the quote! macro
    let expanded = quote! {

        #[tokio::test]
        #[serial]
        #(#wrapper_attributes)*
        async fn #original_function_name() {
            let mut settings = insta::Settings::clone_current();
            settings.set_prepend_module_to_snapshot(false);
            let _guard = settings.bind_to_scope();
            testing::request::<App, Migrator, _, _>(|request, ctx| async move {
                #block
            }).await;
        }
    };

    println!("{}", expanded);
    // Convert the expanded code back into a TokenStream and return it
    expanded.into()
}
