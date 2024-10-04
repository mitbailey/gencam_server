#![deny(missing_docs)]
/*!
 * # GenCam Server
 * This crate is an example of the client-facing portion of a Generic Camera-compatible server. This example loads an image from the file system, serializes it, and sends it using a GenCamPacket. The server listens for incoming connections and responds to requests for images.
 */

use std::{net::TcpListener, thread::spawn};
use refimage::DynamicImageOwned;
use image::open;
use gencam_packet::{GenCamPacket, PacketType};

/// Loads a static image from path and sends it over websocket.
/// Packet is serialized using serde_json.
fn load_and_transmit_debug_image(path: &str, websocket: &mut tungstenite::WebSocket<std::net::TcpStream>, i: i32) {
    // Load the image as a DynamicImage.
    let img = open(path).expect("Could not load image");

    // This allows different image data to be sent using the same image. The hue is adjusted as per the passed i argument.
    let img = img.huerotate(90 * i);

    // Converts the DynamicImage to DynamicImageOwned.
    let img = DynamicImageOwned::try_from(img).expect("Could not convert image.");

    // Create a new GenCamPacket with the image data.
    let pkt = GenCamPacket::new(PacketType::Image, 0, 256, 256, Some(img.as_raw_u8().to_vec()));
    // Set msg to the serialized pkt.
    let msg = serde_json::to_vec(&pkt).unwrap();
    // Send the message.
    websocket.send(msg.into()).unwrap(); 
}

fn main() {
    // Counter to track how many images have been sent.
    let mut i = 0;

    // Set up websocket listening.
    let bind_addr = "127.0.0.1:9001";
    let server = TcpListener::bind(bind_addr).unwrap();
    eprintln!("Listening on: ws://{bind_addr}");

    // When we get an incoming client connection...
    for stream in server.incoming() {
        spawn(move || {
            let mut websocket = tungstenite::accept(stream.unwrap()).unwrap();
            eprintln!("New client connected!");

            // When we receive a message...
            while let Ok(msg) = websocket.read() {
                // Only do something if its a binary message that is a deserializable GenCamPacket.
                if let tungstenite::Message::Binary(data) = msg {
                    let pkt: GenCamPacket = serde_json::from_slice(&data).unwrap();
                    match pkt.packet_type {
                        // If we received an image request, load and transmit the debug image.
                        PacketType::ImgReq => {
                            i += 1;
                            load_and_transmit_debug_image("res/test_image_1.png", &mut websocket, i);
                            eprintln!("Responded (ImgReq #{i})");
                        },
                        // Right now, we do not handle any other packet types.
                        _ => {
                            let pkt = GenCamPacket::new(PacketType::Ack, 0, 0, 0, None);
                            // Set msg to serialized pkt.
                            let msg = serde_json::to_vec(&pkt).unwrap();
                            // Send
                            websocket.send(msg.into()).unwrap();  
                        },
                    }
                }
            }

            eprintln!("Client disconnected.");
        });
    }
}
