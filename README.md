<div align="center">
    <h1>
        <img alt="logo" src="./logo/logo.svg" />
        <br />
        Paperplane
    </h1>
    <a href="https://gitlab.com/rasmusmerzin/paperplane/-/commits/master">
        <img alt="build" src="https://img.shields.io/gitlab/pipeline/rasmusmerzin/paperplane/master" />
    </a>
    <a href="https://crates.io/crates/paperplane">
        <img alt="license" src="https://img.shields.io/crates/l/paperplane" />
    </a>
    <a href="https://crates.io/crates/paperplane">
        <img alt="version" src="https://img.shields.io/crates/v/paperplane" />
    </a>
    <a href="https://docs.rs/paperplane">
        <img alt="documentation" src="https://img.shields.io/badge/docs.rs-paperplane-blue"/>
    </a>
    <p>
        WebSocket library which utilizes
        <a href="https://crates.io/crates/async-std">async-std</a>,
        <a href="https://crates.io/crates/async-tungstenite">async-tungstenite</a> &
        <a href="https://crates.io/crates/futures">futures</a>.
        <br />
    </p>
</div>

## Example

```rust
use async_std::task;
use paperplane::tungstenite::Message;
use paperplane::Server;
use std::time;

fn main() {
    let server = Server::new(10);

    // Send count to all connected clients each second
    {
        let server = server.clone();
        task::spawn(async move {
            let mut count = 0usize;
            loop {
                task::sleep(time::Duration::from_secs(1)).await;
                server.send(None, count.to_string()).await.ok();
                count += 1;
            }
        });
    }

    // Print messages sent by clients
    task::block_on(async {
        server.listen("0.0.0.0:8000").await.unwrap();
        while let Some(event) = server.next::<Message>().await {
            println!("{:?}", event);
        }
    });
}
```

Simple duplex communication example.
See [examples folder](./examples) for more examples.
