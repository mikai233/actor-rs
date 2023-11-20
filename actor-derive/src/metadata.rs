use strum::EnumString;
use syn::parse::{Parse, ParseStream};
use syn::Token;

mod kw {
    use syn::custom_keyword;

    custom_keyword!(mtype);
    custom_keyword!(none_serde);
    custom_keyword!(serde);
    custom_keyword!(async_serde);
    custom_keyword!(system_serde);

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
        if lookahead.peek(kw::none_serde) {
            input.parse::<Token![=]>()?;
            input.parse::<String>()?;
            Ok(MessageType::NoneSerde)
        } else if lookahead.peek(kw::serde) {
            input.parse::<Token![=]>()?;
            input.parse::<String>()?;
            Ok(MessageType::Serde)
        } else if lookahead.peek(kw::async_serde) {
            input.parse::<Token![=]>()?;
            input.parse::<String>()?;
            Ok(MessageType::AsyncSerde)
        } else if lookahead.peek(kw::system_serde) {
            input.parse::<Token![=]>()?;
            input.parse::<String>()?;
            Ok(MessageType::SystemSerde)
        } else {
            Err(lookahead.error())
        }
    }
}

pub(crate) struct MessageMeta {
    ty: Option<MessageType>,
    actor: Option<String>,
}

impl Parse for MessageMeta {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::mtype) {
            let kw = input.parse::<kw::none_serde>()?;
        } else if lookahead.peek(kw::actor) {} else if lookahead.peek(kw::async_serde) {} else if lookahead.peek(kw::system_serde) {} else {
            Err(lookahead.error())
        }
        todo!()
    }
}