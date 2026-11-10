use smithay::{
    input::keyboard::{Keysym, ModifiersState},
    reexports::{calloop::EventLoop, wayland_server::Display},
};

mod inputs;
mod spacing;
mod state;
mod winit;
use crate::{
    state::{
        AppState,
        config::{BindingActions, NormalizedModifiers},
    },
    winit::init_winit,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut event_loop: EventLoop<AppState> = EventLoop::try_new()?;

    let display: Display<AppState> = Display::new()?;

    let mut state: AppState = AppState::new(&mut event_loop, display);
    state.user_config.user_bindings.insert(
        (
            NormalizedModifiers::new(true, false, false, false),
            Keysym::from_char('x'),
        ),
        BindingActions::ExecProgram(vec!["fuzzel".to_string()]),
    );

    let _ = init_winit(&mut event_loop, &mut state);

    unsafe { std::env::set_var("WAYLAND_DISPLAY", &state.socket_name) };
    println!("SOCKET_NAME={:?}", state.socket_name);

    event_loop.run(None, &mut state, move |_| {})?;

    Ok(())
}
