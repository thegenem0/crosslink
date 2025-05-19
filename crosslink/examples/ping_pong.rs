use crosslink::define_links; // Use the re-exported macro
use std::sync::Arc;

// 1. Define Message Types
#[derive(Debug, Clone)]
struct Ping(String, u32);

#[derive(Debug, Clone)]
struct Pong(String, u32);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 3. The macro expands to an expression returning (Commander, AppHandles)
    let (commander, handles) = define_links! { // Invoked again to get the instances
        enum AppLinks;
        struct AppHandles;
        links {
            PingerPongerLink: bi_directional (
                endpoint1 ( name: PingerCommHandle, sends: Ping, receives: Pong ),
                endpoint2 ( name: PongerCommHandle ),
                buffer: 8,
            ),
        }
    };

    let commander = Arc::new(commander);

    // --- Pinger Task ---
    let pinger_cmd = commander.clone();
    // Field names are generated: <link_id_lc>_<handle_name_lc>
    let mut pinger_channel = handles.pingerpongerlink_pingercommhandle;
    tokio::spawn(async move {
        for i in 0..3 {
            let msg = Ping(format!("ping from pinger ({})", i), i);
            println!("[Pinger] Sending: {:?}", msg);
            if let Err(e) = pinger_cmd.send(AppLinks::PingerPongerLink, msg).await {
                eprintln!("[Pinger] Send error: {}", e);
                return;
            }

            match pinger_channel.recv().await {
                Some(reply) => println!("[Pinger] Received reply: {:?}", reply),
                None => {
                    println!("[Pinger] Channel closed by ponger.");
                    return;
                }
            }
            sleep(Duration::from_millis(300)).await;
        }
    });

    // --- Ponger Task ---
    let ponger_cmd = commander.clone();
    let mut ponger_channel = handles.pingerpongerlink_pongercommhandle;
    tokio::spawn(async move {
        loop {
            match ponger_channel.recv().await {
                Some(ping_msg) => {
                    println!("[Ponger] Received: {:?}", ping_msg);
                    let reply = Pong(format!("pong responding to '{}'", ping_msg.0), ping_msg.1);
                    println!("[Ponger] Sending reply: {:?}", reply);
                    if let Err(e) = ponger_cmd.send(AppLinks::PingerPongerLink, reply).await {
                        eprintln!("[Ponger] Send error: {}", e);
                        break; // Exit task on send error
                    }
                }
                None => {
                    println!("[Ponger] Channel closed by pinger.");
                    break; // Exit task
                }
            }
        }
    });

    // --- Monitor Broadcaster Task (using the unidirectional link) ---
    let monitor_cmd = commander.clone();
    // let _monitor_sender_handle = handles.systemmonitorlink_monitorsenderhandle; // Handle useful if it had methods
    tokio::spawn(async move {
        for i in 0..5 {
            let status_msg = format!("System status update #{}", i);
            println!("[MonitorSender] Broadcasting: '{}'", status_msg);
            if let Err(e) = monitor_cmd
                .send(AppLinks::SystemMonitorLink, status_msg)
                .await
            {
                eprintln!("[MonitorSender] Broadcast error: {}", e);
            }
            sleep(Duration::from_secs(1)).await;
        }
    });

    // --- Monitor Receiver Task ---
    let mut monitor_recv_channel = handles.systemmonitorlink_monitorreceiverhandle;
    tokio::spawn(async move {
        while let Some(status) = monitor_recv_channel.recv().await {
            println!("[MonitorReceiver] Got status: '{}'", status);
        }
        println!("[MonitorReceiver] Monitor channel closed.");
    });

    // Let tasks run for a while
    println!("Tasks started. Running for 5 seconds...");
    sleep(Duration::from_secs(5)).await;
    println!("Example finished.");

    Ok(())
}
