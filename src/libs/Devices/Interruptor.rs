use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/**
 * Start a new Interruptor thread with the given action
 *
 * The Interruptor thread will fire an interrupt every time the
 * associated closure returns true. The thread will then sleep for 'duration'
 * 
 * This behaviour loops infinitely
 * 
 * ARGS:
 * 
 * name: The name for the Interruptor
 * 
 * duration: The firing speed of the interruptor, as Duration
 *
 * ch_send: Sender mpsc to Core's internal receiver
 * 
 * action: The function determining when to fire the interrupt. Fn() -> bool + Send + 'static
*/
pub fn new<FuncTyp>(name: &'static str,duration: Duration, ch_send: &mpsc::Sender<u32>, verbose: bool, action: FuncTyp) -> std::thread::JoinHandle<()>
    where FuncTyp: Fn() -> bool + Send + 'static
{

    let ch_send = ch_send.clone();

    if verbose { println!("[{}]: Spawned interruptor with timeout {:?}",name.to_uppercase(),duration); }

    thread::Builder::new().name(name.to_string()).spawn(move || {
        
        loop {
            if action() { 

                if verbose {
                    println!("[{}]: awakened after {:?}, sending interrupt",name.to_uppercase(),duration);
                }

                match ch_send.send(1) {
                    Ok(_) => {}
                    Err(_) => {println!("[{}]: CORE channel unavailable: CORE assumed dead, closing", name.to_uppercase()); break;}
                }

            }

            thread::sleep(duration);
        }

    }).unwrap()
}

/**
 * Start a new Interruptor thread, with the default implementation for a clock
 *
 * ARGS:
 * 
 * name: The name for the Interruptor
 *
 * resolution: The firing speed of the interruptor, in Duration
 *
 * ch_send: Sender mpsc to Core's internal receiver
*/
pub fn new_default(name: &'static str,resolution: Duration, ch_send: &mpsc::Sender<u32>, verbose: bool) -> std::thread::JoinHandle<()> {

    new(name, resolution, ch_send, verbose, move || {
        true
    })

}

#[test]
fn triggers() {

    let (send,recv) = mpsc::channel();

    new_default("TEST", Duration::new(0, 2), &send, true);

    thread::sleep_ms(1000);
    assert_eq!(recv.recv().unwrap(), 1);
    thread::sleep_ms(200);
    assert_eq!(recv.recv().unwrap(), 1);

}

#[test]
fn FnPassing() {
    
    let (send,recv) = mpsc::channel();

    new("CUSTOMFN", Duration::new(0,5), &send, true, move || 4 % 2 == 0 );

    assert_eq!(recv.recv().unwrap(), 1);
    assert_eq!(recv.recv().unwrap(), 1);
}


