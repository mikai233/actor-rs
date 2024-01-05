#![feature(proc_macro_span)]

use proc_macro::TokenStream;

use proc_macro2::{Ident, Span};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::DeriveInput;

use crate::metadata::{CodecType, MessageImpl};

mod message;
mod metadata;
mod as_any;

#[proc_macro_derive(EmptyCodec)]
pub fn empty_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::Message, CodecType::NoneSerde, false).into()
}

#[proc_macro_derive(CEmptyCodec)]
pub fn cloneable_empty_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::Message, CodecType::NoneSerde, true).into()
}

#[proc_macro_derive(MessageCodec)]
pub fn message_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::Message, CodecType::Serde, false).into()
}

#[proc_macro_derive(CMessageCodec)]
pub fn cloneable_message_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::Message, CodecType::Serde, true).into()
}

#[proc_macro_derive(SystemCodec)]
pub fn system_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::SystemMessage, CodecType::Serde, false).into()
}

#[proc_macro_derive(CSystemCodec)]
pub fn clonealbe_system_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::SystemMessage, CodecType::Serde, true).into()
}

#[proc_macro_derive(OrphanCodec)]
pub fn orphan_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::OrphanMessage, CodecType::Serde, false).into()
}

#[proc_macro_derive(OrphanEmptyCodec)]
pub fn orphan_empty_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::OrphanMessage, CodecType::NoneSerde, false).into()
}

#[proc_macro_derive(COrphanCodec)]
pub fn cloneable_orphan_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::OrphanMessage, CodecType::Serde, true).into()
}

#[proc_macro_derive(COrphanEmptyCodec)]
pub fn cloneable_orphan_empty_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::OrphanMessage, CodecType::NoneSerde, true).into()
}

#[proc_macro_derive(AsAny)]
pub fn as_any(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    as_any::expand(ast).into()
}

pub(crate) fn with_crate(path: syn::Path) -> proc_macro2::TokenStream {
    let found_crate = crate_name("actor-core").expect("actor-core is present in `Cargo.toml`");
    match found_crate {
        FoundCrate::Itself => quote!(crate::#path),
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!(#ident::#path)
        }
    }
}