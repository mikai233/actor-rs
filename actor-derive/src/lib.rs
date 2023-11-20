use proc_macro::TokenStream;

use proc_macro2::{Ident, Span};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::DeriveInput;

mod message;
mod serialize_message;
mod metadata;

#[proc_macro_derive(EmptyCodec)]
pub fn empty_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(&ast).into()
}

#[proc_macro_derive(MessageCodec, attributes(mtype))]
pub fn message_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    todo!()
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