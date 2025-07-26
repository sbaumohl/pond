use std::{
    collections::{BTreeSet, HashMap, HashSet},
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
        keyboard::{FilterResult, Keysym, ModifiersState},
        pointer::{AxisFrame, ButtonEvent, MotionEvent},
        Seat, SeatState,
    },
    utils::{Logical, Point, SERIAL_COUNTER},
    wayland::{
        compositor::CompositorState, fractional_scale::FractionalScaleManagerState,
        output::OutputManagerState, selection::data_device::DataDeviceState,
        shell::xdg::XdgShellState, shm::ShmState, socket::ListeningSocketSource,
    },
};
use wayland_server::{protocol::wl_surface::WlSurface, Display, DisplayHandle};

use crate::{config::schema::Configuration, ClientState, LoopData};

#[derive(Debug)]
pub struct WindowGrab {
    pub window: smithay::desktop::Window,
    pub grab_offset: Point<i32, Logical>,
}

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

    pub keybindings: HashMap<Keybinding, UserCommand>,

    // fractional scaling
    pub fractional_scale_manager: FractionalScaleManagerState,

    pub grabbed_window: Option<WindowGrab>,
}

impl PondState {
    pub fn init(
        event_loop: &mut EventLoop<LoopData>,
        display: &Display<PondState>,
        config: Configuration,
        // session: LibSeatSession,
    ) -> Self {
        let source = ListeningSocketSource::new_auto().unwrap();
        let socket_name = source.socket_name().to_string_lossy().into_owned();

        let shm_state = ShmState::new::<Self>(&display.handle(), vec![]);
        let mut seat_state = SeatState::new();
        let mut seat = seat_state.new_wl_seat(&display.handle(), "winit");
        seat.add_keyboard(Default::default(), 200, 25).unwrap();
        seat.add_pointer();

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
            fractional_scale_manager: FractionalScaleManagerState::new::<PondState>(
                &display.handle(),
            ),
            keybindings: parse_bindings(&config).unwrap(),
            grabbed_window: None,
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

#[derive(Debug)]
pub enum UserCommand {
    Execute(String),
    Quit,
    Shutdown,
    LogOut,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Keybinding {
    pub key: Keysym,
    pub modifiers: ModifiersState,
}

impl Keybinding {
    pub fn new(key: Keysym, modifiers: ModifiersState) -> Self {
        Keybinding { key, modifiers }
    }
}

pub fn parse_bindings(config: &Configuration) -> Result<HashMap<Keybinding, UserCommand>, String> {
    let mut bindings: HashMap<Keybinding, UserCommand> =
        HashMap::with_capacity(config.keybinds.len());

    'binding: for binding in &config.keybinds {
        let mut key: Option<Keysym> = None;
        let mut modifiers: ModifiersState = ModifiersState::default();
        modifiers.serialized.depressed = 1;

        for maybe_key in binding.key.to_lowercase().split("+") {
            match maybe_key {
                "super" => modifiers.logo = true,
                "ctrl" => modifiers.ctrl = true,
                "shift" => modifiers.shift = true,
                c if c.len() == 1 => {
                    key = Some(Keysym::from_char(c.chars().next().unwrap()));
                }
                _ => {
                    continue 'binding;
                }
            };
        }

        if let Some(k) = key {
            bindings.insert(
                Keybinding::new(k, modifiers),
                UserCommand::Execute(binding.command.clone()),
            );
        }
    }

    Ok(bindings)
}
