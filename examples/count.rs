use async_std::task;
use paperplane::tungstenite::Message;
use paperplane::Server;
use std::time;

fn main() {
    let server = Server::<()>::new(10);

    // Send count to all connected clients each second
    {
        let server = server.clone();
        task::spawn(async move {
            let mut count = 0usize;
            loop {
                task::sleep(time::Duration::from_secs(1)).await;
                server.send_all(Message::Text(count.to_string())).await.ok();
                count += 1;
            }
        });
    }

    // Print messages sent by clients
    task::block_on(async {
        server.listen("0.0.0.0:8000").await.unwrap();
        while let Some(event) = server.next().await {
            println!("{:?}", event);
        }
    });
}

// Could also be written this way
fn _main() {
    let server = Server::<()>::new(10);

    // Print messages sent by clients
    {
        let server = server.clone();
        task::spawn(async move {
            server.listen("0.0.0.0:8000").await.unwrap();
            while let Some(event) = server.next().await {
                println!("{:?}", event);
            }
        });
    }

    // Send count to all connected clients each second
    task::block_on(async {
        let mut count = 0usize;
        loop {
            task::sleep(time::Duration::from_secs(1)).await;
            server.send_all(Message::Text(count.to_string())).await.ok();
            count += 1;
        }
    });
}
