use std::io::{Write};
use std::net::TcpStream;

use multiplayer_game_player_test::run;
fn main() {
    // let mut stream = TcpStream::connect("5.tcp.eu.ngrok.io:14302").unwrap();
    // let buffer: [u8; 8] = [0; 8];
    // loop {
    //     stream.write(&buffer).unwrap();
    // }
    pollster::block_on(run());
}
