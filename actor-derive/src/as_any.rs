use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_str;

use crate::with_crate;

pub fn expand(ast: syn::DeriveInput) -> TokenStream {
    let ident = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let as_any_path = with_crate(parse_str("ext::as_any::AsAny").unwrap());
    quote! {
        impl #impl_generics #as_any_path for #ident #ty_generics #where_clause {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }

            fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
                self
            }
        }
    }
}