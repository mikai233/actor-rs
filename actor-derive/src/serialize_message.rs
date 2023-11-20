use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, parse_str};

use crate::with_crate;

pub fn expand_user(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let codec = with_crate(parse_str("actor::CodecMessage").unwrap());
    let decoder = with_crate(parse_str("decoder::MessageDecoder").unwrap());
    quote! {
        impl #impl_generics #codec for #name #ty_generics #where_clause {
            fn into_any(self: Box<Self>) -> Box<dyn Any> {
                self
            }

            fn decoder() -> Option<Box<dyn crate::decoder::MessageDecoder>> where Self: Sized {
                Some(actor::user_message_decoder!(Ping, TestActor))
            }

            fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
                None
            }
        }
    }
}

pub fn expand_system(ast: &syn::DeriveInput) -> TokenStream {
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