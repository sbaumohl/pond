use std::sync::{atomic::AtomicBool, Arc};

use calloop::LoopHandle;
use smithay::{
    backend::{
        renderer::utils::on_commit_buffer_handler, session::libseat::LibSeatSession, x11::Window,
    },
    desktop::{PopupManager, Space, Window},
    input::SeatState,
    wayland::{
        buffer::BufferHandler,
        compositor::{
            get_parent, is_sync_subsurface, CompositorClientState, CompositorHandler,
            CompositorState,
        },
        output::OutputManagerState,
        selection::data_device::DataDeviceState,
        shell::xdg::{XdgShellHandler, XdgShellState},
        shm::{ShmHandler, ShmState},
        socket::ListeningSocketSource,
    },
};
use wayland_backend::server::{ClientData, ClientId, DisconnectReason};
use wayland_client::protocol::wl_buffer;
use wayland_server::{Client, Display, DisplayHandle};

#[derive(Debug)]
pub struct PondState {
    pub socket_name: Option<String>,
    pub running: Arc<AtomicBool>,
    pub loop_handle: LoopHandle<'static, PondState>,

    // data fields (seperate into generic later)
    pub session: LibSeatSession,
    pub display_handle: DisplayHandle,
    pub keyboards: Vec<smithay::reexports::input::Device>,

    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<PondState>,
    pub data_device_state: DataDeviceState,
    pub popups: PopupManager,
    pub space: Space<smithay::desktop::Window>,
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
        handle: LoopHandle<'static, PondState>,
        display: Display<PondState>,
        session: LibSeatSession,
    ) -> PondState {
        let source = ListeningSocketSource::new_auto().unwrap();
        let socket_name = source.socket_name().to_string_lossy().into_owned();

        PondState {
            socket_name: Some(socket_name),
            running: Arc::new(AtomicBool::new(true)),
            loop_handle: handle,
            session,
            display_handle: display.handle(),
            keyboards: Vec::with_capacity(3),
            compositor_state: CompositorState::new(&display.handle()),
            space: Space::default(),
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
