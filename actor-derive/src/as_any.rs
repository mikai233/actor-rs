use proc_macro2::TokenStream;
use quote::quote;

use crate::{with_crate_str, CRATE_ACTOR_CORE};

pub fn expand(input: &syn::DeriveInput) -> syn::Result<TokenStream> {
    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let as_any_path = with_crate_str(CRATE_ACTOR_CORE, "ext::as_any::AsAny")?;
    let stream = quote! {
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
    };
    Ok(stream)
}