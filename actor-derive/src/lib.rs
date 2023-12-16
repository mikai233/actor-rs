use proc_macro::TokenStream;

use proc_macro2::{Ident, Span};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::DeriveInput;

use crate::metadata::{CodecType, MessageImpl};

mod message;
mod metadata;

#[proc_macro_derive(EmptyCodec)]
pub fn empty_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::Message, CodecType::NoneSerde, false).into()
}

#[proc_macro_derive(CloneableEmptyCodec, attributes(actor))]
pub fn cloneable_empty_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::Message, CodecType::NoneSerde, true).into()
}

#[proc_macro_derive(MessageCodec, attributes(actor))]
pub fn message_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::Message, CodecType::Serde, false).into()
}

#[proc_macro_derive(CloneableMessageCodec, attributes(actor))]
pub fn cloneable_message_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::Message, CodecType::Serde, true).into()
}

#[proc_macro_derive(AsyncMessageCodec, attributes(actor))]
pub fn async_message_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::AsyncMessage, CodecType::Serde, false).into()
}

#[proc_macro_derive(CloneableAsyncMessageCodec, attributes(actor))]
pub fn cloneable_async_message_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::AsyncMessage, CodecType::Serde, true).into()
}

#[proc_macro_derive(SystemMessageCodec)]
pub fn system_message_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::SystemMessage, CodecType::Serde, false).into()
}

#[proc_macro_derive(CloneableSystemMessageCodec)]
pub fn clonealbe_system_message_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::SystemMessage, CodecType::Serde, true).into()
}

#[proc_macro_derive(UntypedMessageCodec, attributes(actor))]
pub fn untyped_message_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::UntypedMessage, CodecType::Serde, false).into()
}

#[proc_macro_derive(UntypedMessageEmptyCodec)]
pub fn untyped_message_empty_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::UntypedMessage, CodecType::NoneSerde, false).into()
}

#[proc_macro_derive(CloneableUntypedMessageCodec, attributes(actor))]
pub fn cloneable_untyped_message_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::UntypedMessage, CodecType::Serde, true).into()
}

#[proc_macro_derive(CloneableUntypedMessageEmptyCodec, attributes(actor))]
pub fn cloneable_untyped_message_empty_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::UntypedMessage, CodecType::NoneSerde, true).into()
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