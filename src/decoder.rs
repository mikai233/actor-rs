use crate::actor::DynamicMessage;

pub trait MessageDecoder: Send + Sync + 'static {
    fn decode(&self, bytes: &[u8]) -> anyhow::Result<DynamicMessage>;
}

#[macro_export]
macro_rules! user_message_decoder {
    ($message:ty, $actor:ty) => {
        {
            struct D;
            impl MessageDecoder for D {
                fn decode(&self, bytes: &[u8]) -> anyhow::Result<crate::actor::DynamicMessage> {
                    let message: $message = crate::ext::decode_bytes(bytes)?;
                    let message = crate::delegate::user::UserDelegate::<$actor>::new(message);
                    Ok(message.into())
                }
            }
            Box::new(D)
        }
    };
}

#[macro_export]
macro_rules! system_message_decoder {
    ($message:ident) => {
        {
            struct D;
            impl MessageDecoder for D {
                fn decode(&self, bytes: &[u8]) -> anyhow::Result<crate::actor::DynamicMessage> {
                    let message: $message = crate::ext::decode_bytes(bytes)?;
                    let message = crate::delegate::system::SystemDelegate::new(message);
                    Ok(message.into())
                }
            }
            Box::new(D)
        }
    };
}