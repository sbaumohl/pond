use smithay::{
    delegate_compositor, delegate_data_device, delegate_output, delegate_seat, delegate_shm,
    delegate_xdg_shell,
    input::SeatHandler,
    reexports::wayland_server::{
        Resource,
        protocol::{
            wl_buffer, wl_compositor::WlCompositor, wl_output::WlOutput, wl_shm::WlShm,
            wl_surface::WlSurface,
        },
    },
    wayland::{
        buffer::BufferHandler,
        output::{OutputHandler, WlOutputData},
        selection::{
            SelectionHandler,
            data_device::{
                ClientDndGrabHandler, DataDeviceHandler, ServerDnDGrab, ServerDndGrabHandler,
                set_data_device_focus,
            },
        },
        shm::ShmHandler,
    },
};

use crate::state::AppState;

impl ShmHandler for AppState {
    fn shm_state(&self) -> &smithay::wayland::shm::ShmState {
        &self.shm_state
    }
}

impl BufferHandler for AppState {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

delegate_seat!(AppState);

impl SeatHandler for AppState {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut smithay::input::SeatState<Self> {
        &mut self.seat_state
    }
    fn focus_changed(
        &mut self,
        seat: &smithay::input::Seat<Self>,
        _focused: Option<&Self::KeyboardFocus>,
    ) {
        let dh = &self.display_handle;
        let client = _focused.and_then(|s| dh.get_client(s.id()).ok());
        set_data_device_focus(dh, seat, client);
    }
    fn cursor_image(
        &mut self,
        _seat: &smithay::input::Seat<Self>,
        _image: smithay::input::pointer::CursorImageStatus,
    ) {
    }
    fn led_state_changed(
        &mut self,
        _seat: &smithay::input::Seat<Self>,
        _led_state: smithay::input::keyboard::LedState,
    ) {
    }
}

impl DataDeviceHandler for AppState {
    fn data_device_state(&self) -> &smithay::wayland::selection::data_device::DataDeviceState {
        &self.data_device_state
    }
}

impl ServerDndGrabHandler for AppState {}
impl ClientDndGrabHandler for AppState {}
impl SelectionHandler for AppState {
    type SelectionUserData = ();
}

impl OutputHandler for AppState {
    fn output_bound(&mut self, output: smithay::output::Output, _wl_output: WlOutput) {
        // println!("OUTPUT BOUND={:#?}", output);
    }
}

delegate_output!(AppState);
delegate_compositor!(AppState);
delegate_data_device!(AppState);
delegate_xdg_shell!(AppState);
delegate_shm!(AppState);
