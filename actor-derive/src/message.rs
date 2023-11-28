use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Attribute, ImplGenerics, parse_str, Type, TypeGenerics, WhereClause};

use crate::metadata::{CodecType, MessageImpl};
use crate::with_crate;

pub fn expand(ast: syn::DeriveInput, message_impl: MessageImpl, codec_type: CodecType, cloneable: bool) -> TokenStream {
    let message_ty = ast.ident;
    let actor_attr = ast.attrs.into_iter().filter(|attr| attr.path.is_ident("actor")).next();
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let codec_trait = with_crate(parse_str("CodecMessage").unwrap());
    let decoder_trait = with_crate(parse_str("decoder::MessageDecoder").unwrap());
    let ext_path = with_crate(parse_str("ext").unwrap());
    let dy_message = with_crate(parse_str("DynMessage").unwrap());
    let decoder = expand_decoder(actor_attr.as_ref(), &message_ty, &message_impl, &codec_type, &ext_path, &dy_message);
    let encode = expand_encode(&codec_type, &ext_path);
    let dyn_clone = expand_dyn_clone(&dy_message, actor_attr.as_ref(), &message_impl, cloneable);
    let codec_impl = quote! {
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

            fn dyn_clone(&self) -> Option<#dy_message> {
                #dyn_clone
            }
        }
    };
    let impl_message = if matches!(message_impl, MessageImpl::UntypedMessage) {
        let message_trait = with_crate(parse_str("UntypedMessage").unwrap());
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

pub(crate) fn expand_decoder(actor_attr: Option<&Attribute>, message_ty: &Ident, message_impl: &MessageImpl, codec_type: &CodecType, ext_path: &TokenStream, dy_message: &TokenStream) -> TokenStream {
    let provider_trait = with_crate(parse_str("provider::ActorRefProvider").unwrap());
    let decoder_trait = with_crate(parse_str("decoder::MessageDecoder").unwrap());
    match codec_type {
        CodecType::NoneSerde => {
            quote!(None)
        }
        CodecType::Serde => {
            match message_impl {
                MessageImpl::Message => {
                    let actor_ty = actor_attr.expect("actor attribute not found").parse_args::<Type>().expect("expect a type");
                    let user_delegate = with_crate(parse_str("delegate::user::UserDelegate").unwrap());
                    decoder(&decoder_trait, &provider_trait, &dy_message, || {
                        quote! {
                            let message: #message_ty = #ext_path::decode_bytes(bytes)?;
                            let message = #user_delegate::<#actor_ty>::new(message);
                            Ok(message.into())
                        }
                    })
                }
                MessageImpl::AsyncMessage => {
                    let actor_ty = actor_attr.expect("actor attribute not found").parse_args::<Type>().expect("expect a type");
                    let async_delegate = with_crate(parse_str("delegate::user::AsyncUserDelegate").unwrap());
                    decoder(&decoder_trait, &provider_trait, &dy_message, || {
                        quote! {
                            let message: #message_ty = #ext_path::decode_bytes(bytes)?;
                            let message = #async_delegate::<#actor_ty>::new(message);
                            Ok(message.into())
                        }
                    })
                }
                MessageImpl::SystemMessage => {
                    let system_delegate = with_crate(parse_str("delegate::system::SystemDelegate").unwrap());
                    decoder(&decoder_trait, &provider_trait, &dy_message, || {
                        quote! {
                            let message: #message_ty = #ext_path::decode_bytes(bytes)?;
                            let message = #system_delegate::new(message);
                            Ok(message.into())
                        }
                    })
                }
                MessageImpl::UntypedMessage => {
                    decoder(&decoder_trait, &provider_trait, &dy_message, || {
                        quote! {
                            let message: #message_ty = #ext_path::decode_bytes(bytes)?;
                            let message = #dy_message::untyped(message);
                            Ok(message)
                        }
                    })
                }
            }
        }
    }
}

pub(crate) fn decoder<F>(decoder_trait: &TokenStream, provider_trait: &TokenStream, dy_message: &TokenStream, fn_body: F) -> TokenStream where F: FnOnce() -> TokenStream {
    let body = fn_body();
    quote! {
        #[derive(Clone)]
        struct D;
        impl #decoder_trait for D {
            fn decode(&self, provider: &#provider_trait, bytes: &[u8]) -> anyhow::Result<#dy_message> {
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

pub(crate) fn expand_dyn_clone(dy_message: &TokenStream, actor_attr: Option<&Attribute>, message_impl: &MessageImpl, cloneable: bool) -> TokenStream {
    if !cloneable {
        quote! {
            None
        }
    } else {
        match message_impl {
            MessageImpl::Message => {
                let user_delegate = with_crate(parse_str("delegate::user::UserDelegate").unwrap());
                let actor_ty = actor_attr.expect("actor attribute not found").parse_args::<Type>().expect("expect a type");
                quote! {
                    let message = #user_delegate::<#actor_ty>::new(Clone::clone(self));
                    Some(message.into())
                }
            }
            MessageImpl::AsyncMessage => {
                let async_delegate = with_crate(parse_str("delegate::user::AsyncUserDelegate").unwrap());
                let actor_ty = actor_attr.expect("actor attribute not found").parse_args::<Type>().expect("expect a type");
                quote! {
                    let message = #async_delegate::<#actor_ty>::new(Clone::clone(self));
                    Some(message.into())
                }
            }
            MessageImpl::SystemMessage => {
                let system_delegate = with_crate(parse_str("delegate::system::SystemDelegate").unwrap());
                quote! {
                    let message = #system_delegate::new(Clone::clone(self));
                    Some(message.into())
                }
            }
            MessageImpl::UntypedMessage => {
                quote! {
                    let message = #dy_message::untyped(Clone::clone(self));
                    Some(message)
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