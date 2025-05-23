use crosslink::{Router, define_crosslink};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

/// These are generated by `define_crosslink!`
pub use ping_pong_link::{marker::*, setup_ping_pong_link};

define_crosslink! {
    link_id: "PingPongLink",
    PingerHandle {
        sends: Ping,
        receives: Pong,
    },
    PongerHandle {
        sends: Pong,
        receives: Ping,
    },
    buffer_size: 16,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum Ping {
    Test(String),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum Pong {
    Test(String),
}

#[tokio::main]
#[allow(dead_code)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut router = Router::new();
    setup_ping_pong_link(&mut router, None);

    // New Arc so we can pass it around
    let router = Arc::new(router);

    // Freely clonable into any tokio task
    let pinger_router = Arc::clone(&router);
    tokio::spawn(async move {
        // Take receiver using the generated marker type for Pinger receiving Pong
        // This is ::ping_pong_link::marker::PingerHandleRecv
        let mut pinger_rx = pinger_router
            .take_receiver::<PingerHandleRecv, Pong>()
            .expect("Pinger failed to take Pong receiver");

        for i in 0..3 {
            let msg = Ping::Test(format!("ping from pinger ({})", i));
            println!("[Pinger] Sending: {:?}", msg);

            // Send using the generated marker type
            if let Err(e) = pinger_router.send::<PingerHandleSend, _>(msg).await {
                eprintln!("[Pinger] Send error: {}", e);
                return;
            }

            // Receive on the receiver the same way we normally would
            // Except this is typed automatically, returning `Option<Pong>`
            if let Some(pong_msg) = pinger_rx.recv().await {
                println!("[Pinger] Received: {:?}", pong_msg);
            } else {
                println!("[Pinger] Ponger disconnected.");
                return;
            }
            sleep(Duration::from_millis(300)).await;
        }
        println!("[Pinger] Finished.");
    });

    let ponger_router = Arc::clone(&router);
    tokio::spawn(async move {
        // Take receiver using the generated marker type
        // This is ::ping_pong_link::marker::PongerHandleRecv
        let mut ponger_rx = ponger_router
            .take_receiver::<PongerHandleRecv, Ping>()
            .expect("Ponger failed to take Ping receiver");

        loop {
            // Receive `Option<Ping>` messages generated by another task
            if let Some(ping_msg) = ponger_rx.recv().await {
                println!("[Ponger] Received: {:?}", ping_msg);
                let reply = Pong::Test("ack".to_string());
                println!("[Ponger] Sending reply: {:?}", reply);

                // And send too if you want
                if let Err(e) = ponger_router.send::<PongerHandleSend, _>(reply).await {
                    eprintln!("[Ponger] Send error: {}", e);
                    return;
                }
            } else {
                println!("[Ponger] Pinger disconnected.");
                return;
            }
        }
    });

    // Keep main alive for a bit for tasks to run
    // This is not necessary if you're using `tokio::spawn` directly,
    // and you have a main loop that runs for the lifetime of the program.
    // IF you terminate too early, issues can happen the same way as with
    // regular tokio tasks.
    // (this is just a tokio channel wrapper if we're being honest)
    // Make sure to keep things alive for long enoughl.
    sleep(Duration::from_secs(2)).await;
    Ok(())
}
