use aery::Aery;
use bevy::app::Plugin;

pub mod cells;

pub mod prelude {
    pub use crate::cells::cell_query::*;
    pub use crate::cells::commands::{CellCommandExt, CellCommands};
    pub use crate::cells::CellMapLabel;

    pub use crate::cells::coords::*;
    pub use crate::cells::*;
    pub use crate::CellsPlugin;
}

/// Adds Cells dependencies to the App.
/// # Note
/// If you are using [Aery](https://crates.io/crates/aery), add it to the App before this plugin, or just add this plugin.
/// This plugin will add Aery if it's not in the app, since it is a unique plugin,
/// having multiple will panic.
pub struct CellsPlugin;

impl Plugin for CellsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        if !app.is_plugin_added::<Aery>() {
            app.add_plugins(Aery);
        }
    }
}
