use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt::Debug,
    sync::Mutex,
};

use tokio::sync::mpsc;

use crate::{
    error::CommsError,
    receiver::{ConcreteReceiver, ConcreteReceiverTrait, DynReceiver},
    sender::{ConcreteSender, ConcreteSenderTrait, DynSender},
};

#[derive(Debug, Default)]
#[allow(clippy::type_complexity)]
pub struct Router {
    typed_senders: HashMap<TypeId, Box<dyn DynSender>>,
    typed_receivers: HashMap<TypeId, (TypeId, Mutex<Option<Box<dyn DynReceiver>>>)>,
}

impl Router {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn __internal_register_sender<SenderMarker, Msg>(
        &mut self,
        sender: mpsc::Sender<Msg>,
    ) -> Result<(), CommsError>
    where
        SenderMarker: Any + Send + Sync + 'static,
        Msg: ConcreteSenderTrait,
    {
        let marker_type_id = TypeId::of::<SenderMarker>();

        if self.typed_senders.contains_key(&marker_type_id) {
            return Err(CommsError::PathwayAlreadyRegistered(format!(
                "Sender for marker type '{}' already registered.",
                std::any::type_name::<SenderMarker>()
            )));
        }

        self.typed_senders
            .insert(marker_type_id, Box::new(ConcreteSender { sender }));

        Ok(())
    }

    pub fn __internal_register_receiver<ReceiverMarker, Msg>(
        &mut self,
        receiver: mpsc::Receiver<Msg>, // Receiver for the owning end of the pathway
    ) -> Result<(), CommsError>
    where
        ReceiverMarker: Any + Send + Sync + 'static,
        Msg: ConcreteReceiverTrait,
    {
        let marker_type_id = TypeId::of::<ReceiverMarker>();
        if self.typed_receivers.contains_key(&marker_type_id) {
            return Err(CommsError::PathwayAlreadyRegistered(format!(
                "Receiver for marker type '{}' already registered.",
                std::any::type_name::<ReceiverMarker>()
            )));
        }

        let dyn_receiver_box: Box<dyn DynReceiver> = Box::new(ConcreteReceiver { receiver });
        self.typed_receivers.insert(
            marker_type_id,
            (TypeId::of::<Msg>(), Mutex::new(Some(dyn_receiver_box))),
        );

        Ok(())
    }

    /// Sends a message on a specified link.
    pub async fn send<SenderMarker, Msg>(&self, message: Msg) -> Result<(), CommsError>
    where
        SenderMarker: Any + Send + Sync + 'static,
        Msg: ConcreteSenderTrait,
    {
        let marker_type_id = TypeId::of::<SenderMarker>();
        let msg_type_id_to_send = TypeId::of::<Msg>();

        match self.typed_senders.get(&marker_type_id) {
            Some(dyn_sender) => {
                if dyn_sender.accepts_message_type_id() != msg_type_id_to_send {
                    return Err(CommsError::InternalInconsistency(format!(
                        "Metadata mismatch for link '{}', pathway '{}'.
                        Expected type '{}' for sending, but sender is configured for '{}'.",
                        std::any::type_name::<SenderMarker>(),
                        dyn_sender.message_type_name(),
                        std::any::type_name::<Msg>(),
                        dyn_sender.message_type_name()
                    )));
                }
                dyn_sender.send_erased(Box::new(message)).await
            }
            None => Err(CommsError::PathwayNotFound(format!(
                "No pathway configured for marker type '{}' that accepts message type '{}'.
                Ensure this message type is defined for sending on this link",
                std::any::type_name::<SenderMarker>(),
                std::any::type_name::<Msg>()
            ))),
        }
    }

    pub fn take_receiver<ReceiverMarker, Msg>(&self) -> Result<mpsc::Receiver<Msg>, CommsError>
    where
        ReceiverMarker: Any + Send + Sync + 'static,
        Msg: Send + 'static + Debug + Sync,
    {
        let marker_type_id = TypeId::of::<ReceiverMarker>();
        let expected_msg_type_id = TypeId::of::<Msg>();

        match self.typed_receivers.get(&marker_type_id) {
            Some((reg_type_id, receiver_lock)) => {
                if *reg_type_id != expected_msg_type_id {
                    return Err(CommsError::TypeMismatch(format!(
                        "Expected type '{}' for receiving.",
                        std::any::type_name::<Msg>(),
                    )));
                }

                let mut recv_guard = receiver_lock.lock().map_err(|e| {
                    CommsError::InternalInconsistency(format!(
                        "Failed to lock receiver for link '{}' and handle '{}'. Error: {}",
                        std::any::type_name::<ReceiverMarker>(),
                        std::any::type_name::<Msg>(),
                        e
                    ))
                })?;

                if let Some(dyn_receiver) = recv_guard.take() {
                    match dyn_receiver.into_any().downcast::<ConcreteReceiver<Msg>>() {
                        Ok(concrete_box_recv) => Ok(concrete_box_recv.receiver),
                        Err(_) => Err(CommsError::InternalInconsistency(format!(
                            "Critical: Downcast to ConcreteReceiver<{}> failed for key '{}' after TypeId match.",
                            std::any::type_name::<ReceiverMarker>(),
                            std::any::type_name::<Msg>()
                        ))),
                    }
                } else {
                    Err(CommsError::InternalInconsistency(format!(
                        "Failed to take receiver for link '{}' and handle '{}'.",
                        std::any::type_name::<ReceiverMarker>(),
                        std::any::type_name::<Msg>()
                    )))
                }
            }
            None => Err(CommsError::PathwayNotFound(format!(
                "No receiver for link '{}' and handle '{}' found.",
                std::any::type_name::<ReceiverMarker>(),
                std::any::type_name::<Msg>()
            ))),
        }
    }
}
