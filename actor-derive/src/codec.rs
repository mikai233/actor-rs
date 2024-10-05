use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use crate::{with_crate_str, CRATE_ACTOR_REMOTE};

pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let message_trait = with_crate_str(CRATE_ACTOR_REMOTE, "Message")?;
    let message_codec_trait = with_crate_str(CRATE_ACTOR_REMOTE, "MessageCodec")?;
    let message_codec_registry_trait = with_crate_str(CRATE_ACTOR_REMOTE, "MessageCodecRegistry")?;
    let stream = quote! {
        impl #impl_generics #message_codec_trait for #name #ty_generics #where_clause {
            type M: #message_trait;

            fn encode(message: &Self::M, _: &dyn #message_codec_registry_trait) -> anyhow::Result<Vec<u8>> {
                let bytes = bincode::serialize(message)?;
                Ok(bytes)
            }

            fn decode(bytes: &[u8], _: &dyn #message_codec_registry_trait) -> anyhow::Result<Self::M> {
                let message = bincode::deserialize(bytes)?;
                Ok(message)
            }
        };
    };
    Ok(stream)
}
