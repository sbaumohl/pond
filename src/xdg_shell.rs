use smithay::{delegate_xdg_shell, desktop::Window, wayland::shell::xdg::XdgShellHandler};

use crate::PondState;

impl XdgShellHandler for PondState {
    fn xdg_shell_state(&mut self) -> &mut smithay::wayland::shell::xdg::XdgShellState {
        &mut self.xdg_shell_state
    }

    fn grab(
        &mut self,
        surface: smithay::wayland::shell::xdg::PopupSurface,
        seat: wayland_server::protocol::wl_seat::WlSeat,
        serial: smithay::utils::Serial,
    ) {
        println!("Grab!");
    }

    fn new_popup(
        &mut self,
        surface: smithay::wayland::shell::xdg::PopupSurface,
        positioner: smithay::wayland::shell::xdg::PositionerState,
    ) {
        println!("New Popup!");
    }

    fn new_client(&mut self, client: smithay::wayland::shell::xdg::ShellClient) {}

    fn client_pong(&mut self, client: smithay::wayland::shell::xdg::ShellClient) {}

    fn new_toplevel(&mut self, surface: smithay::wayland::shell::xdg::ToplevelSurface) {
        let window = Window::new_wayland_window(surface);
        self.space.map_element(window, (100, 100), true);
    }

    fn move_request(
        &mut self,
        surface: smithay::wayland::shell::xdg::ToplevelSurface,
        seat: wayland_server::protocol::wl_seat::WlSeat,
        serial: smithay::utils::Serial,
    ) {
    }

    fn ack_configure(
        &mut self,
        surface: wayland_server::protocol::wl_surface::WlSurface,
        configure: smithay::wayland::shell::xdg::Configure,
    ) {
    }

    fn title_changed(&mut self, surface: smithay::wayland::shell::xdg::ToplevelSurface) {}

    fn resize_request(
        &mut self,
        surface: smithay::wayland::shell::xdg::ToplevelSurface,
        seat: wayland_server::protocol::wl_seat::WlSeat,
        serial: smithay::utils::Serial,
        edges: wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge,
    ) {
    }

    fn app_id_changed(&mut self, surface: smithay::wayland::shell::xdg::ToplevelSurface) {}

    fn parent_changed(&mut self, surface: smithay::wayland::shell::xdg::ToplevelSurface) {}

    fn popup_destroyed(&mut self, surface: smithay::wayland::shell::xdg::PopupSurface) {}

    fn maximize_request(&mut self, surface: smithay::wayland::shell::xdg::ToplevelSurface) {}

    fn minimize_request(&mut self, surface: smithay::wayland::shell::xdg::ToplevelSurface) {}

    fn show_window_menu(
        &mut self,
        surface: smithay::wayland::shell::xdg::ToplevelSurface,
        seat: wayland_server::protocol::wl_seat::WlSeat,
        serial: smithay::utils::Serial,
        location: smithay::utils::Point<i32, smithay::utils::Logical>,
    ) {
    }

    fn client_destroyed(&mut self, client: smithay::wayland::shell::xdg::ShellClient) {}

    fn unmaximize_request(&mut self, surface: smithay::wayland::shell::xdg::ToplevelSurface) {}

    fn fullscreen_request(
        &mut self,
        surface: smithay::wayland::shell::xdg::ToplevelSurface,
        output: Option<wayland_server::protocol::wl_output::WlOutput>,
    ) {
    }

    fn reposition_request(
        &mut self,
        surface: smithay::wayland::shell::xdg::PopupSurface,
        positioner: smithay::wayland::shell::xdg::PositionerState,
        token: u32,
    ) {
    }

    fn toplevel_destroyed(&mut self, surface: smithay::wayland::shell::xdg::ToplevelSurface) {}

    fn unfullscreen_request(&mut self, surface: smithay::wayland::shell::xdg::ToplevelSurface) {}
}

delegate_xdg_shell!(PondState);
