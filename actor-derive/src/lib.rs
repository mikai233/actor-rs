use proc_macro::TokenStream;

use syn::DeriveInput;

mod message;
mod serialize_message;

#[proc_macro_derive(EmptyCodec)]
pub fn empty_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(&ast).into()
}

#[proc_macro_derive(SerializeCodec)]
pub fn serialize_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    todo!()
}

#[proc_macro_derive(SystemSerializeCodec)]
pub fn serialize_system_codec_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    todo!()
}