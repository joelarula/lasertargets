use bevy::{asset::uuid::Uuid, prelude::*};
use common::{game::{GameSessionUpdate, Game, GameRegistry, GameSession, GameSessionCreated}, state::GameState};
use bevy_quinnet::shared::ClientId;

#[derive(Message)]
pub struct InitGameSessionEvent {
    pub game_id: u16,
    pub game_session_uuid: Uuid,
    pub initial_state: GameState,
}

#[derive(Message)]
pub struct StartGameEvent {
    pub game_session_uuid: Uuid,
}

#[derive(Message)]
pub struct PauseGameEvent {
    pub game_session_uuid: Uuid,
}

#[derive(Message)]
pub struct ResumeGameEvent {
    pub game_session_uuid: Uuid,
}

#[derive(Message)]
pub struct FinishGameEvent {
    pub game_session_uuid: Uuid,
}

#[derive(Message)]
pub struct ExitGameEvent {
    pub game_session_uuid: Uuid,
}



pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<InitGameSessionEvent>()
            .add_message::<StartGameEvent>()
            .add_message::<PauseGameEvent>()
            .add_message::<ResumeGameEvent>()
            .add_message::<FinishGameEvent>()
            .add_message::<ExitGameEvent>()
            .add_message::<GameSessionCreated>()
            .add_systems(Update, handle_init_game)
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
    mut next_server_state: ResMut<NextState<common::state::ServerState>>,
    mut game_sessions: Query<(Entity, &mut GameSession)>,
    mut exit_game_events: MessageReader<ExitGameEvent>,
    mut broadcast_events: MessageWriter<GameSessionUpdate>,
) {
    for event in exit_game_events.read() {
        if let Some((entity, mut game_session)) = game_sessions.iter_mut().find(|(_, gs)| gs.session_id == event.game_session_uuid) {
            game_session.state = GameState::Finished;
            broadcast_events.write(GameSessionUpdate { game_session: game_session.clone() });
            commands.entity(entity).despawn();
            next_game_state.set(GameState::Finished);
            next_server_state.set(common::state::ServerState::Menu);
        }
    }
}

fn handle_init_game(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    _current_state: Res<State<GameState>>,
    mut init_game_events: MessageReader<InitGameSessionEvent>,
    mut game_session_created: MessageWriter<GameSessionCreated>,
    game_registry: Res<GameRegistry>,
) {
    for event in init_game_events.read() {
        info!("Received InitGameSessionEvent for game ID: {}", event.game_id);

        // Get game from registry using game_id
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

        next_state.set(GameState::InGame);
        game_session_created.write(GameSessionCreated {
            game_session: session.clone(),
        });

    }
}

fn handle_start_game(
    mut game_sessions: Query<(Entity, &mut GameSession)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut start_game_events: MessageReader<StartGameEvent>,
    mut broadcast_events: MessageWriter<GameSessionUpdate>,
) {
    for event in start_game_events.read() {
        if let Some((_, mut game_session)) = game_sessions.iter_mut().find(|(_, gs)| gs.session_id == event.game_session_uuid) {
            next_state.set(GameState::InGame);
            game_session.start();
            broadcast_events.write(GameSessionUpdate { game_session: game_session.clone() });
        }
    }
}

fn handle_pause_game(
    mut game_sessions: Query<(Entity, &mut GameSession)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut pause_game_events: MessageReader<PauseGameEvent>,
    mut broadcast_events: MessageWriter<GameSessionUpdate>,
) {
    for event in pause_game_events.read() {
        if let Some((_, mut game_session)) = game_sessions.iter_mut().find(|(_, gs)| gs.session_id == event.game_session_uuid) {
            next_state.set(GameState::Paused);
            game_session.pause();
            broadcast_events.write(GameSessionUpdate { game_session: game_session.clone() });
        }
    }
}

fn handle_resume_game(
    mut game_sessions: Query<(Entity, &mut GameSession)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut resume_game_events: MessageReader<ResumeGameEvent>,
    mut broadcast_events: MessageWriter<GameSessionUpdate>,
) {
    for event in resume_game_events.read() {
        if let Some((_, mut game_session)) = game_sessions.iter_mut().find(|(_, gs)| gs.session_id == event.game_session_uuid) {
            next_state.set(GameState::InGame);
            game_session.resume();
            broadcast_events.write(GameSessionUpdate { game_session: game_session.clone() });
        }
    }
}

fn handle_finish_game(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    mut game_sessions: Query<(Entity, &mut GameSession)>,
    mut stop_game_events: MessageReader<FinishGameEvent>,
    mut broadcast_events: MessageWriter<GameSessionUpdate>,
) {
    for event in stop_game_events.read() {
        if let Some((entity, mut game_session)) = game_sessions.iter_mut().find(|(_, gs)| gs.session_id == event.game_session_uuid) {
            game_session.stop();
            broadcast_events.write(GameSessionUpdate { game_session: game_session.clone() });
        }
    }
}
