use std::{any::Any, fmt::Debug};
use tokio::sync::mpsc;

pub trait ConcreteReceiverTrait: Send + 'static + Debug {}
impl<T: Send + 'static + Debug> ConcreteReceiverTrait for T {}

pub trait DynReceiver: Send + Debug {
    /// Consumes the Box<dyn DynReceiver> and converts it into a Box<dyn Any + Send>.
    /// This is essential for downcasting to a concrete type if needed.
    fn into_any(self: Box<Self>) -> Box<dyn Any + Send>;
}

#[derive(Debug)]
pub(crate) struct ConcreteReceiver<T: ConcreteReceiverTrait> {
    pub receiver: mpsc::Receiver<T>,
}

impl<T: ConcreteReceiverTrait> DynReceiver for ConcreteReceiver<T> {
    fn into_any(self: Box<Self>) -> Box<dyn Any + Send> {
        // Since ConcreteReceiver<T> is 'static,
        // Box<ConcreteReceiver<T>> can be cast to Box<dyn Any + Send>.
        self
    }
}
