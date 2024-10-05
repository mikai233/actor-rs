use crate::{with_crate_str, CRATE_ACTOR_CORE};
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let cloneable = input.attrs.iter().any(|attr| { attr.path.is_ident("cloneable") });
    let message_trait = with_crate_str(CRATE_ACTOR_CORE, "message::Message")?;
    let signature_type = with_crate_str(CRATE_ACTOR_CORE, "message::Signature")?;

    let clone_box_stream = if cloneable {
        quote! {
            fn clone_box(&self) -> Option<Box<dyn #message_trait>> {
                Some(Box::new(self.clone()))
            }
        }
    } else {
        quote! {
            fn clone_box(&self) -> Option<Box<dyn #message_trait>> {
                None
            }
        }
    };
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let stream = quote! {
        impl #impl_generics #message_trait for #name #ty_generics #where_clause {

            fn signature_sized() -> #signature_type
            where
                Self: Sized,
            {
                #signature_type::new::<Self>()
            }

            fn signature(&self) -> #signature_type {
                #signature_type::new::<Self>()
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
                self
            }

            fn is_cloneable(&self) -> bool {
                #cloneable
            }

            #clone_box_stream
        }
    };
    Ok(stream)
}