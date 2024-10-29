use futures_util::{SinkExt, StreamExt};
use log::*;
use std::{net::SocketAddr, time::Duration};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{
    accept_async,
    tungstenite::{Error, Message, Result},
};

use refimage::DynamicImageOwned;
use image::open;
use gencam_packet::GenCamPacket;

async fn load_and_transmit_debug_image(path: &str, i: i32) -> Vec<u8> {
    // Load the image as a DynamicImage.
    let img = open(path).expect("Could not load image");

    // This allows different image data to be sent using the same image. The hue is adjusted as per the passed i argument.
    let img = img.huerotate(90 * i);

    // Converts the DynamicImage to DynamicImageOwned.
    let img = DynamicImageOwned::try_from(img).expect("Could not convert image.");

    // Create a new GenCamPacket with the image data.
    let pkt = GenCamPacket::image(img.as_raw_u8().to_vec(), 64, 64);
    
    // Set msg to the serialized pkt.
    serde_json::to_vec(&pkt).unwrap()
}

async fn accept_connection(peer: SocketAddr, stream: TcpStream) {
    if let Err(e) = handle_connection(peer, stream).await {
        match e {
            Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => (),
            err => error!("Error processing connection: {}", err),
        }
    }
}

async fn handle_connection(peer: SocketAddr, stream: TcpStream) -> Result<()> {
    let ws_stream = accept_async(stream).await.expect("Failed to accept");
    info!("New WebSocket connection: {}", peer);
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Sets the periodic interval.
    let mut interval = tokio::time::interval(Duration::from_millis(2500));

    // Echo incoming WebSocket messages and send a message periodically every second.
    let mut i = 0;
    loop {
        println!("Looping...");
        tokio::select! {
            
            msg = ws_receiver.next() => {
                match msg {
                    Some(msg) => {
                        let msg = msg?;
                        if msg.is_binary() { // Message is binary, lets look for our packet.
                            println!("Received binary message.");

                            let inbuf = msg.into_data();
                            let outbuf: Vec<u8>;
                            let pkt: GenCamPacket = serde_json::from_slice(&inbuf).unwrap();

                            match pkt {
                                GenCamPacket::ImageRequest { .. } => {
                                    println!("Received ImgReq packet.");

                                    i += 1;
                                    // load_and_transmit_debug_image("res/test_image_1.png", &mut websocket, i);
                                    outbuf = load_and_transmit_debug_image(&format!("res/test_{}.png", i%10), i).await;
                                    println!("Responded (ImgReq #{i})");
                                },
                                _ => {
                                    println!("Received an unhandled packet type. Replying with Ack.");

                                    // Construct an Ack packet.
                                    let pkt = GenCamPacket::ack();
                                    // Set outbuf to serialized pkt.
                                    outbuf = serde_json::to_vec(&pkt).unwrap();
                                },
                            }

                            println!("TRANSMITTING {} BYTES.", outbuf.len());
                            ws_sender.send(Message::Binary(outbuf)).await?;

                        } else if msg.is_close() {
                            println!("Received close message.");
                            break;
                        } else {
                            println!("Received an unhandled message type.");
                            warn!("Unexpected message type.");
                        }
                    }
                    None => break,
                }
            }
            _ = interval.tick() => { // Periodically sends this message (right now 1 Hz).
                println!("Transmitting an image packet.");
                i += 1;
                let outbuf = load_and_transmit_debug_image(&format!("res/test_{}.png", i%10), i).await;
                println!("TRANSMITTING {} BYTES.", outbuf.len());
                ws_sender.send(Message::Binary(outbuf)).await?;
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let addr = "127.0.0.1:9001";
    let listener = TcpListener::bind(&addr).await.expect("Can't listen");
    info!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let peer = stream.peer_addr().expect("connected streams should have a peer address");
        info!("Peer address: {}", peer);

        tokio::spawn(accept_connection(peer, stream));
    }
}