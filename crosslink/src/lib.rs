//! # Crosslink
//!
//! A typed, asynchronous communication framework for Rust applications,
//! enabling clear and robust inter-component messaging.
//!
//! ## Features
//!
//! - **Type-Safe Links:** Define communication links between components with explicit message types.
//! - **Centralized Commander:** A single `Commander` instance dispatches messages, simplifying dependencies.
//! - **Procedural Macro Setup:** The `define_links!` macro generates boilerplate for link definitions,
//!   enum keys, and typed receiver handles.
//! - **Asynchronous:** Built on `tokio` MPSC channels.
//!
//! ## Usage
//!
//! ```rust,ignore
//! // Define your message types
//! #[derive(Debug, Clone)]
//! struct PingMsg(String);
//! #[derive(Debug, Clone)]
//! struct PongMsg(String);
//!
//! // Use the define_links! macro to set up communication
//! crosslink::define_links! {
//!     enum AppLinkKeys;  // Name for your link ID enum
//!     struct AppHandles; // Name for the struct holding receiver handles
//!
//!     links {
//!         PingerToPonger: bi_directional (
//!             ep1 ( name: PingerHandle, sends: PingMsg, receives: PongMsg ),
//!             ep2 ( name: PongerHandle ), // Message types inferred from ep1
//!             buffer: 16,
//!         ),
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // The macro expands to an expression returning (Commander, AppHandles)
//!     let (commander, mut handles) = crosslink::define_links! { /* ... as above ... */ };
//!     let commander = std::sync::Arc::new(commander);
//!
//!     let pinger_commander = commander.clone();
//!     let mut pinger_handle = handles.pinger_to_ponger_pinger_handle; // Generated field name
//!
//!     tokio::spawn(async move {
//!         let ping = PingMsg("hello".to_string());
//!         println!("[Pinger] Sending: {:?}", ping);
//!         pinger_commander.send(AppLinkKeys::PingerToPonger, ping).await.unwrap();
//!
//!         if let Some(pong) = pinger_handle.recv().await {
//!             println!("[Pinger] Received: {:?}", pong);
//!         }
//!     });
//!
//!     let ponger_commander = commander.clone();
//!     let mut ponger_handle = handles.pinger_to_ponger_ponger_handle;
//!
//!     tokio::spawn(async move {
//!         if let Some(ping) = ponger_handle.recv().await {
//!             println!("[Ponger] Received: {:?}", ping);
//!             let pong = PongMsg(format!("ack to {}", ping.0));
//!             println!("[Ponger] Sending: {:?}", pong);
//!             ponger_commander.send(AppLinkKeys::PingerToPonger, pong).await.unwrap();
//!         }
//!     });
//!
//!     // Keep main alive for a bit for tasks to run
//!     tokio::time::sleep(std::time::Duration::from_secs(1)).await;
//!     Ok(())
//! }
//! ```
//! (Note: The above example needs to be placed where `define_links!` is accessible,
//!  typically in user code, not library documentation directly if macro pathing is an issue.)

// Re-export the procedural macro from the crosslink-macros crate
pub use crosslink_macros::define_links;

// Potentially re-export strum for users if they need to interact with AsRefStr more directly,
// though it's mainly an internal detail for the macro and Commander.
// pub use strum;

pub mod error;
pub mod router;
pub mod sender;

pub use error::CommsError;
pub use router::{InternalDispatchKey, Router, TypeToPathwayMapping};
