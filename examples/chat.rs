use async_std::task;
use paperplane::tungstenite::Message;
use paperplane::{Event, Server};

fn main() {
    let server = Server::new(10);

    task::block_on(async {
        server.listen("0.0.0.0:8000").await.unwrap();

        while let Some(event) = server.next().await {
            let server = server.clone();
            task::spawn(async move {
                match event {
                    Event::Connected(id) => server
                        .send_map(|conn_id| match conn_id == id {
                            true => Some(Message::Text("Welcome!".into())),
                            false => Some(Message::Text(format!("{} connected", id))),
                        })
                        .await
                        .ok(),
                    Event::Disconnected(id) => server
                        .send_all(Message::Text(format!("{} disconnected", id)))
                        .await
                        .ok(),
                    Event::Kicked(id, reason) => server
                        .send_all(Message::Text(format!("{} kicked: '{}'", id, reason)))
                        .await
                        .ok(),
                    Event::Message(id, msg) => match msg.to_text() {
                        Ok("close") => server.close().await.ok(),
                        Ok("exit") => server.kick(id, "exit").await.ok(),
                        Ok("kickall") => server.kick_all("kickall".into()).await.ok(),
                        Ok("kickthem") => server
                            .kick_map(|conn_id| match conn_id == id {
                                true => None,
                                false => Some("kickthem".into()),
                            })
                            .await
                            .ok(),
                        _ => server
                            .send_map(|conn_id| match conn_id == id {
                                true => None,
                                false => Some(Message::Text(format!("{}: {}", id, msg))),
                            })
                            .await
                            .ok(),
                    },
                }
            });
        }
    });
}
