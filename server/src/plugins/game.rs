use bevy::{asset::uuid::Uuid, prelude::*};
use common::{game::{Game, GameRegistry, GameSession}, state::GameState};
use bevy_quinnet::shared::ClientId;

#[derive(Message)]
pub struct InitGameSessionEvent {
    pub game_id: u16,
    pub game_name: String,
    pub game_session_uuid: Uuid,
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
pub struct StopGameEvent {
    pub game_session_uuid: Uuid,
}

#[derive(Message, Debug, Clone)]
pub struct GameSessionCreated {
    pub game_session: GameSession,
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<InitGameSessionEvent>()
            .add_message::<StartGameEvent>()
            .add_message::<PauseGameEvent>()
            .add_message::<ResumeGameEvent>()
            .add_message::<StopGameEvent>()
            .add_message::<GameSessionCreated>()
            .add_systems(Update, handle_init_game)
            .add_systems(Update, handle_start_game)
            .add_systems(Update, handle_pause_game)
            .add_systems(Update, handle_resume_game)
            .add_systems(Update, handle_stop_game);
    }
}

fn handle_init_game(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    current_state: Res<State<GameState>>,
    mut init_game_events: MessageReader<InitGameSessionEvent>,
    mut game_session_created: MessageWriter<GameSessionCreated>,
) {
    for event in init_game_events.read() {
        info!("Received InitGameSessionEvent for: {}", event.game_session_uuid);

        let mut new_game_session = GameSession {
            id: event.game_id,
            name: event.game_name.clone(),
            uuid: bevy::asset::uuid::Uuid::new_v4(),
            paused: false,
            started: false,
            start_timestamp: None,
            end_timestamp: None,
        };

        new_game_session.start();
        commands.spawn(new_game_session.clone());

        next_state.set(GameState::InGame(event.game_id));
        game_session_created.write(GameSessionCreated {
            game_session: new_game_session,
        });
    }
}

fn handle_start_game(
    mut game_sessions: Query<(Entity, &mut GameSession)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut start_game_events: MessageReader<StartGameEvent>,
) {
    for event in start_game_events.read() {
        if let Some((_, mut game_session)) = game_sessions.iter_mut().find(|(_, gs)| gs.uuid == event.game_session_uuid) {
            next_state.set(GameState::InGame(game_session.id));
            game_session.start();
       }
    }
}

fn handle_pause_game(
    mut game_sessions: Query<(Entity, &mut GameSession)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut pause_game_events: MessageReader<PauseGameEvent>,
) {
    for event in pause_game_events.read() {
        if let Some((_, mut game_session)) = game_sessions.iter_mut().find(|(_, gs)| gs.uuid == event.game_session_uuid) {
            next_state.set(GameState::Paused(game_session.id));
            game_session.pause();
        }
    }
}

fn handle_resume_game(
    mut game_sessions: Query<(Entity, &mut GameSession)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut resume_game_events: MessageReader<ResumeGameEvent>,
) {
    for event in resume_game_events.read() {
        if let Some((_, mut game_session)) = game_sessions.iter_mut().find(|(_, gs)| gs.uuid == event.game_session_uuid) {
            next_state.set(GameState::InGame(game_session.id));
            game_session.resume();
        }
    }
}

fn handle_stop_game(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    mut game_sessions: Query<(Entity, &mut GameSession)>,
    mut stop_game_events: MessageReader<StopGameEvent>,
) {
    for event in stop_game_events.read() {
        if let Some((entity, mut game_session)) = game_sessions.iter_mut().find(|(_, gs)| gs.uuid == event.game_session_uuid) {
           
            game_session.stop();
            commands.entity(entity).despawn();
            next_state.set(GameState::Finished(game_session.id));
        }
    }
}
