use ahash::AHashMap as HashMap;
use smithay::input::keyboard::{FilterResult, Keysym, KeysymHandle, ModifiersState};

use crate::state::AppState;

#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) enum BindingActions {
    ExecProgram(Vec<String>),
    Noop,
}

#[derive(Debug, Default)]
pub struct UserConfig {
    pub user_bindings: HashMap<(NormalizedModifiers, Keysym), BindingActions>,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct NormalizedModifiers {
    shift: bool,
    ctrl: bool,
    caps_lock: bool,
    logo_super: bool,
}

impl NormalizedModifiers {
    pub fn new(shift: bool, ctrl: bool, caps_lock: bool, logo_super: bool) -> Self {
        Self {
            shift,
            ctrl,
            caps_lock,
            logo_super,
        }
    }
}

impl From<&ModifiersState> for NormalizedModifiers {
    fn from(value: &ModifiersState) -> Self {
        Self {
            shift: value.shift,
            ctrl: value.ctrl,
            caps_lock: value.caps_lock,
            logo_super: value.logo,
        }
    }
}

pub fn keybinding_pass(
    state: &mut AppState,
    modifiers: &ModifiersState,
    key_sym_handle: KeysymHandle,
) -> FilterResult<()> {
    let sym = key_sym_handle
        .raw_latin_sym_or_raw_current_sym()
        .unwrap_or(key_sym_handle.modified_sym());
    let binding_map = (NormalizedModifiers::from(modifiers), sym);

    if let Some(action) = state.user_config.user_bindings.get(&binding_map) {
        match action {
            BindingActions::ExecProgram(cmd) => {
                let _ = std::process::Command::new(&cmd[0])
                    .args(&cmd[1..])
                    .env("WAYLAND_DISPLAY", &state.socket_name)
                    .spawn();
                return FilterResult::Intercept(());
            }
            BindingActions::Noop => FilterResult::Forward,
        }
    } else {
        FilterResult::Forward
    }
}
