use std::{
    any::{Any, TypeId},
    fmt::Debug,
    pin::Pin,
};

use tokio::sync::mpsc;

use crate::error::CommsError;

pub(crate) trait DynSender: Send + Sync + Debug {
    fn send_erased(
        &self,
        msg: Box<dyn Any + Send>,
    ) -> Pin<Box<dyn Future<Output = Result<(), CommsError>> + Send>>;
    fn accepts_message_type_id(&self) -> TypeId;
    fn message_type_name(&self) -> &'static str;
}

/// Just a type alias with the required trait bounds.
/// and a blanket impl for any `T`
pub(crate) trait ConcreteSenderTrait:
    Send + Sync + 'static + std::fmt::Debug + Clone
{
}
impl<T: Send + Sync + 'static + std::fmt::Debug + Clone> ConcreteSenderTrait for T {}

#[derive(Debug)]
pub(crate) struct ConcreteSender<T: ConcreteSenderTrait> {
    pub sender: mpsc::Sender<T>,
    pub _marker: std::marker::PhantomData<T>,
}

impl<T: ConcreteSenderTrait> DynSender for ConcreteSender<T> {
    fn send_erased(
        &self,
        msg_any: Box<dyn Any + Send>,
    ) -> Pin<Box<dyn Future<Output = Result<(), CommsError>> + Send>> {
        match msg_any.downcast::<T>() {
            Ok(concrete_msg) => {
                let sender_clone = self.sender.clone();
                Box::pin(async move {
                    sender_clone.send(*concrete_msg).await.map_err(|e| {
                        CommsError::SendFailed(format!(
                            "Failed to send message of type {}: {:?}",
                            std::any::type_name::<T>(),
                            e
                        ))
                    })
                })
            }
            Err(_) => Box::pin(async {
                Err(CommsError::TypeMismatch(format!(
                    "Downcast failed. Expected type {} for sender, got different type.",
                    std::any::type_name::<T>()
                )))
            }),
        }
    }

    fn accepts_message_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn message_type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
}
