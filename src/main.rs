use std::net::UdpSocket;

use mdrc_pacbot_util::gui;

fn main() {
    let socket = UdpSocket::bind("0.0.0.0:20001").expect("couldn't bind to address");
    println!("bound {socket:?}");
    // socket
    //     .connect("127.0.0.1:8080")
    //     .expect("connect function failed");

    let mut buf = [0; 100];
    loop {
        let (len, from_addr) = socket.recv_from(&mut buf).expect("couldn't recv message");
        let message = &buf[..len];
        println!("got recv: len={len}, from_addr={from_addr}, data={message:?}");
        if let Ok(string) = std::str::from_utf8(message) {
            println!("   from_utf8: {string:?}");
        }
        if let Ok(four) = message.try_into() {
            println!("   as i16: {:?}", i16::from_le_bytes(four));
        }

        // let message = &std::f32::consts::PI.to_le_bytes();

        socket
            .send_to(message, from_addr)
            .expect("couldn't send message");
        println!("sent back");
    }

    return;

    // network::start_network_thread();
    gui::run_gui();
}
