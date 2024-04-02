use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{ImplGenerics, TypeGenerics, WhereClause};

use crate::metadata::{CodecType, MessageImpl};
use crate::with_crate_str;

pub fn expand(
    ast: syn::DeriveInput,
    message_impl: MessageImpl,
    codec_type: CodecType,
    cloneable: bool,
) -> TokenStream {
    let message_ty = ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let codec_trait = with_crate_str("CodecMessage");
    let decoder_trait = with_crate_str("message::MessageDecoder");
    let ext_path = with_crate_str("ext");
    let dy_message = with_crate_str("DynMessage");
    let reg = with_crate_str("message::message_registration::MessageRegistration");
    let eyre_result = with_crate_str("eyre::Result");
    let eyre = with_crate_str("eyre::eyre");
    let decoder = expand_decoder(
        &message_ty,
        &message_impl,
        &codec_type,
        &ext_path,
        &dy_message,
        &reg,
        &eyre_result,
    );
    let encode = expand_encode(&message_ty, &ty_generics, &codec_type, &ext_path, &eyre);
    let dyn_clone = expand_dyn_clone(
        &message_ty,
        &ty_generics,
        &dy_message,
        &message_impl,
        &eyre,
        cloneable,
    );
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

            fn encode(&self, _reg: &#reg) -> #eyre_result<Vec<u8>> {
                #encode
            }

            fn dyn_clone(&self) -> #eyre_result<#dy_message> {
                #dyn_clone
            }

            fn is_cloneable(&self) -> bool {
                #cloneable
            }
        }
    };
    let impl_message = if matches!(message_impl, MessageImpl::OrphanMessage) {
        let message_trait = with_crate_str("OrphanMessage");
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
    eyre_result: &TokenStream,
) -> TokenStream {
    let decoder_trait = with_crate_str("message::MessageDecoder");
    match codec_type {
        CodecType::NonCodec => {
            quote!(None)
        }
        CodecType::Codec => {
            match message_impl {
                MessageImpl::Message => {
                    decoder(decoder_trait, dy_message, reg, eyre_result, || {
                        quote! {
                            let message: #message_ty = #ext_path::decode_bytes(bytes)?;
                            let message = #dy_message::user(message);
                            Ok(message)
                        }
                    })
                }
                MessageImpl::SystemMessage => {
                    decoder(decoder_trait, dy_message, reg, eyre_result, || {
                        quote! {
                            let message: #message_ty = #ext_path::decode_bytes(bytes)?;
                            let message = #dy_message::system(message);
                            Ok(message)
                        }
                    })
                }
                MessageImpl::OrphanMessage => {
                    decoder(decoder_trait, dy_message, reg, eyre_result, || {
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
    decoder_trait: TokenStream,
    dy_message: &TokenStream,
    reg: &TokenStream,
    eyre_result: &TokenStream,
    fn_body: F,
) -> TokenStream where F: FnOnce() -> TokenStream {
    let body = fn_body();
    quote! {
        #[derive(Clone)]
        struct D;
        impl #decoder_trait for D {
            fn decode(&self, bytes: &[u8], _reg: &#reg) -> #eyre_result<#dy_message> {
                #body
            }
        }
        Some(Box::new(D))
    }
}

pub(crate) fn expand_encode(
    message_ty: &Ident,
    ty_generics: &TypeGenerics,
    codec_type: &CodecType,
    ext_path: &TokenStream,
    eyre: &TokenStream,
) -> TokenStream {
    match codec_type {
        CodecType::NonCodec => {
            quote! {
                Err(#eyre!("{} cannot codec", std::any::type_name::<#message_ty #ty_generics>()))
            }
        }
        CodecType::Codec => {
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
    eyre: &TokenStream,
    cloneable: bool,
) -> TokenStream {
    if !cloneable {
        quote! {
            Err(#eyre!("message {} is not cloneable", std::any::type_name::<#message_ty #ty_generics>()))
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