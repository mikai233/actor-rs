use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{ImplGenerics, parse_str, TypeGenerics, WhereClause};

use crate::metadata::{CodecType, MessageImpl};
use crate::with_crate;

pub fn expand(
    ast: syn::DeriveInput,
    message_impl: MessageImpl,
    codec_type: CodecType,
    cloneable: bool,
) -> TokenStream {
    let message_ty = ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let codec_trait = with_crate(parse_str("CodecMessage").unwrap());
    let decoder_trait = with_crate(parse_str("message::MessageDecoder").unwrap());
    let ext_path = with_crate(parse_str("ext").unwrap());
    let dy_message = with_crate(parse_str("DynMessage").unwrap());
    let reg = with_crate(parse_str("message::message_registration::MessageRegistration").unwrap());
    let decoder = expand_decoder(&message_ty, &message_impl, &codec_type, &ext_path, &dy_message, &reg);
    let encode = expand_encode(&codec_type, &ext_path);
    let dyn_clone = expand_dyn_clone(&message_ty, &ty_generics, &dy_message, &message_impl, cloneable);
    let codec_impl = quote! {
        impl #impl_generics #codec_trait for #message_ty #ty_generics #where_clause {
            fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
                self
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn decoder() -> Option<Box<dyn #decoder_trait>> where Self: Sized {
                #decoder
            }

            fn encode(&self, _reg: &#reg) -> Result<Vec<u8>, bincode::error::EncodeError> {
                #encode
            }

            fn dyn_clone(&self) -> anyhow::Result<#dy_message> {
                #dyn_clone
            }

            fn is_cloneable(&self) -> bool {
                #cloneable
            }
        }
    };
    let impl_message = if matches!(message_impl, MessageImpl::OrphanMessage) {
        let message_trait = with_crate(parse_str("OrphanMessage").unwrap());
        Some(expand_message_impl(&message_ty, message_trait, &impl_generics, &ty_generics, where_clause))
    } else {
        None
    };
    match impl_message {
        None => {
            quote! {
                #codec_impl
            }
        }
        Some(impl_message) => {
            quote! {
                #codec_impl
                #impl_message
            }
        }
    }
}

pub(crate) fn expand_decoder(
    message_ty: &Ident,
    message_impl: &MessageImpl,
    codec_type: &CodecType,
    ext_path: &TokenStream,
    dy_message: &TokenStream,
    reg: &TokenStream,
) -> TokenStream {
    let decoder_trait = with_crate(parse_str("message::MessageDecoder").unwrap());
    match codec_type {
        CodecType::NoneSerde => {
            quote!(None)
        }
        CodecType::Serde => {
            match message_impl {
                MessageImpl::Message => {
                    decoder(&decoder_trait, &dy_message, &reg, || {
                        quote! {
                            let message: #message_ty = #ext_path::decode_bytes(bytes)?;
                            let message = #dy_message::user(message);
                            Ok(message)
                        }
                    })
                }
                MessageImpl::SystemMessage => {
                    decoder(&decoder_trait, &dy_message, &reg, || {
                        quote! {
                            let message: #message_ty = #ext_path::decode_bytes(bytes)?;
                            let message = #dy_message::system(message);
                            Ok(message)
                        }
                    })
                }
                MessageImpl::OrphanMessage => {
                    decoder(&decoder_trait, &dy_message, &reg, || {
                        quote! {
                            let message: #message_ty = #ext_path::decode_bytes(bytes)?;
                            let message = #dy_message::orphan(message);
                            Ok(message)
                        }
                    })
                }
            }
        }
    }
}

pub(crate) fn decoder<F>(
    decoder_trait: &TokenStream,
    dy_message: &TokenStream,
    reg: &TokenStream,
    fn_body: F,
) -> TokenStream where F: FnOnce() -> TokenStream {
    let body = fn_body();
    quote! {
        #[derive(Clone)]
        struct D;
        impl #decoder_trait for D {
            fn decode(&self, bytes: &[u8], _reg: &#reg) -> Result<#dy_message, bincode::error::DecodeError> {
                #body
            }
        }
        Some(Box::new(D))
    }
}

pub(crate) fn expand_encode(codec_type: &CodecType, ext_path: &TokenStream) -> TokenStream {
    match codec_type {
        CodecType::NoneSerde => {
            quote! {
                Err(bincode::error::EncodeError::Other("this type cannot encode"))
            }
        }
        _ => {
            quote! {
                #ext_path::encode_bytes(self)
            }
        }
    }
}

pub(crate) fn expand_dyn_clone(
    message_ty: &Ident,
    ty_generics: &TypeGenerics,
    dy_message: &TokenStream,
    message_impl: &MessageImpl,
    cloneable: bool,
) -> TokenStream {
    if !cloneable {
        quote! {
            Err(anyhow::anyhow!("message {} is not cloneable", std::any::type_name::<#message_ty #ty_generics>()))
        }
    } else {
        match message_impl {
            MessageImpl::Message => {
                quote! {
                    let message = #dy_message::user(Clone::clone(self));
                    Ok(message)
                }
            }
            MessageImpl::SystemMessage => {
                quote! {
                    let message = #dy_message::system(Clone::clone(self));
                    Ok(message)
                }
            }
            MessageImpl::OrphanMessage => {
                quote! {
                    let message = #dy_message::orphan(Clone::clone(self));
                    Ok(message)
                }
            }
        }
    }
}

pub(crate) fn expand_message_impl(
    message_ty: &Ident,
    message_trait: TokenStream,
    impl_generics: &ImplGenerics,
    ty_generics: &TypeGenerics,
    where_clause: Option<&WhereClause>) -> TokenStream {
    quote! {
        impl #impl_generics #message_trait for #message_ty #ty_generics #where_clause {

        }
    }
}