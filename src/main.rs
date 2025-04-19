use std::time::Duration;

use calloop::EventLoop;
use smithay::{
    backend::{
        input::InputEvent,
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        session::{libseat::LibSeatSession, Session},
        udev::UdevBackend,
    },
    reexports::input::Libinput,
};
use wayland_server::Display;

mod pondcore;

struct Data {}

// https://wayland-book.com/introduction.html
// https://github.com/Smithay/smithay/blob/master/anvil/src/udev.rs

fn main() {
    let mut event_loop: EventLoop<Data> = EventLoop::try_new().unwrap();
    let display: Display<pondcore::Pond> = Display::new().unwrap();
    let dis_handle = display.handle();

    let (session, notifier) = match LibSeatSession::new() {
        Ok(ret) => ret,
        Err(err) => {
            println!("{}", err);
            panic!("Could not start a session!");
        }
    };

    let udev_backend = match UdevBackend::new(session.seat()) {
        Ok(ret) => ret,
        Err(err) => {
            // error!(error = ?err, "Failed to initialize udev backend");
            panic!("Failed to initialzie udev backend");
            return;
        }
    };

    // initialize libinput
    let mut libinput_context =
        Libinput::new_with_udev::<LibinputSessionInterface<LibSeatSession>>(session.clone().into());

    libinput_context.udev_assign_seat(&session.seat()).unwrap();
    let libinput_backend = LibinputInputBackend::new(libinput_context.clone());

    event_loop
        .handle()
        .insert_source(libinput_backend, move |mut event, _, data| {
            if let InputEvent::Keyboard { event } = event {
                println!("{:#?}", event);
            }
        })
        .unwrap();

    println!("BACKEND SETUP!");
    loop {
        match event_loop.dispatch(Duration::from_millis(16), &mut Data {}) {
            Ok(_) => {}
            Err(err) => {
                println!("Error! {}", err);
                break;
            }
        }
    }
}
