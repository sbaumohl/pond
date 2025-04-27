use std::time::Duration;

use calloop::EventLoop;
use smithay::{
    backend::{
        input::InputEvent,
        renderer::{
            damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement,
            gles::GlesRenderer,
        },
        session::{self, libseat::LibSeatSession, Session},
        winit::WinitEvent,
    },
    input::keyboard::KeyboardHandle,
    output::{Mode, Output, Subpixel},
    reexports::input::{Device, Libinput},
    utils::Rectangle,
    wayland::{
        compositor::CompositorClientState, security_context::SecurityContext,
        socket::ListeningSocketSource,
    },
};
use wayland_backend::server::{ClientId, DisconnectReason};
use wayland_server::{Display, DisplayHandle};

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

struct LoopData {
    pub data: PondState,
    pub handle: DisplayHandle,
}

fn main() {
    let mut event_loop: EventLoop<LoopData> = EventLoop::try_new().unwrap();
    let display: Display<PondState> = Display::new().unwrap();
    let dis_handle = display.handle();

    // let (session, notifier) = match LibSeatSession::new() {
    //     Ok(ret) => ret,
    //     Err(err) => {
    //         println!("{}", err);
    //         panic!("Could not start a session!");
    //     }
    // };

    let (mut backend, mut winit) = match smithay::backend::winit::init::<GlesRenderer>() {
        Ok(ret) => ret,
        Err(err) => {
            panic!("Could not initialize winit");
            return;
        }
    };

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
    output.change_current_state(
        Some(mode),
        Some(smithay::utils::Transform::Flipped180),
        None,
        Some((0, 0).into()),
    );
    output.set_preferred(mode);

    let _global = output.create_global::<PondState>(&dis_handle);

    let source = ListeningSocketSource::new_auto().unwrap();
    let socket_name = source.socket_name().to_string_lossy().into_owned();

    let mut state: PondState = PondState::init(&mut event_loop, &display);

    state.space.map_output(&output, (0, 0));

    let mut data: LoopData = LoopData {
        data: state,
        handle: display.handle(),
    };

    let mut damage_tracker = OutputDamageTracker::from_output(&output);
    event_loop
        .handle()
        .insert_source(winit, move |event, _, data| {
            let display = &mut data.handle;
            let state = &mut data.data;

            match event {
                WinitEvent::Resized { size, .. } => {
                    output.change_current_state(
                        Some(Mode {
                            size,
                            refresh: 60_000,
                        }),
                        None,
                        None,
                        None,
                    );
                }
                WinitEvent::Input(event) => state.process_input_event(event),
                WinitEvent::Redraw => {
                    let size = backend.window_size();
                    let damage = Rectangle::from_size(size);

                    backend.bind().unwrap();
                    smithay::desktop::space::render_output::<
                        _,
                        WaylandSurfaceRenderElement<GlesRenderer>,
                        _,
                        _,
                    >(
                        &output,
                        backend.renderer(),
                        1.0,
                        0,
                        [&state.space],
                        &[],
                        &mut damage_tracker,
                        [0.1, 0.1, 0.1, 1.0],
                    )
                    .unwrap();
                    backend.submit(Some(&[damage])).unwrap();

                    state.space.elements().for_each(|window| {
                        window.send_frame(
                            &output,
                            state.start_time.elapsed(),
                            Some(Duration::ZERO),
                            |_, _| Some(output.clone()),
                        )
                    });

                    state.space.refresh();
                    state.popups.cleanup();
                    let _ = display.flush_clients();

                    // Ask for redraw to schedule new frame.
                    backend.window().request_redraw();
                }
                WinitEvent::CloseRequested => {
                    state.loop_signal.stop();
                }
                _ => (),
            };
        });

    println!("BACKEND SETUP!");
    // while state.running.load(std::sync::atomic::Ordering::SeqCst) {
    //     match event_loop.dispatch(Duration::from_millis(16), &mut data) {
    //         Ok(_) => {}
    //         Err(err) => {
    //             println!("Error! {}", err);
    //             break;
    //         }
    //     }
    // }
    event_loop.run(None, &mut data, move |_| {});
}
