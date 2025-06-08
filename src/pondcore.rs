use std::{
    collections::HashSet,
    env,
    process::Command,
    sync::{atomic::AtomicBool, Arc},
};

use calloop::{EventLoop, LoopSignal};
use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputBackend, InputEvent,
        KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
    },
    desktop::{PopupManager, Space, WindowSurfaceType},
    input::{
        keyboard::{FilterResult, Keysym},
        pointer::{AxisFrame, ButtonEvent, MotionEvent},
        Seat, SeatState,
    },
    utils::{Logical, Point, SERIAL_COUNTER},
    wayland::{
        compositor::CompositorState, output::OutputManagerState,
        selection::data_device::DataDeviceState, shell::xdg::XdgShellState, shm::ShmState,
        socket::ListeningSocketSource,
    },
};
use wayland_server::{protocol::wl_surface::WlSurface, Display, DisplayHandle};

use crate::{ClientState, LoopData};

#[derive(Debug)]
pub struct PondState {
    pub socket_name: String,
    pub running: Arc<AtomicBool>,

    // data fields (seperate into generic later)
    // pub session: LibSeatSession,
    pub display_handle: DisplayHandle,
    pub keyboards: Vec<smithay::reexports::input::Device>,
    pub pressed_keys: HashSet<Keysym>,

    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<PondState>,
    pub data_device_state: DataDeviceState,
    pub popups: PopupManager,
    pub space: Space<smithay::desktop::Window>,

    pub seat: Seat<Self>,
    pub start_time: std::time::Instant,
    pub loop_signal: LoopSignal,
}

impl PondState {
    pub fn init(
        event_loop: &mut EventLoop<LoopData>,
        display: &Display<PondState>,
        // session: LibSeatSession,
    ) -> Self {
        let source = ListeningSocketSource::new_auto().unwrap();
        let socket_name = source.socket_name().to_string_lossy().into_owned();

        let shm_state = ShmState::new::<Self>(&display.handle(), vec![]);
        let mut seat_state = SeatState::new();
        let mut seat = seat_state.new_wl_seat(&display.handle(), "winit");
        seat.add_keyboard(Default::default(), 200, 25).unwrap();

        let d_handle = display.handle();

        let _ = event_loop
            .handle()
            .insert_source(source, move |client_stream, _, state| {
                let _ = state
                    .handle
                    .insert_client(client_stream, Arc::new(ClientState::default()));
            });

        PondState {
            socket_name,
            running: Arc::new(AtomicBool::new(true)),
            // session,
            display_handle: display.handle(),
            keyboards: Vec::with_capacity(3),
            pressed_keys: HashSet::with_capacity(10),
            compositor_state: CompositorState::new::<Self>(&display.handle()),
            space: Space::default(),
            popups: PopupManager::default(),
            output_manager_state: OutputManagerState::new_with_xdg_output::<Self>(
                &display.handle(),
            ),
            shm_state,
            seat_state,
            data_device_state: DataDeviceState::new::<Self>(&display.handle()),
            seat,
            start_time: std::time::Instant::now(),
            loop_signal: event_loop.get_signal(),
            xdg_shell_state: XdgShellState::new::<Self>(&d_handle),
        }
    }

    pub fn surface_under(
        &self,
        pos: Point<f64, Logical>,
    ) -> Option<(WlSurface, Point<f64, Logical>)> {
        self.space
            .element_under(pos)
            .and_then(|(window, location)| {
                window
                    .surface_under(pos - location.to_f64(), WindowSurfaceType::ALL)
                    .map(|(s, p)| (s, (p + location).to_f64()))
            })
    }
}

impl PondState {
    pub fn process_input_event<I: InputBackend>(&mut self, event: InputEvent<I>) {
        match event {
            InputEvent::Keyboard { event, .. } => {
                let serial = SERIAL_COUNTER.next_serial();
                let time = Event::time_msec(&event);

                self.seat.get_keyboard().unwrap().input::<(), _>(
                    self,
                    event.key_code(),
                    event.state(),
                    serial,
                    time,
                    |state, modifiers_state, handler| {
                        match event.state() {
                            smithay::backend::input::KeyState::Pressed => {
                                state.pressed_keys.insert(handler.modified_sym());
                            }
                            smithay::backend::input::KeyState::Released => {
                                state.pressed_keys.remove(&handler.modified_sym());
                            }
                        }

                        if state.pressed_keys == HashSet::from_iter([Keysym::D, Keysym::Shift_L]) {
                            env::set_var("WAYLAND_DISPLAY", &state.socket_name);
                            match Command::new("foot").spawn() {
                                Ok(child) => {
                                    println!("CHILD: {}", child.id());
                                }
                                Err(e) => {
                                    panic!("Error spawning program!");
                                }
                            }
                            FilterResult::Intercept(())
                        } else {
                            FilterResult::Forward
                        }
                    },
                );
            }
            InputEvent::PointerMotion { .. } => {}
            InputEvent::PointerMotionAbsolute { event, .. } => {
                return;
                let output = self.space.outputs().next().unwrap();

                let output_geo = self.space.output_geometry(output).unwrap();

                let pos = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();

                let serial = SERIAL_COUNTER.next_serial();

                let pointer = self.seat.get_pointer().unwrap();

                let under = self.surface_under(pos);

                pointer.motion(
                    self,
                    under,
                    &MotionEvent {
                        location: pos,
                        serial,
                        time: event.time_msec(),
                    },
                );
                pointer.frame(self);
            }
            InputEvent::PointerButton { event, .. } => {
                return;
                let pointer = self.seat.get_pointer().unwrap();
                let keyboard = self.seat.get_keyboard().unwrap();

                let serial = SERIAL_COUNTER.next_serial();

                let button = event.button_code();

                let button_state = event.state();

                if ButtonState::Pressed == button_state && !pointer.is_grabbed() {
                    if let Some((window, _loc)) = self
                        .space
                        .element_under(pointer.current_location())
                        .map(|(w, l)| (w.clone(), l))
                    {
                        self.space.raise_element(&window, true);
                        keyboard.set_focus(
                            self,
                            Some(window.toplevel().unwrap().wl_surface().clone()),
                            serial,
                        );
                        self.space.elements().for_each(|window| {
                            window.toplevel().unwrap().send_pending_configure();
                        });
                    } else {
                        self.space.elements().for_each(|window| {
                            window.set_activated(false);
                            window.toplevel().unwrap().send_pending_configure();
                        });
                        keyboard.set_focus(self, Option::<WlSurface>::None, serial);
                    }
                };

                pointer.button(
                    self,
                    &ButtonEvent {
                        button,
                        state: button_state,
                        serial,
                        time: event.time_msec(),
                    },
                );
                pointer.frame(self);
            }
            InputEvent::PointerAxis { event, .. } => {
                return;
                let source = event.source();

                let horizontal_amount = event.amount(Axis::Horizontal).unwrap_or_else(|| {
                    event.amount_v120(Axis::Horizontal).unwrap_or(0.0) * 15.0 / 120.
                });
                let vertical_amount = event.amount(Axis::Vertical).unwrap_or_else(|| {
                    event.amount_v120(Axis::Vertical).unwrap_or(0.0) * 15.0 / 120.
                });
                let horizontal_amount_discrete = event.amount_v120(Axis::Horizontal);
                let vertical_amount_discrete = event.amount_v120(Axis::Vertical);

                let mut frame = AxisFrame::new(event.time_msec()).source(source);
                if horizontal_amount != 0.0 {
                    frame = frame.value(Axis::Horizontal, horizontal_amount);
                    if let Some(discrete) = horizontal_amount_discrete {
                        frame = frame.v120(Axis::Horizontal, discrete as i32);
                    }
                }
                if vertical_amount != 0.0 {
                    frame = frame.value(Axis::Vertical, vertical_amount);
                    if let Some(discrete) = vertical_amount_discrete {
                        frame = frame.v120(Axis::Vertical, discrete as i32);
                    }
                }

                if source == AxisSource::Finger {
                    if event.amount(Axis::Horizontal) == Some(0.0) {
                        frame = frame.stop(Axis::Horizontal);
                    }
                    if event.amount(Axis::Vertical) == Some(0.0) {
                        frame = frame.stop(Axis::Vertical);
                    }
                }

                let pointer = self.seat.get_pointer().unwrap();
                pointer.axis(self, frame);
                pointer.frame(self);
            }
            _ => {}
        }
    }
}
