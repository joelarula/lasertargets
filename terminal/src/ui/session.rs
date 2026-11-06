//! The running game session: passes the server's session messages on to the games, and while
//! a session runs shows buttons to leave it and to pause or resume it.

use bevy::prelude::*;
use common::curator_core::CuratorEventEnvelope;
use common::game::{GameSession, GameSessionCreated, GameSessionUpdate};
use common::network::{NetworkMessage, ToServer};
use common::state::{ServerState, SessionState, TerminalState};
use terminal_ui::{Docking, ToolbarItem, ToolbarPressed};

use crate::net::ServerMessage;

/// The session the server last told us about
#[derive(Resource, Default)]
struct CurrentSession(Option<GameSession>);

pub struct SessionPlugin;

impl Plugin for SessionPlugin {
    fn build(&self, app: &mut App) {
        let forget = |mut session: ResMut<CurrentSession>| session.0 = None;
        app.init_resource::<CurrentSession>()
            .add_systems(Update, receive_sessions)
            .add_systems(OnEnter(ServerState::Session), spawn_session_buttons)
            .add_systems(OnExit(ServerState::Session), forget)
            .add_systems(OnEnter(TerminalState::Connecting), forget);
    }
}

fn receive_sessions(
    mut messages: MessageReader<ServerMessage>,
    mut current: ResMut<CurrentSession>,
    mut created: MessageWriter<GameSessionCreated>,
    mut updated: MessageWriter<GameSessionUpdate>,
) {
    for ServerMessage(message) in messages.read() {
        match message {
            NetworkMessage::SessionCreated(session) => {
                let game_session: GameSession = session.clone().into();
                current.0 = Some(game_session.clone());
                created.write(GameSessionCreated { game_session });
            }
            NetworkMessage::SessionUpdate(session) => {
                let game_session: GameSession = session.clone().into();
                current.0 = Some(game_session.clone());
                updated.write(GameSessionUpdate { game_session });
            }
            _ => {}
        }
    }
}

fn spawn_session_buttons(mut commands: Commands, current: Res<CurrentSession>) {
    let paused = current.0.as_ref().is_some_and(|s| s.state == SessionState::Paused);
    let button = |name: &str, order: u8, icon: &str| {
        (ToolbarItem::new(name).icon(icon).order(order).dock(Docking::Left).on(true), DespawnOnExit(ServerState::Session))
    };

    commands
        .spawn(button("exit_game", 1, "\u{f060}"))
        .observe(|_: On<ToolbarPressed>, current: Res<CurrentSession>, mut to_server: MessageWriter<ToServer>| {
            match &current.0 {
                Some(session) => {
                    to_server.write(ToServer(NetworkMessage::CloseSession { session_id: session.session_id }));
                }
                None => warn!("Exit pressed, but no session is known"),
            }
        });

    // Play icon while paused, pause icon otherwise
    commands
        .spawn(button("pause_resume_game", 2, if paused { "\u{f04b}" } else { "\u{f04c}" }))
        .observe(|_: On<ToolbarPressed>, current: Res<CurrentSession>, mut to_server: MessageWriter<ToServer>| {
            let Some(session) = &current.0 else { return };
            let tag = if session.state == SessionState::Paused { "resume_session" } else { "pause_session" };
            let envelope = CuratorEventEnvelope::new(tag, serde_json::json!({})).with_session(session.session_id);
            to_server.write(ToServer(NetworkMessage::CuratorEnvelope(envelope)));
        });
}
