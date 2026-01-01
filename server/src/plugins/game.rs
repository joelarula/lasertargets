use bevy::{asset::uuid::Uuid, prelude::*};
use common::game::{GameSession, Game, GameRegistry};
use bevy_quinnet::shared::ClientId;

#[derive(Message)]
pub struct StartGameEvent {
    pub game_name: String,
    pub client_id: ClientId,
}

#[derive(Message)]
pub struct PauseGameEvent {
    pub game_session_uuid: Uuid,
    pub client_id: ClientId,
}

#[derive(Message)]
pub struct ResumeGameEvent {
    pub game_session_uuid: Uuid,
    pub client_id: ClientId,
}

#[derive(Message)]
pub struct StopGameEvent {
    pub game_session_uuid: Uuid,
    pub client_id: ClientId,
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_start_game);
        app.add_systems(Update, handle_pause_game);
        app.add_systems(Update, handle_resume_game);
        app.add_systems(Update, handle_stop_game);
    }
}

fn handle_start_game(
    mut commands: Commands,
    mut start_game_events: MessageReader<StartGameEvent>,
) {
    for event in start_game_events.read() {
        info!("Received StartGameEvent for: {}", event.game_name);
        let mut new_game_session = GameSession {
            name: event.game_name.clone(),
            uuid: bevy::asset::uuid::Uuid::new_v4(),
            paused: false,
            start_timestamp: None,
            end_timestamp: None,
            actors: Vec::new(),
        };
        new_game_session.start();
        commands.spawn(new_game_session);
        info!("Game session '{}' started.", event.game_name);
        // TODO: Send GameSessionResponse back to client_id (this will be done in network.rs, which listens to game session changes)
    }
}

fn handle_pause_game(
    mut game_sessions: Query<(Entity, &mut GameSession)>,
    mut pause_game_events: MessageReader<PauseGameEvent>,
) {
    for event in pause_game_events.read() {
        info!("Received PauseGameEvent for: {}", event.game_session_uuid);
        if let Some((_, mut game_session)) = game_sessions.iter_mut().find(|(_, gs)| gs.uuid == event.game_session_uuid) {
            game_session.pause();
            info!("Game session '{}' paused.", game_session.uuid);
            // TODO: Send GameSessionResponse back to client_id (this will be done in network.rs, which listens to game session changes)
        }
    }
}

fn handle_resume_game(
    mut game_sessions: Query<(Entity, &mut GameSession)>,
    mut resume_game_events: MessageReader<ResumeGameEvent>,
) {
    for event in resume_game_events.read() {
        info!("Received ResumeGameEvent for: {}", event.game_session_uuid);
        if let Some((_, mut game_session)) = game_sessions.iter_mut().find(|(_, gs)| gs.uuid == event.game_session_uuid) {
            game_session.resume();
            info!("Game session '{}' resumed.", game_session.uuid);
            // TODO: Send GameSessionResponse back to client_id (this will be done in network.rs, which listens to game session changes)
        }
    }
}

fn handle_stop_game(
    mut commands: Commands,
    mut game_sessions: Query<(Entity, &mut GameSession)>,
    mut stop_game_events: MessageReader<StopGameEvent>,
) {
    for event in stop_game_events.read() {
        info!("Received StopGameEvent for: {}", event.game_session_uuid);
        if let Some((entity, mut game_session)) = game_sessions.iter_mut().find(|(_, gs)| gs.uuid == event.game_session_uuid) {
            game_session.stop();
            commands.entity(entity).despawn();
            info!("Game session '{}' stopped and despawned.", game_session.name);
            // TODO: Send GameSessionResponse back to client_id (this will be done in network.rs, which listens to game session changes)
        }
    }
}
