use proc_macro::TokenStream;

use proc_macro2::{Ident, Span};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::{parse_str, DeriveInput};

mod as_any;
mod codec;
mod message;

const CRATE_ACTOR_CORE: &str = "actor-core";

const CRATE_ACTOR_REMOTE: &str = "actor-remote";

#[proc_macro_derive(Message, attributes(cloneable))]
pub fn message_derive(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    let stream = message::expand(&input).unwrap_or_else(|err| err.to_compile_error());
    stream.into()
}

#[proc_macro_derive(MessageCodec)]
pub fn message_codec_derive(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    let stream = codec::expand(&input).unwrap_or_else(|err| err.to_compile_error());
    stream.into()
}

#[proc_macro_derive(AsAny)]
pub fn as_any(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    let stream = as_any::expand(&input).unwrap_or_else(|err| err.to_compile_error());
    stream.into()
}

pub(crate) fn with_crate(orig_name: &str, path: syn::Path) -> proc_macro2::TokenStream {
    let found_crate = crate_name(orig_name).expect("actor-core is present in `Cargo.toml`");
    match found_crate {
        FoundCrate::Itself => quote!(crate::#path),
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!(#ident::#path)
        }
    }
}

pub(crate) fn with_crate_str(orig_name: &str, s: &str) -> syn::Result<proc_macro2::TokenStream> {
    let path = parse_str(s)?;
    let path_with_crate = with_crate(orig_name, path);
    Ok(path_with_crate)
}
