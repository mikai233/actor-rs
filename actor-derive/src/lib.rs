use proc_macro::TokenStream;

use proc_macro2::{Ident, Span};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::{DeriveInput, parse_str, Token};
use syn::__private::TokenStream2;
use syn::punctuated::Punctuated;

use crate::metadata::{CodecMeta, MessageType};

mod message;
mod serialize_message;
mod metadata;

#[proc_macro_derive(EmptyCodec)]
pub fn empty_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(&ast).into()
}

#[proc_macro_derive(MessageCodec, attributes(message))]
pub fn message_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let mut message_type = MessageType::NoneSerde;
    let mut actor_name = "".to_string();
    let codec_meta = ast.attrs.into_iter()
        .filter(|attr| attr.path.is_ident("message"))
        .try_fold(Vec::new(), |mut vec, attr| {
            vec.extend(attr.parse_args_with(Punctuated::<CodecMeta, Token![,]>::parse_terminated)?);
            Ok::<Vec<CodecMeta>, syn::Error>(vec)
        }).unwrap();
    for meta in codec_meta {
        match meta {
            CodecMeta::Type(ty) => {
                message_type = ty;
            }
            CodecMeta::Actor(ty) => {
                actor_name = ty.name.value();
            }
        }
    }
    let message_ty = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let codec_trait = with_crate(parse_str("actor::CodecMessage").unwrap());
    let decoder_trait = with_crate(parse_str("decoder::MessageDecoder").unwrap());
    let decoder = gen_decoder(message_ty, &message_type, &actor_name);
    let encode = gen_encode(&message_type);
    let s = quote! {
        impl #impl_generics #codec_trait for #message_ty #ty_generics #where_clause {
            fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
                self
            }

            fn decoder() -> Option<Box<dyn #decoder_trait >> where Self: Sized {
                #decoder
            }

            fn encode(&self) -> Option<anyhow::Result<Vec<u8>>> {
                #encode
            }
        }
    };
    s.into()
}

fn gen_decoder(message_ty: &Ident, message_type: &MessageType, actor_name: &String) -> TokenStream2 {
    let actor_ident = Ident::new(actor_name, Span::call_site());
    let decoder_trait = with_crate(parse_str("decoder::MessageDecoder").unwrap());
    let dy_message = with_crate(parse_str("actor::DynamicMessage").unwrap());
    let ext_path = with_crate(parse_str("ext").unwrap());
    match message_type {
        MessageType::NoneSerde => {
            quote! {
                None
            }
        }
        MessageType::Serde => {
            let user_delegate = with_crate(parse_str("delegate::user::UserDelegate").unwrap());
            quote! {
                struct D;
                impl #decoder_trait for D {
                    fn decode(&self, bytes: &[u8]) -> anyhow::Result<#dy_message> {
                        let message: #message_ty = #ext_path::decode_bytes(bytes)?;
                        let message = #user_delegate::<#actor_ident>::new(message);
                        Ok(message.into())
                    }
                }
                Some(Box::new(D))
            }
        }
        MessageType::AsyncSerde => {
            let async_delegate = with_crate(parse_str("delegate::user::AsyncUserDelegate").unwrap());
            quote! {
                struct D;
                impl #decoder_trait for D {
                    fn decode(&self, bytes: &[u8]) -> anyhow::Result<#dy_message> {
                        let message: #message_ty = #ext_path::decode_bytes(bytes)?;
                        let message = #async_delegate::<#actor_ident>::new(message);
                        Ok(message.into())
                    }
                }
                Some(Box::new(D))
            }
        }
        MessageType::SystemSerde => {
            let system_delegate = with_crate(parse_str("delegate::system::SystemDelegate").unwrap());
            quote! {
                struct D;
                impl #decoder_trait for D {
                    fn decode(&self, bytes: &[u8]) -> anyhow::Result<#dy_message> {
                        let message: #message_ty = #ext_path::decode_bytes(bytes)?;
                        let message = #system_delegate::<#actor_ident>::new(message);
                        Ok(message.into())
                    }
                }
                Some(Box::new(D))
            }
        }
    }
}

fn gen_encode(message_type: &MessageType) -> TokenStream2 {
    let ext_path = with_crate(parse_str("ext").unwrap());
    match message_type {
        MessageType::NoneSerde => {
            quote! {
                None
            }
        }
        _ => {
            quote! {
                Some(#ext_path::encode_bytes(self))
            }
        }
    }
}

#[proc_macro_derive(SystemCodec)]
pub fn system_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    todo!()
}

pub(crate) fn with_crate(path: syn::Path) -> proc_macro2::TokenStream {
    let found_crate = crate_name("actor").expect("actor is present in `Cargo.toml`");
    match found_crate {
        FoundCrate::Itself => quote!(crate::#path),
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!(#ident::#path)
        }
    }
}