use super::super::Definitions::Arch::INTERR_FLAG;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/**
 * Start a new Interruptor thread
 *
 * ARGS:
 *
 * timeout: The firing speed of the interruptor, in seconds
 *
 * ch_send: Sender mpsc to Core's internal receiver
*/
pub fn new_default(name: &'static str,timeout: u64, ch_send_r: &mpsc::Sender<u32>, verbose: bool) -> std::thread::JoinHandle<()> {

    let ch_send = ch_send_r.clone();

    if verbose { println!("[{}]: Spawned interruptor with timeout {}",name.to_uppercase(),timeout); }

    return thread::Builder::new().name(name.to_string()).spawn(move || {

        loop {
            thread::sleep(Duration::new(timeout,0));

            if verbose {
                println!("[{}]: awakened after {}s, sending interrupt",name.to_uppercase(),timeout);
            }
            

            ch_send.send(1).expect( &(format!("[{}]: CORE channel unavailable", name.to_uppercase() )) );
        }

    }).unwrap();

}

#[test]
fn triggers() {
    use super::super::Core;

    let c = Core::new(true);
    thread::sleep(Duration::new(2,0));
}


