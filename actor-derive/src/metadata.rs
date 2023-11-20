use std::str::FromStr;

use strum::EnumString;
use syn::{LitStr, Token};
use syn::parse::{Parse, ParseStream};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(mtype);
    custom_keyword!(actor);
}

#[derive(Debug, EnumString)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum MessageType {
    NoneSerde,
    Serde,
    AsyncSerde,
    SystemSerde,
}

impl Parse for MessageType {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::mtype) {
            input.parse::<kw::mtype>()?;
            input.parse::<Token![=]>()?;
            let m_type = input.parse::<LitStr>()?;
            let m_type = MessageType::from_str(m_type.value().as_str()).unwrap();
            Ok(m_type)
        } else {
            Err(lookahead.error())
        }
    }
}

pub(crate) struct ActorType {
    pub(crate) name: LitStr,
}

impl Parse for ActorType {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::actor) {
            input.parse::<kw::actor>()?;
            input.parse::<Token![=]>()?;
            let m_type = input.parse::<LitStr>()?;
            Ok(ActorType { name: m_type })
        } else {
            Err(lookahead.error())
        }
    }
}

pub(crate) enum CodecMeta {
    Type(MessageType),
    Actor(ActorType),
}

impl Parse for CodecMeta {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::mtype) {
            let ty = MessageType::parse(input)?;
            Ok(CodecMeta::Type(ty))
        } else if lookahead.peek(kw::actor) {
            let ty = ActorType::parse(input)?;
            Ok(CodecMeta::Actor(ty))
        } else {
            Err(lookahead.error())
        }
    }
}