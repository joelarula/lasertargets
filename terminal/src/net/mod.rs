//! Talking to the server: the connection itself, the server state the terminal mirrors, and the
//! input it forwards. Other plugins only read [`ServerMessage`] and write `ToServer`.

mod input;
mod link;
mod sync;

use bevy::prelude::*;

pub use link::ServerMessage;

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((link::LinkPlugin, sync::SyncPlugin, input::InputPlugin));
    }
}
