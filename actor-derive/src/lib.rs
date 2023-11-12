mod message;
mod serialize_message;

use proc_macro::TokenStream;
use syn::DeriveInput;

#[proc_macro_derive(Codec)]
pub fn codec_message_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    message::expand(&ast).into()
}

#[proc_macro_derive(SerializeCodec)]
pub fn codec_serialize_message_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    todo!()
}

#[proc_macro_derive(SystemSerializeCodec)]
pub fn codec_serialize_system_message_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    todo!()
}