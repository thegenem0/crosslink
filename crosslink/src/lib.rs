//! # Crosslink
//!
//! Typed, asynchronous communication channels for Rust, built on Tokio MPSC.
//!
//! Crosslink uses a procedural macro (`define_crosslink!`) to generate type-safe
//! communication links, managed by a central `Router`. This provides a clear and
//! robust way to handle inter-component messaging.
//!
//! ## Quick Start
//!
//! ```rust
//! use crosslink::{Router, define_crosslink, CommsError};
//! use std::sync::Arc;
//! use tokio::time::{sleep, Duration};
//!
//! // 1. Define your message types
//! #[derive(Debug, Clone)]
//! pub struct Ping(u32);
//! // Ensure types satisfy Crosslink's internal trait bounds:
//! // For sending: Send + Sync + 'static + Debug + Clone
//! // For receiving: Send + 'static + Debug
//! impl crosslink::sender::ConcreteSenderTrait for Ping {}
//! impl crosslink::receiver::ConcreteReceiverMessageTrait for Ping {}
//!
//! #[derive(Debug, Clone)]
//! pub struct Pong(u32);
//! impl crosslink::sender::ConcreteSenderTrait for Pong {}
//! impl crosslink::receiver::ConcreteReceiverMessageTrait for Pong {}
//!
//! // 2. Define the link using the macro (typically at module level)
//! // This generates:
//! // - `pub mod ping_pong { ... }` containing marker types and setup function.
//! // - Marker types like `ping_pong::marker::PingerSenderMarker`.
//! // - Setup function `ping_pong::setup_ping_pong_link(...)`.
//! define_crosslink! {
//!     link_id_prefix: PingPong, // Forms module `ping_pong` & func `setup_ping_pong`
//!     Pinger { sends: Ping, receives: Pong }, // Defines `ping_pong::Pinger` (nominal)
//!     Ponger { sends: Pong, receives: Ping }, // Defines `ping_pong::Ponger` (nominal)
//!     buffer_size: 8,
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // 3. Setup Router
//!     let mut router = Router::new();
//!     // Call the generated setup function for the link
//!     ping_pong::setup_ping_pong(&mut router, Some(8)); // buffer_override
//!
//!     let shared_router = Arc::new(router);
//!
//!     // 4. Pinger Task
//!     let pinger_router = Arc::clone(&shared_router);
//!     tokio::spawn(async move {
//!         // Obtain receiver using the generated marker and expected message type
//!         let mut pinger_rx = pinger_router
//!             .take_receiver::<ping_pong::marker::PingPongPingerReceiverMarker, Pong>()
//!             .expect("Pinger: Failed to take Pong receiver");
//!
//!         for i in 0..2 {
//!             let msg = Ping(i);
//!             println!("[Pinger] Sending: {:?}", msg);
//!             // Send using the corresponding sender marker and message type
//!             if let Err(e) = pinger_router.send::<ping_pong::marker::PingPongPingerSenderMarker, _>(msg).await {
//!                 eprintln!("[Pinger] Send error: {}", e); return;
//!             }
//!
//!             if let Some(pong_msg) = pinger_rx.recv().await {
//!                 println!("[Pinger] Received: {:?}", pong_msg);
//!             } else {
//!                 println!("[Pinger] Ponger disconnected."); return;
//!             }
//!         }
//!     });
//!
//!     // 5. Ponger Task
//!     let ponger_router = Arc::clone(&shared_router);
//!     tokio::spawn(async move {
//!         let mut ponger_rx = ponger_router
//!             .take_receiver::<ping_pong::marker::PingPongPongerReceiverMarker, Ping>()
//!             .expect("Ponger: Failed to take Ping receiver");
//!
//!         while let Some(ping_msg) = ponger_rx.recv().await {
//!             println!("[Ponger] Received: {:?}", ping_msg);
//!             let reply = Pong(ping_msg.0); // Respond with Pong
//!             println!("[Ponger] Sending: {:?}", reply);
//!             if let Err(e) = ponger_router.send::<ping_pong::marker::PingPongPongerSenderMarker, _>(reply).await {
//!                 eprintln!("[Ponger] Send error: {}", e); return;
//!             }
//!         }
//!         println!("[Ponger] Pinger disconnected.");
//!     });
//!
//!     sleep(Duration::from_millis(500)).await; // Allow tasks to run
//!     Ok(())
//! }
//! ```
//!
//! The `define_crosslink!` macro handles the generation of necessary types and
//! a setup function for each link, which configures your central `Router`.
//! You then use the `Router` with generated marker types for type-safe message
//! sending and receiver acquisition.

pub mod error;
pub mod receiver;
pub mod router;
pub mod sender;

pub use error::CommsError;
pub use router::Router;

pub use crosslink_macros::define_crosslink;
