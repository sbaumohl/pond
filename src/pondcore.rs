use smithay::wayland::compositor::{CompositorClientState, CompositorHandler, CompositorState};
use wayland_client::protocol::wl_surface::WlSurface;

pub struct Pond {}

// impl CompositorHandler for Pond {
//     fn compositor_state(&mut self) -> &mut CompositorState {
//         &mut self.compositor_state
//     }
//
//     fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
//         &client.get_data::<ClientState>().unwrap().compositor_state
//     }
//
//     fn commit(&mut self, surface: &WlSurface) {
//         on_commit_buffer_handler::<Self>(surface);
//     }
// }
