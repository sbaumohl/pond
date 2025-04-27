use std::sync::{atomic::AtomicBool, Arc};

use calloop::{EventLoop, LoopHandle, LoopSignal};
use smithay::{
    backend::{
        input::{Event, InputBackend, InputEvent, KeyboardKeyEvent},
        renderer::utils::on_commit_buffer_handler,
    },
    delegate_compositor, delegate_data_device, delegate_output, delegate_seat, delegate_shm,
    desktop::{PopupManager, Space},
    input::{keyboard::FilterResult, Seat, SeatHandler, SeatState},
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::SERIAL_COUNTER,
    wayland::{
        buffer::BufferHandler,
        compositor::{
            get_parent, is_sync_subsurface, CompositorClientState, CompositorHandler,
            CompositorState,
        },
        output::{OutputHandler, OutputManagerState},
        selection::{
            data_device::{
                set_data_device_focus, ClientDndGrabHandler, DataDeviceHandler, DataDeviceState,
                ServerDndGrabHandler,
            },
            SelectionHandler,
        },
        shm::{ShmHandler, ShmPoolUserData, ShmState},
        socket::ListeningSocketSource,
    },
};
use wayland_backend::server::{ClientData, ClientId, DisconnectReason};
use wayland_server::{protocol::wl_buffer, Client, Display, DisplayHandle, Resource};

use crate::LoopData;

#[derive(Debug)]
pub struct PondState {
    pub socket_name: Option<String>,
    pub running: Arc<AtomicBool>,

    // data fields (seperate into generic later)
    // pub session: LibSeatSession,
    pub display_handle: DisplayHandle,
    pub keyboards: Vec<smithay::reexports::input::Device>,

    pub compositor_state: CompositorState,
    // pub xdg_shell_state: XdgShellState,
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

#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
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

        PondState {
            socket_name: Some(socket_name),
            running: Arc::new(AtomicBool::new(true)),
            // session,
            display_handle: display.handle(),
            keyboards: Vec::with_capacity(3),
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
        }
    }
}

impl CompositorHandler for PondState {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &wayland_server::protocol::wl_surface::WlSurface) {
        on_commit_buffer_handler::<Self>(surface);
        if !is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = get_parent(&root) {
                root = parent;
            }
            if let Some(window) = self
                .space
                .elements()
                .find(|w| w.toplevel().unwrap().wl_surface() == &root)
            {
                window.on_commit();
            }
        };

        // xdg_shell::handle_commit(&mut self.popups, &self.space, surface);
        // ::handle_commit(&mut self.space, surface);
    }
}

impl BufferHandler for PondState {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

impl ShmHandler for PondState {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

impl SeatHandler for PondState {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<PondState> {
        &mut self.seat_state
    }

    fn cursor_image(
        &mut self,
        _seat: &Seat<Self>,
        _image: smithay::input::pointer::CursorImageStatus,
    ) {
    }

    fn focus_changed(&mut self, seat: &Seat<Self>, focused: Option<&WlSurface>) {
        let dh = &self.display_handle;
        let client = focused.and_then(|s| dh.get_client(s.id()).ok());
        set_data_device_focus(dh, seat, client);
    }
}

impl SelectionHandler for PondState {
    type SelectionUserData = ();
}

impl DataDeviceHandler for PondState {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}

delegate_seat!(PondState);
delegate_data_device!(PondState);
impl OutputHandler for PondState {}
delegate_output!(PondState);

delegate_compositor!(PondState);
delegate_shm!(PondState);

impl ClientDndGrabHandler for PondState {}
impl ServerDndGrabHandler for PondState {}

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
                    |_, _, _| FilterResult::Forward,
                );
            }
            InputEvent::PointerMotion { .. } => {}
            // InputEvent::PointerMotionAbsolute { event, .. } => {
            //     let output = self.space.outputs().next().unwrap();
            //
            //     let output_geo = self.space.output_geometry(output).unwrap();
            //
            //     let pos = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();
            //
            //     let serial = SERIAL_COUNTER.next_serial();
            //
            //     let pointer = self.seat.get_pointer().unwrap();
            //
            //     let under = self.surface_under(pos);
            //
            //     pointer.motion(
            //         self,
            //         under,
            //         &MotionEvent {
            //             location: pos,
            //             serial,
            //             time: event.time_msec(),
            //         },
            //     );
            //     pointer.frame(self);
            // }
            // InputEvent::PointerButton { event, .. } => {
            //     let pointer = self.seat.get_pointer().unwrap();
            //     let keyboard = self.seat.get_keyboard().unwrap();
            //
            //     let serial = SERIAL_COUNTER.next_serial();
            //
            //     let button = event.button_code();
            //
            //     let button_state = event.state();
            //
            //     if ButtonState::Pressed == button_state && !pointer.is_grabbed() {
            //         if let Some((window, _loc)) = self
            //             .space
            //             .element_under(pointer.current_location())
            //             .map(|(w, l)| (w.clone(), l))
            //         {
            //             self.space.raise_element(&window, true);
            //             keyboard.set_focus(
            //                 self,
            //                 Some(window.toplevel().unwrap().wl_surface().clone()),
            //                 serial,
            //             );
            //             self.space.elements().for_each(|window| {
            //                 window.toplevel().unwrap().send_pending_configure();
            //             });
            //         } else {
            //             self.space.elements().for_each(|window| {
            //                 window.set_activated(false);
            //                 window.toplevel().unwrap().send_pending_configure();
            //             });
            //             keyboard.set_focus(self, Option::<WlSurface>::None, serial);
            //         }
            //     };
            //
            //     pointer.button(
            //         self,
            //         &ButtonEvent {
            //             button,
            //             state: button_state,
            //             serial,
            //             time: event.time_msec(),
            //         },
            //     );
            //     pointer.frame(self);
            // }
            // InputEvent::PointerAxis { event, .. } => {
            //     let source = event.source();
            //
            //     let horizontal_amount = event.amount(Axis::Horizontal).unwrap_or_else(|| {
            //         event.amount_v120(Axis::Horizontal).unwrap_or(0.0) * 15.0 / 120.
            //     });
            //     let vertical_amount = event.amount(Axis::Vertical).unwrap_or_else(|| {
            //         event.amount_v120(Axis::Vertical).unwrap_or(0.0) * 15.0 / 120.
            //     });
            //     let horizontal_amount_discrete = event.amount_v120(Axis::Horizontal);
            //     let vertical_amount_discrete = event.amount_v120(Axis::Vertical);
            //
            //     let mut frame = AxisFrame::new(event.time_msec()).source(source);
            //     if horizontal_amount != 0.0 {
            //         frame = frame.value(Axis::Horizontal, horizontal_amount);
            //         if let Some(discrete) = horizontal_amount_discrete {
            //             frame = frame.v120(Axis::Horizontal, discrete as i32);
            //         }
            //     }
            //     if vertical_amount != 0.0 {
            //         frame = frame.value(Axis::Vertical, vertical_amount);
            //         if let Some(discrete) = vertical_amount_discrete {
            //             frame = frame.v120(Axis::Vertical, discrete as i32);
            //         }
            //     }
            //
            //     if source == AxisSource::Finger {
            //         if event.amount(Axis::Horizontal) == Some(0.0) {
            //             frame = frame.stop(Axis::Horizontal);
            //         }
            //         if event.amount(Axis::Vertical) == Some(0.0) {
            //             frame = frame.stop(Axis::Vertical);
            //         }
            //     }
            //
            //     let pointer = self.seat.get_pointer().unwrap();
            //     pointer.axis(self, frame);
            //     pointer.frame(self);
            // }
            _ => {}
        }
    }
}
