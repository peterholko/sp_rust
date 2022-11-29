// use tokio::task;
// use tokio::time::{sleep, Duration};
/// use tokio_test::assert_ok;
use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*; // Used for writing assertions
use std::process::Command;
use std::{thread, time};

use tungstenite::{connect, Message};
use url::Url;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "cmd")]
enum NetworkPacket {
    #[serde(rename = "login")]
    Login { username: String, password: String },
    #[serde(rename = "select_class")]
    SelectedClass { classname: String },
    #[serde(rename = "move_unit")]
    Move { x: i32, y: i32 },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "packet")]
enum ResponsePacket {
    #[serde(rename = "select_class")]
    SelectClass {
        player: u32,
    },
    #[serde(rename = "info_select_class")]
    InfoSelectClass {
        result: String,
    },
    PlayerMoved {
        player_id: i32,
        x: i32,
        y: i32,
    },
    Ok,
    Error {
        errmsg: String,
    },
}

#[test]
fn new_player() -> Result<(), Box<dyn std::error::Error>> {
    let foo = Command::new("/Users/peterholko/ph/test/siege_perilous/target/debug/siege_perilous")
        .spawn();

    let time = time::Duration::from_millis(2000);
    thread::sleep(time);

    let (mut socket, response) =
        connect(Url::parse("ws://127.0.0.1:9002").unwrap()).expect("Can't connect");

    println!("Connected to the server");
    println!("Response HTTP code: {}", response.status());
    println!("Response contains the following headers:");
    for (ref header, _value) in response.headers() {
        println!("* {}", header);
    }

    let login = NetworkPacket::Login {
        username: "joe".to_string(),
        password: "123123".to_string(),
    };

    let message = serde_json::to_string(&login).unwrap();

    socket.write_message(Message::Text(message)).unwrap();

    let msg = socket.read_message().expect("Error reading message");
    println!("Received: {}", msg);

    let res_packet: ResponsePacket = match serde_json::from_str(msg.to_text().unwrap()) {
        Ok(packet) => {
            match packet {
                ResponsePacket::SelectClass { player } => {
                    println!("Client (SelectClass) => player: {:?}", player);

                    let select_class = NetworkPacket::SelectedClass {
                        classname: "warrior".to_string(),
                    };

                    let message = serde_json::to_string(&select_class).unwrap();

                    socket.write_message(Message::Text(message)).unwrap();

                    let msg = socket.read_message().expect("Error reading message");
                    println!("Received: {}", msg);

                    let expected = r#"{"packet":"info_select_class","result":"success"}"#;

                    assert_eq!(expected, msg.into_text().unwrap());

                    //Return packet
                    ResponsePacket::Ok
                }
                _ => ResponsePacket::Error {
                    errmsg: "Unknown packet".to_owned(),
                },
            }
        }

        Err(_) => ResponsePacket::Error {
            errmsg: "Unknown packet".to_owned(),
        },
    };

    println!("res_packet: {:?}", res_packet);

    Ok(())
}
