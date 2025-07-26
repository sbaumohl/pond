use std::{sync::Arc, time::Duration};

use calloop::{generic::Generic, EventLoop, Interest};
use smithay::{
    backend::{
        renderer::{
            damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement,
            gles::GlesRenderer,
        },
        winit::WinitEvent,
    },
    output::{Mode, Output, Subpixel},
    utils::Rectangle,
    wayland::{compositor::CompositorClientState, security_context::SecurityContext},
};
use wayland_backend::server::{ClientId, DisconnectReason};
use wayland_server::{Display, DisplayHandle};

mod config;
mod input;
mod pondcore;
mod state_handlers;
mod xdg_shell;
use pondcore::PondState;

use crate::config::schema::Configuration;

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
    let conf = Configuration::from_file("./example.toml".to_string());

    let mut event_loop: EventLoop<LoopData> = EventLoop::try_new().unwrap();
    let display: Display<PondState> = Display::new().unwrap();
    let dis_handle = display.handle();

    let (mut backend, mut winit) = match smithay::backend::winit::init::<GlesRenderer>() {
        Ok(ret) => ret,
        Err(err) => {
            panic!("Could not initialize winit");
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

    let mut state: PondState = PondState::init(&mut event_loop, &display, conf);

    state.space.map_output(&output, (0, 0));

    let mut data: LoopData = LoopData {
        data: state,
        handle: display.handle(),
    };

    let mut damage_tracker = OutputDamageTracker::from_output(&output);

    let _ = event_loop.handle().insert_source(
        Generic::new(display, Interest::READ, calloop::Mode::Level),
        |_, display, state| {
            unsafe {
                display.get_mut().dispatch_clients(&mut state.data).unwrap();
            }
            Ok(calloop::PostAction::Continue)
        },
    );

    let _ = event_loop
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

    event_loop.run(None, &mut data, move |_| {}).unwrap();
}
