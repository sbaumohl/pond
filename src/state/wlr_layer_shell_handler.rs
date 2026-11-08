use smithay::{
    delegate_layer_shell,
    desktop::{LayerSurface, layer_map_for_output},
    output::Output,
    reexports::wayland_server::protocol::wl_output::WlOutput,
    wayland::shell::wlr_layer::WlrLayerShellHandler,
};

use crate::state::AppState;

impl WlrLayerShellHandler for AppState {
    fn shell_state(&mut self) -> &mut smithay::wayland::shell::wlr_layer::WlrLayerShellState {
        &mut self.wlr_layer_shell_state
    }

    fn new_layer_surface(
        &mut self,
        surface: smithay::wayland::shell::wlr_layer::LayerSurface,
        output: Option<WlOutput>,
        _layer: smithay::wayland::shell::wlr_layer::Layer,
        namespace: String,
    ) {
        let wrapped_pure_output = output
            .as_ref()
            .and_then(Output::from_resource)
            .or_else(|| self.space.outputs().next().cloned());

        if wrapped_pure_output.is_none() {
            surface.send_close();
            return;
        }
        let unwrapped_output = wrapped_pure_output.unwrap();

        let mut map = layer_map_for_output(&unwrapped_output);
        let layer = LayerSurface::new(surface, namespace);
        map.map_layer(&layer).unwrap();
    }

    fn new_popup(
        &mut self,
        parent: smithay::wayland::shell::wlr_layer::LayerSurface,
        popup: smithay::wayland::shell::xdg::PopupSurface,
    ) {
    }

    fn ack_configure(
        &mut self,
        surface: smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
        configure: smithay::wayland::shell::wlr_layer::LayerSurfaceConfigure,
    ) {
    }

    fn layer_destroyed(&mut self, surface: smithay::wayland::shell::wlr_layer::LayerSurface) {
        let found = self.space.outputs().find_map(|output| {
            let map = layer_map_for_output(output);
            let layer = map
                .layers()
                .find(|l| l.layer_surface() == &surface)
                .cloned()?;

            Some((output.clone(), layer))
        });

        if let Some((output, layer)) = found {
            let mut map = layer_map_for_output(&output);
            map.unmap_layer(&layer);
        }
    }
}

delegate_layer_shell!(AppState);
