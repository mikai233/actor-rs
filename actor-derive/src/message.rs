use proc_macro2::TokenStream;
use quote::quote;

pub fn expand(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    quote! {
        impl #impl_generics crate::actor::CodecMessage for #name #ty_generics #where_clause {
            fn into_any(self: Box<Self>) -> Box<dyn Any> {
                self
            }

            fn decoder() -> Option<Box<dyn crate::decoder::MessageDecoder>> where Self: Sized {
                None
            }

            fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
                None
            }
        }
    }
}