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
    message::expand(ast, MessageImpl::Message, CodecType::NoneSerde).into()
}

#[proc_macro_derive(MessageCodec, attributes(actor))]
pub fn message_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::Message, CodecType::Serde).into()
}

#[proc_macro_derive(AsyncMessageCodec, attributes(actor))]
pub fn async_message_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::AsyncMessage, CodecType::Serde).into()
}

#[proc_macro_derive(SystemMessageCodec)]
pub fn system_message_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::SystemMessage, CodecType::Serde).into()
}

#[proc_macro_derive(DeferredMessageCodec)]
pub fn deferred_message_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::DeferredMessage, CodecType::Serde).into()
}

#[proc_macro_derive(UntypedMessageCodec, attributes(actor))]
pub fn untyped_message_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(ast, MessageImpl::UntypedMessage, CodecType::Serde).into()
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