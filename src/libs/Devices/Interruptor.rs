use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/**
 * Start a new Interruptor thread with the given action
 *
 * The Interruptor thread will fire an interrupt every time the
 * associated closure returns true. The thread will then sleep for 'duration'
 *
 * This behaviour loops infinitely until the ch_open_flag is set to false
 *
 * ARGS:
 *
 * name: The name for the Interruptor
 *
 * duration: The firing speed of the interruptor, as Duration
 *
 * ch_send: Sender mpsc to Core's internal receiver
 *
 * ch_open_flag: Flag to signal closing of the channel
 *
 * action: The function determining when to fire the interrupt. Fn() -> bool + Send + 'static
*/
pub fn new<FuncTyp>(
    name: &'static str,
    duration: Duration,
    ch_send: &mpsc::Sender<u32>,
    ch_open_flag: Arc<AtomicBool>,
    verbose: bool,
    action: FuncTyp,
) -> std::thread::JoinHandle<()>
where
    FuncTyp: Fn() -> bool + Send + 'static,
{
    let ch_send = ch_send.clone();

    if verbose {
        println!(
            "[{}]: Spawned interruptor with timeout {:?}",
            name.to_uppercase(),
            duration
        );
    }

    thread::Builder::new()
        .name(name.to_string())
        .spawn(move || {
            loop {
                // flag is false -> close thread
                if !ch_open_flag.load(Ordering::Relaxed) {
                    if verbose {
                        println!("[{}]: Channel close flag set: closing", name.to_uppercase());
                    }
                    break;
                }

                if action() {
                    if verbose {
                        println!(
                            "[{}]: awakened after {:?}, sending interrupt",
                            name.to_uppercase(),
                            duration
                        );
                    }

                    match ch_send.send(1) {
                        Ok(_) => {}
                        Err(_) => {
                            if verbose {
                                println!(
                                    "[{}]: CORE channel unavailable: CORE assumed dead, closing",
                                    name.to_uppercase()
                                );
                            }
                            break;
                        }
                    }
                }

                thread::sleep(duration);
            }
        })
        .unwrap()
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
 *
 * ch_open_flag: Flag to signal closing of the channel
*/
pub fn new_default(
    name: &'static str,
    resolution: Duration,
    ch_send: &mpsc::Sender<u32>,
    ch_open_flag: Arc<AtomicBool>,
    verbose: bool,
) -> std::thread::JoinHandle<()> {
    new(
        name,
        resolution,
        ch_send,
        ch_open_flag,
        verbose,
        move || true,
    )
}

#[test]
fn triggers() {
    let (send, recv) = mpsc::channel();

    new_default(
        "TEST",
        Duration::new(0, 2),
        &send,
        Arc::new(AtomicBool::new(true)),
        true,
    );

    thread::sleep(Duration::new(0, 1));
    assert_eq!(recv.recv().unwrap(), 1);
    thread::sleep(Duration::new(0, 1));
    assert_eq!(recv.recv().unwrap(), 1);
}

#[test]
fn fn_passing() {
    let (send, recv) = mpsc::channel();

    new(
        "CUSTOMFN",
        Duration::new(0, 1),
        &send,
        Arc::new(AtomicBool::new(true)),
        true,
        move || 4 % 2 == 0,
    );

    assert_eq!(recv.recv().unwrap(), 1);
    assert_ne!(recv.recv().unwrap(), 0);
}

#[test]
fn closing_signal() {
    let (send, recv) = mpsc::channel();

    new_default(
        "TEST",
        Duration::new(0, 1),
        &send,
        Arc::new(AtomicBool::new(false)),
        true,
    );

    //is_err: The AtomicBool was passed as false so the thread will finish as soon as it is started.
    //The channel is dropped and therefore closed, recv from a closed channel returns Err
    assert!(recv.recv_timeout(Duration::new(0, 2)).is_err());
}
