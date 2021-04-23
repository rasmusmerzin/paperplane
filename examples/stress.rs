use async_std::task;
use paperplane::tungstenite::Message;
use paperplane::{Event, Server};

fn main() {
    let server = Server::new(10);

    task::block_on(async {
        server.listen("0.0.0.0:8000").await.unwrap();

        while let Some(event) = server.next::<Message>().await {
            task::spawn(async move {
                match event {
                    Event::Message(id, msg) => {
                        println!(
                            "{}: {}",
                            id,
                            match msg.len() > 32 {
                                true => format!("[#{}]", msg.len()),
                                false => msg.into_text().unwrap(),
                            }
                        );
                    }
                    _ => eprintln!("{:?}", event),
                }
            });
        }
    });
}
