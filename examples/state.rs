use async_std::task;
use paperplane::tungstenite::Message;
use paperplane::{Event, Server};

fn main() {
    // create a new server with session type as optional string
    let server = Server::<Option<String>>::new(10);

    task::block_on(async {
        server.listen("0.0.0.0:8000").await.unwrap();

        while let Some(event) = server.next().await {
            let server = server.clone();
            task::spawn(async move {
                match event {
                    Event::Message(id, msg) => match msg.to_text() {
                        Ok("last") => {
                            // return session state
                            server
                                .send(
                                    id,
                                    Message::Text(format!(
                                        "{:?}",
                                        server.get_session(id).await.flatten()
                                    )),
                                )
                                .await
                                .ok();
                        }
                        _ => {
                            // set session state to the received message string
                            server.set_session(id, Some(msg.to_string())).await.ok();
                            // echo message to other connections
                            server
                                .send_map(|conn_id| match conn_id == id {
                                    true => None,
                                    false => Some(Message::Text(format!("{}: {}", id, msg))),
                                })
                                .await
                                .ok();
                        }
                    },
                    _ => println!("{:?}", event),
                }
            });
        }
    });
}
