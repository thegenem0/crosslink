# Crosslink 🔗

Crosslink provides a typed, asynchronous communication framework for Rust, enabling clear and robust inter-component messaging using Tokio MPSC channels.

It uses a procedural macro (`define_crosslink!`) to help you define strongly-typed communication links, which are then managed by a central `Router` for dispatching messages and acquiring receivers.

## Features

- **Type-Safe Links:** Define communication pathways with compile-time message type enforcement.
- **Centralized `Router`:** Simplifies message dispatch and receiver acquisition.

- **Macro-Driven Setup:** `define_crosslink!` generates necessary types and a setup function for each link, reducing boilerplate.
- **Asynchronous:** Built for Tokio-based applications.

## Example

For examples, see the [examples](crosslink/examples) directory.
