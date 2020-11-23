use async_std::task;
use paperplane::{Server, Event};

fn main() {
    let server = Server::new();

    task::block_on(async {
        server.listen("0.0.0.0:8000").await.unwrap();

        while let Some(event) = server.next().await {
            match event {
                Event::Connected(id) => println!("{} connected", id),
                Event::Disconnected(id) => println!("{} disconnected", id),
                Event::Kicked(id, reason) => println!("{} kicked: {}", id, reason),
                Event::Message(id, msg) => match msg.to_text() {
                    Ok("close") => {
                        println!("{} closed server listeners", id);
                        server.close().await.ok();
                    }
                    Ok("exit") => {
                        println!("{} exited", id);
                        server.kick(id, "exit").await.ok();
                    }
                    Ok("kickall") => {
                        println!("{} kicked all", id);
                        server.kick_all("kickall".into()).await.ok();
                    }
                    Ok("kickthem") => {
                        println!("{} kicked others", id);
                        server.kick_map(|conn_id| match conn_id == id {
                            true => None,
                            false => Some("kickthem".into()),
                        }).await.ok();
                    }
                    _ => println!("{} sent '{}'", id, msg),
                }
            }
        }
    });
}
