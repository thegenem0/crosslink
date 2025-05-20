#[derive(Debug, Clone, thiserror::Error)]
pub enum CommsError {
    #[error("Send failed: {0}")]
    SendFailed(String),

    #[error("Receive failed: {0}")]
    RecvFailed(String),

    #[error("Type mismatch: {0}")]
    TypeMismatch(String),

    #[error("Pathway already registered: {0}")]
    PathwayAlreadyRegistered(String),

    #[error("Pathway not found: {0}")]
    PathwayNotFound(String),

    #[error("Link not found: {0}")]
    LinkNotFound(String),

    #[error("Message type not mapped for link: {0}")]
    MessageTypeNotMappedForLink(String),

    #[error("Internal inconsistency: {0}")]
    InternalInconsistency(String),
}
