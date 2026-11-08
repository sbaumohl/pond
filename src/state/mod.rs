use std::{ffi::OsString, sync::Arc, time::Instant};

use smithay::{
    backend::renderer::utils::on_commit_buffer_handler,
    delegate_primary_selection,
    desktop::{PopupManager, Space, Window, WindowSurfaceType, layer_map_for_output},
    input::{Seat, SeatState},
    reexports::{
        calloop::{EventLoop, LoopSignal, PostAction, generic},
        wayland_server::{Display, DisplayHandle, protocol::wl_surface::WlSurface},
    },
    utils::{Logical, Point, SERIAL_COUNTER},
    wayland::{
        compositor::{
            CompositorClientState, CompositorHandler, CompositorState, get_parent,
            is_sync_subsurface,
        },
        selection::{data_device::DataDeviceState, primary_selection::PrimarySelectionState},
        shell::{wlr_layer::WlrLayerShellState, xdg::XdgShellState},
        shm::ShmState,
        socket::ListeningSocketSource,
    },
};
use wayland_backend::server::ClientData;

use crate::spacing::Workspace;

mod global_dispatch;
mod wlr_layer_shell_handler;
mod xdg_shell;

#[derive(Debug)]
pub struct AppState {
    pub start_time: Instant,
    compositor_state: CompositorState,
    pub display_handle: DisplayHandle,
    shm_state: ShmState,
    pub socket_name: OsString,
    seat_state: SeatState<Self>,
    pub seat: Seat<Self>,
    data_device_state: DataDeviceState,
    pub space: Space<Window>,
    pub xdg_shell_state: XdgShellState,
    pub popups: PopupManager,
    pub loop_signal: LoopSignal,

    pub layout_tree: Workspace<Window>,
    pub primary_selection_state: PrimarySelectionState,

    pub wlr_layer_shell_state: WlrLayerShellState,
}

impl AppState {
    pub fn new(event_loop: &mut EventLoop<AppState>, display: Display<Self>) -> Self {
        let start_time = std::time::Instant::now();
        let dh = display.handle();

        // shared memory state
        let shm_state = ShmState::new::<Self>(&dh, vec![]);
        let compositor_state = CompositorState::new::<Self>(&dh);
        let xdg_shell_state = XdgShellState::new::<Self>(&dh);

        let mut seat_state = SeatState::new();
        let mut seat: smithay::input::Seat<Self> = seat_state.new_wl_seat(&dh, "winit");

        let space = Space::default();

        // Notify clients that we have a keyboard, for the sake of the example we assume that keyboard is always present.
        // You may want to track keyboard hot-plug in real compositor.
        seat.add_keyboard(Default::default(), 200, 25).unwrap();

        // Notify clients that we have a pointer (mouse)
        // Here we assume that there is always pointer plugged in
        seat.add_pointer();

        Self {
            popups: PopupManager::default(),
            xdg_shell_state,
            start_time,
            space,
            seat,
            seat_state,
            data_device_state: DataDeviceState::new::<Self>(&dh),
            compositor_state,
            shm_state,
            socket_name: Self::init_wayland_socket(event_loop, display),
            loop_signal: event_loop.get_signal(),
            layout_tree: Workspace::new(),
            primary_selection_state: PrimarySelectionState::new::<Self>(&dh),
            wlr_layer_shell_state: WlrLayerShellState::new::<Self>(&dh),
            display_handle: dh,
        }
    }

    fn init_wayland_socket(event_loop: &mut EventLoop<Self>, display: Display<Self>) -> OsString {
        let listening_socket =
            ListeningSocketSource::new_auto().expect("Could not initialize listening socket!");

        let socket_name: OsString = listening_socket.socket_name().to_os_string();

        // this routes new clients to create their own AppClientState struct
        let _ =
            event_loop
                .handle()
                .insert_source(listening_socket, move |client_stream, _, state| {
                    state
                        .display_handle
                        .insert_client(client_stream, Arc::new(AppClientState::default()))
                        .unwrap();
                });

        // This routes call backs from clients to the right dispatches
        let _ = event_loop.handle().insert_source(
            generic::Generic::new(
                display,
                smithay::reexports::calloop::Interest::READ,
                smithay::reexports::calloop::Mode::Level,
            ),
            |_, display, state| {
                unsafe {
                    display.get_mut().dispatch_clients(state).unwrap();
                }
                Ok(PostAction::Continue)
            },
        );

        socket_name
    }

    pub fn apply_layouts(&mut self) {
        self.layout_tree.apply_layout(&mut self.space);
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

#[derive(Debug, Default)]
pub struct AppClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for AppClientState {
    fn initialized(&self, _client_id: smithay::reexports::wayland_server::backend::ClientId) {}

    fn disconnected(
        &self,
        _client_id: smithay::reexports::wayland_server::backend::ClientId,
        _reason: smithay::reexports::wayland_server::backend::DisconnectReason,
    ) {
    }
}

impl CompositorHandler for AppState {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(
        &self,
        client: &'a smithay::reexports::wayland_server::Client,
    ) -> &'a CompositorClientState {
        &client
            .get_data::<AppClientState>()
            .unwrap()
            .compositor_state
    }

    fn commit(
        &mut self,
        surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
    ) {
        on_commit_buffer_handler::<Self>(surface);

        if !is_sync_subsurface(surface) {
            let mut root = surface.clone();

            while let Some(parent) = get_parent(&root) {
                root = parent
            }

            if let Some(window) = self
                .space
                .elements()
                .find(|w| w.toplevel().unwrap().wl_surface() == &root)
            {
                window.on_commit();
            }
        }

        let mapped_output = self
            .space
            .outputs()
            .find(|o| {
                let map = layer_map_for_output(o);
                map.layer_for_surface(surface, WindowSurfaceType::TOPLEVEL)
                    .is_some()
            })
            .cloned();

        if let Some(tl_output) = mapped_output {
            let mut map = layer_map_for_output(&tl_output);
            map.arrange();

            let layer = map
                .layer_for_surface(surface, WindowSurfaceType::TOPLEVEL)
                .unwrap();
            layer.layer_surface().send_configure();

            if layer.can_receive_keyboard_focus() {
                self.seat.get_keyboard().unwrap().set_focus(
                    self,
                    Some(layer.wl_surface().clone()),
                    SERIAL_COUNTER.next_serial(),
                );
            }
        }

        // layer_map_for_output();

        xdg_shell::handle_commit(&mut self.popups, &self.space, surface);
    }
}
