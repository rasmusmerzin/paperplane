use async_std::task;
use paperplane::{Server, Event, Message};
use std::time;

fn main() {
    let server = Server::new();

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

    task::block_on(async {
        server.listen("0.0.0.0:8000").await.unwrap();
        while let Some(event) = server.next().await {
            match event {
                Event::Message(id, msg) => server.send_map(|conn_id| match conn_id == id {
                    true => None,
                    false => Some(msg.clone()),
                }).await.ok(),
                _ => None
            };
        }
    });
}
