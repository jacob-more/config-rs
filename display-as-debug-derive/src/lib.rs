use quote::quote;
use syn::DeriveInput;

/// Same as the `Debug` derive macro, but uses the type's `Display`
/// implementation for the `Debug` implementation.
#[proc_macro_derive(
    DisplayAsDebug,
    attributes(key, group_key, default, lazy_lock, exhaustive, parse)
)]
pub fn config(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);
    derive_debug(ast).into()
}

fn derive_debug(ast: DeriveInput) -> proc_macro2::TokenStream {
    let (impl_generics, type_generics, where_clause) = ast.generics.split_for_impl();
    let ident = &ast.ident;

    quote! {
        impl #impl_generics ::std::fmt::Debug for #ident #type_generics #where_clause {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                ::core::write!(f, "{self}")
            }
        }
    }
}
