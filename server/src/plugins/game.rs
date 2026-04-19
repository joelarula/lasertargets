use bevy::{ prelude::*};
use common::{game::{ExitGameEvent, FinishGameEvent, GameRegistry, GameSession, GameSessionCreated, GameSessionUpdate, InitGameSessionEvent, PauseGameEvent, ResumeGameEvent, StartGameEvent}, state::{GameState, ServerState}};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_init_game)
            .add_systems(Update, handle_start_game)
            .add_systems(Update, handle_pause_game)
            .add_systems(Update, handle_resume_game)
            .add_systems(Update, handle_finish_game)
            .add_systems(Update, handle_exit_game);
    }
}

fn handle_exit_game(
    mut commands: Commands,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut next_server_state: ResMut<NextState<ServerState>>,
    game_sessions: Query<(Entity, &GameSession)>,
    mut exit_game_events: MessageReader<ExitGameEvent>,
    mut broadcast_events: MessageWriter<GameSessionUpdate>,
) {
    for event in exit_game_events.read() {
        info!("[handle_exit_game] Received ExitGameEvent for session: {}", event.game_session_uuid);
        if let Some((entity, game_session)) = game_sessions.iter().find(|(_, gs)| gs.session_id == event.game_session_uuid) {
            info!("[handle_exit_game] Found GameSession entity, setting state to Finished and despawning");
            let mut session = game_session.clone();
            session.state = GameState::Finished;
            broadcast_events.write(GameSessionUpdate { game_session: session });
            commands.entity(entity).despawn();
        } else {
            warn!("[handle_exit_game] No GameSession entity found for session: {}", event.game_session_uuid);
        }
        info!("[handle_exit_game] Setting GameState to Finished and ServerState to Menu");
        next_game_state.set(GameState::Finished);
        next_server_state.set(ServerState::Menu);
    }
}

fn handle_init_game(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    mut next_server_state: ResMut<NextState<ServerState>>,
    mut init_game_events: MessageReader<InitGameSessionEvent>,
    mut game_session_created: MessageWriter<GameSessionCreated>,
    game_registry: Res<GameRegistry>,
) {
    for event in init_game_events.read() {
        info!("Received InitGameSessionEvent for game ID: {}", event.game_id);

        let game = match game_registry.get_game_by_id(event.game_id) {
            Some(game) => game,
            None => {
                warn!("Game with ID {} not found in registry", event.game_id);
                continue;
            }
        };

        let mut new_game_session = GameSession {
            game_id: event.game_id,
            session_id: event.game_session_uuid,
            name: game.name.clone(),
            state: event.initial_state.clone(),
            start_timestamp: None,
            end_timestamp: None,
        };

        new_game_session.start();
        let session = new_game_session.clone();
        commands.spawn(session.clone());

        next_server_state.set(ServerState::InGame);
        next_state.set(GameState::InGame);
  
        game_session_created.write(GameSessionCreated {
            game_session: session,
        });
    }
}

fn handle_start_game(
    mut game_sessions: Query<&mut GameSession>,
    mut next_state: ResMut<NextState<GameState>>,
    mut start_game_events: MessageReader<StartGameEvent>,
    mut broadcast_events: MessageWriter<GameSessionUpdate>,
) {
    for event in start_game_events.read() {
        if let Some(mut game_session) = game_sessions.iter_mut().find(|gs| gs.session_id == event.game_session_uuid) {
            next_state.set(GameState::InGame);
            game_session.start();
            broadcast_events.write(GameSessionUpdate { game_session: game_session.clone() });
        }
    }
}

fn handle_pause_game(
    mut game_sessions: Query<&mut GameSession>,
    mut next_state: ResMut<NextState<GameState>>,
    mut pause_game_events: MessageReader<PauseGameEvent>,
    mut broadcast_events: MessageWriter<GameSessionUpdate>,
) {
    for event in pause_game_events.read() {
        if let Some(mut game_session) = game_sessions.iter_mut().find(|gs| gs.session_id == event.game_session_uuid) {
            next_state.set(GameState::Paused);
            game_session.pause();
            broadcast_events.write(GameSessionUpdate { game_session: game_session.clone() });
        }
    }
}

fn handle_resume_game(
    mut game_sessions: Query<&mut GameSession>,
    mut next_state: ResMut<NextState<GameState>>,
    mut resume_game_events: MessageReader<ResumeGameEvent>,
    mut broadcast_events: MessageWriter<GameSessionUpdate>,
) {
    for event in resume_game_events.read() {
        if let Some(mut game_session) = game_sessions.iter_mut().find(|gs| gs.session_id == event.game_session_uuid) {
            next_state.set(GameState::InGame);
            game_session.resume();
            broadcast_events.write(GameSessionUpdate { game_session: game_session.clone() });
        }
    }
}

fn handle_finish_game(
    mut game_sessions: Query<&mut GameSession>,
    mut stop_game_events: MessageReader<FinishGameEvent>,
    mut broadcast_events: MessageWriter<GameSessionUpdate>,
) {
    for event in stop_game_events.read() {
        if let Some(mut game_session) = game_sessions.iter_mut().find(|gs| gs.session_id == event.game_session_uuid) {
            game_session.stop();
            broadcast_events.write(GameSessionUpdate { game_session: game_session.clone() });
        }
    }
}
