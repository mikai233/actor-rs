use proc_macro::TokenStream;

use proc_macro2::{Ident, Span};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::{parse_str, DeriveInput};

mod as_any;
mod message;
mod metadata;

#[proc_macro_derive(Message, attributes(cloneable))]
pub fn empty_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    todo!()
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

pub(crate) fn with_crate_str(s: &str) -> proc_macro2::TokenStream {
    with_crate(parse_str(s).unwrap())
}
