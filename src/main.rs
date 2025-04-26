use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use calloop::{EventLoop, LoopHandle};
use smithay::{
    backend::{
        input::InputEvent,
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        renderer::gles::GlesRenderer,
        session::{self, libseat::LibSeatSession, Session},
        udev::UdevBackend,
    }, input::keyboard::KeyboardHandle, output::{Mode, Output, Subpixel}, reexports::input::{Device, Libinput}, wayland::{
        compositor::CompositorClientState, security_context::SecurityContext,
        socket::ListeningSocketSource,
    }
};
use wayland_backend::server::{ClientId, DisconnectReason};
use wayland_server::Display;

mod pondcore;
use pondcore::PondState;

#[derive(Debug, Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
    pub security_context: Option<SecurityContext>,
}
impl wayland_server::backend::ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}

// https://wayland-book.com/introduction.html
// https://github.com/Smithay/smithay/blob/master/anvil/src/udev.rs
//
// HERE:
// https://wayland-book.com/wayland-display/creation.html

fn main() {
    let mut event_loop: EventLoop<PondState> = EventLoop::try_new().unwrap();
    let display: Display<PondState> = Display::new().unwrap();
    let dis_handle = display.handle();

    let (session, notifier) = match LibSeatSession::new() {
        Ok(ret) => ret,
        Err(err) => {
            println!("{}", err);
            panic!("Could not start a session!");
        }
    };

    // let udev_backend = match UdevBackend::new(session.seat()) {
    //     Ok(ret) => ret,
    //     Err(err) => {
    //         // error!(error = ?err, "Failed to initialize udev backend");
    //         panic!("Failed to initialzie udev backend");
    //         return;
    //     }
    // };

    let (mut backend, mut winit) = match smithay::backend::winit::init::<GlesRenderer>() {
        Ok(ret) => ret,
        Err(err) => {
            panic!("Could not initialize winit");
            return;
        }
    }

    let mode = Mode {
        size: backend.window_size(),
        refresh: 60_000,
    };

    let output = Output::new(
        "winit".to_string(),
        smithay::output::PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "Smithay".into(),
            model: "Winit".into(),
        },
    );

    let _global = output.create_global::<PondState>(&dis_handle);

    let source = ListeningSocketSource::new_auto().unwrap();
    let socket_name = source.socket_name().to_string_lossy().into_owned();

    event_loop
        .handle()
        .insert_source(source, |client_stream, _, data| {
            data.display_handle
                .insert_client(client_stream, Arc::new(ClientState::default()));
        });

    // initialize libinput
    let mut libinput_context =
        Libinput::new_with_udev::<LibinputSessionInterface<LibSeatSession>>(session.clone().into());

    libinput_context.udev_assign_seat(&session.seat()).unwrap();
    let libinput_backend = LibinputInputBackend::new(libinput_context.clone());

    // handle input
    event_loop
        .handle()
        .insert_source(libinput_backend, move |mut event, _, data| {
            if let InputEvent::DeviceAdded { device } = &mut event {
                if device.has_capability(smithay::reexports::input::DeviceCapability::Keyboard) {
                    data.keyboards.push(device.clone());
                }
            } else if let InputEvent::DeviceRemoved { device } = &mut event {
                if device.has_capability(smithay::reexports::input::DeviceCapability::Keyboard) {
                    data.keyboards.retain(|x| x != device);
                }
            }

            if let InputEvent::Keyboard { event } = event {
                println!("{:#?}", event);
            }
        })
        .unwrap();

    // session management
    event_loop
        .handle()
        .insert_source(notifier, move |event, &mut (), data| match event {
            session::Event::PauseSession => {
                libinput_context.suspend();

                // for backend in data.backend_data.backends.values_mut() {
                //     backend.drm_output_manager.pause();
                //     backend.active_leases.clear();
                //     if let Some(lease_global) = backend.leasing_global.as_mut() {
                //         lease_global.suspend();
                //     }
                // }
            }
            session::Event::ActivateSession => {
                // for (node, backend) in data
                //     .backend_data
                //     .backends
                //     .iter_mut()
                //     .map(|(handle, backend)| (*handle, backend))
                // {
                //     // if we do not care about flicking (caused by modesetting) we could just
                //     // pass true for disable connectors here. this would make sure our drm
                //     // device is in a known state (all connectors and planes disabled).
                //     // but for demonstration we choose a more optimistic path by leaving the
                //     // state as is and assume it will just work. If this assumption fails
                //     // we will try to reset the state when trying to queue a frame.
                //     backend
                //         .drm_output_manager
                //         .activate(false)
                //         .expect("failed to activate drm backend");
                //     if let Some(lease_global) = backend.leasing_global.as_mut() {
                //         lease_global.resume::<Data>();
                //     }
                //     data.handle
                //         .insert_idle(move |data| data.render(node, None, data.clock.now()));
                // }
            }
        })
        .unwrap();

    let mut state: PondState = PondState::init(event_loop.handle(), display, session);

    println!("BACKEND SETUP!");
    while state.running.load(std::sync::atomic::Ordering::SeqCst) {
        match event_loop.dispatch(Duration::from_millis(16), &mut state) {
            Ok(_) => {}
            Err(err) => {
                println!("Error! {}", err);
                break;
            }
        }
    }
}
