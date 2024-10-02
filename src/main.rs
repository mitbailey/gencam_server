#![allow(clippy::unwrap_used, clippy::disallowed_methods)] // We are just testing here.

use std::{net::TcpListener, thread::spawn};
use refimage::DynamicImageOwned;
use image::open;

fn load_and_transmit_image(path: &str, websocket: &mut tungstenite::WebSocket<std::net::TcpStream>) {
    let img = open(path).expect("Could not load image");
    let img = DynamicImageOwned::try_from(img).expect("Could not convert image");
    websocket.send(img.as_raw_u8().into()).expect("Could not send image");
}

fn main() {
    let bind_addr = "127.0.0.1:9001";
    let server = TcpListener::bind(bind_addr).unwrap();
    eprintln!("Listening on: ws://{bind_addr}");
    for stream in server.incoming() {
        spawn(move || {
            let mut websocket = tungstenite::accept(stream.unwrap()).unwrap();
            eprintln!("New client connected");
            while let Ok(msg) = websocket.read() {
                // We do not want to send back ping/pong messages.
                if msg.is_binary() || msg.is_text() {
                    if let Err(err) = websocket.send(msg.clone()) {
                        eprintln!("Error sending message: {err}");
                        break;
                    } else if msg.is_text() && msg.to_text().unwrap() == "send test image" {
                        load_and_transmit_image("res/test_image_1.png", &mut websocket);   
                        eprintln!("Responded (send test image): {msg}");
                    } else {
                        eprintln!("Responded: {msg}");
                    }
                } else {
                    eprintln!("Message received not text or binary.");
                }
            }
            eprintln!("Client left.");
        });
    }
}
