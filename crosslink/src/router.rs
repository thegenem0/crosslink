use std::{any::TypeId, collections::HashMap, fmt::Debug};

use tokio::sync::mpsc;

use crate::{
    error::CommsError,
    sender::{ConcreteSender, DynSender},
};

/// Uniquely identifies a specific directed pathway within the Commander.
/// Format: "UserLinkEnumVariantName/FromStructName_to_ToStructName"
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InternalDispatchKey(pub String); // Made pub for potential advanced usage/debugging

/// Maps a message type to a specific internal pathway for a given user link.
#[derive(Debug)]
pub struct TypeToPathwayMapping {
    pub msg_type_id: TypeId,
    pub msg_type_name: &'static str,
    /// The full InternalDispatchKey to use for this type on this link.
    pub pathway_key_to_use: InternalDispatchKey,
}

#[derive(Debug, Default)]
pub struct Router {
    /// Stores the actual senders, keyed by their full internal pathway.
    pathway_senders: HashMap<InternalDispatchKey, Box<dyn DynSender>>,
    /// Maps a user-facing link ID (string version of the enum variant) to a list of
    /// message types it can send and their corresponding internal pathways.
    /// Key: String representation of the Link Enum variant (e.g., "CoordinatorToLauncher").
    link_id_to_type_dispatch_info: HashMap<String, Vec<TypeToPathwayMapping>>,
}

impl Router {
    pub fn new() -> Self {
        Default::default()
    }

    /// Internal method called by the setup macro to register a communication pathway
    /// and its association with a message type for a given link ID.
    /// **This method is intended for use by the `define_links!` macro only.**
    #[doc(hidden)]
    pub fn __internal_register_pathway_and_type_mapping<Msg>(
        &mut self,
        link_id_variant_name_str: &str, // String representation of the Link Enum variant
        source_struct_name_str: &str,   // Name of the sending handle struct
        target_struct_name_str: &str,   // Name of the receiving handle struct
        sender: mpsc::Sender<Msg>,
    ) -> Result<(), CommsError>
    where
        Msg: Send + Sync + 'static + Debug + Clone,
    {
        let internal_pathway_key_str = format!(
            "{}/{}_to_{}",
            link_id_variant_name_str, source_struct_name_str, target_struct_name_str
        );
        let internal_key = InternalDispatchKey(internal_pathway_key_str.clone());

        // 1. Register the actual sender endpoint
        if self.pathway_senders.contains_key(&internal_key) {
            return Err(CommsError::PathwayAlreadyRegistered(
                internal_pathway_key_str,
            ));
        }
        self.pathway_senders.insert(
            internal_key.clone(),
            Box::new(ConcreteSender {
                sender,
                _marker: std::marker::PhantomData,
            }),
        );

        // 2. Register the mapping from (link_id_variant_name, message_type) to internal_key
        let type_mappings = self
            .link_id_to_type_dispatch_info
            .entry(link_id_variant_name_str.to_string())
            .or_default();

        let msg_type_id = TypeId::of::<Msg>();
        if type_mappings.iter().any(|tm| tm.msg_type_id == msg_type_id) {
            return Err(CommsError::MessageTypeNotMappedForLink(format!(
                "Message type '{}' is already mapped for link '{}' (pathway: {}). 
                Ambiguous for type-based dispatch on this link. 
                Each (link_id, message_type) pair must uniquely identify one pathway.",
                std::any::type_name::<Msg>(),
                link_id_variant_name_str,
                internal_pathway_key_str
            )));
        }

        type_mappings.push(TypeToPathwayMapping {
            msg_type_id,
            msg_type_name: std::any::type_name::<Msg>(),
            pathway_key_to_use: internal_key,
        });

        Ok(())
    }

    /// Sends a message on a specified link. The Commander infers the exact
    /// pathway (direction/target) based on the message's type.
    ///
    /// `LKey` is expected to be the macro-generated enum for link IDs.
    /// It must implement `AsRef<str>` (e.g., via `strum_macros::AsRefStr`).
    pub async fn send<LKey, Message>(
        &self,
        link_key: LKey,
        message: Message,
    ) -> Result<(), CommsError>
    where
        LKey: AsRef<str> + Copy + Debug, // `AsRef<str>` gets "VariantName" from LKey::VariantName
        Message: Send + Sync + 'static + Debug + Clone,
    {
        let link_id_str = link_key.as_ref();
        let msg_type_id_to_send = TypeId::of::<Message>();
        let msg_type_name_to_send = std::any::type_name::<Message>();

        match self.link_id_to_type_dispatch_info.get(link_id_str) {
            Some(pathway_mappings_for_link) => {
                if let Some(mapping_info) = pathway_mappings_for_link
                    .iter()
                    .find(|m| m.msg_type_id == msg_type_id_to_send)
                {
                    match self.pathway_senders.get(&mapping_info.pathway_key_to_use) {
                        Some(dyn_sender) => {
                            if dyn_sender.accepts_message_type_id() != msg_type_id_to_send {
                                return Err(CommsError::InternalInconsistency(format!(
                                    "Metadata mismatch for link '{}', pathway '{}'. 
                                    Expected type '{}' for sending, but sender is configured for '{}'.",
                                    link_id_str,
                                    mapping_info.pathway_key_to_use.0,
                                    msg_type_name_to_send,
                                    dyn_sender.message_type_name()
                                )));
                            }
                            // Pass Box<dyn Any + Send> to send_erased
                            dyn_sender.send_erased(Box::new(message)).await
                        }
                        None => Err(CommsError::PathwayNotFound(format!(
                            "Internal error: Pathway key '{}' (for type '{}' on link '{}') not found in senders map.",
                            mapping_info.pathway_key_to_use.0, msg_type_name_to_send, link_id_str
                        ))),
                    }
                } else {
                    Err(CommsError::MessageTypeNotMappedForLink(format!(
                        "No pathway configured for link '{}' that accepts message type '{}'.
                        Ensure this message type is defined for sending on this link in `define_links!`.",
                        link_id_str, msg_type_name_to_send
                    )))
                }
            }
            None => Err(CommsError::LinkNotFound(format!(
                "Unknown link '{}'. Was it defined and spelled correctly in `define_links!` and Link Enum?",
                link_id_str
            ))),
        }
    }
}
