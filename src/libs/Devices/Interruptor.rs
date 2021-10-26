use super::super::Definitions::{INTERR_FLAG};
use std::sync::mpsc;
use std::thread;


pub fn new(timeout: u32, ch_send: mpsc::Sender<u32>) -> std::thread::JoinHandle<()> {

    return std::thread::spawn(move || {
        
        //get owned ref to interruptFlag

        loop {
            
            thread::sleep_ms(timeout);
            println!("[CLOCK]: awakened after {}ms, sending interrupt",timeout);
            ch_send.send(1);
        }
        
    })

}

#[test]
fn triggers() {
    use super::super::Core;

    let c = Core::new(true);
    thread::sleep_ms(3000);
}


