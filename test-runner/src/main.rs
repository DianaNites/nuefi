#![allow(dead_code, unused_imports, unused_variables)]
use std::{env::args, os::unix::net::UnixStream, process::Command, thread::sleep, time::Duration};

use qapi::{qmp, Qmp};

fn qmp() {
    // let socket_addr = args().nth(1).expect("argument: QMP socket path");
    let socket_addr = "../target/qmp.sock";
    let stream = UnixStream::connect(socket_addr).expect("failed to connect to socket");

    let mut qmp = Qmp::from_stream(&stream);

    let info = qmp.handshake().expect("handshake failed");
    println!("QMP info: {:#?}", info);

    let status = qmp.execute(&qmp::query_status {}).unwrap();
    println!("VCPU status: {:#?}", status);

    loop {
        qmp.nop().unwrap();
        for event in qmp.events() {
            println!("Got event: {:#?}", event);
        }

        sleep(Duration::from_secs(1));
    }
}

pub fn main() {
    qmp();
    // let cmd = Command::new("qemu-system-x86_64");
}
