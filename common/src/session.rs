use bevy::asset::uuid::Uuid;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
pub use crate::state::SessionState;

/// Channel application descriptor registered with the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelApp {
    pub id: u16,
    pub name: String,
}

/// Registry of available channel applications that can be launched
#[derive(Resource, Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelRegistry {
    apps: HashMap<u16, ChannelApp>,
}

impl ChannelRegistry {
    pub fn register_app(&mut self, app: ChannelApp) {
        self.apps.insert(app.id, app);
    }

    pub fn register_game(&mut self, game: ChannelApp) {
        self.register_app(game);
    }

    pub fn get_app_by_id(&self, id: u16) -> Option<&ChannelApp> {
        self.apps.get(&id)
    }

    pub fn get_game_by_id(&self, id: u16) -> Option<&ChannelApp> {
        self.get_app_by_id(id)
    }

    pub fn all_apps(&self) -> Vec<&ChannelApp> {
        let mut list: Vec<&ChannelApp> = self.apps.values().collect();
        list.sort_by_key(|a| a.id);
        list
    }

    pub fn all_games(&self) -> Vec<&ChannelApp> {
        self.all_apps()
    }
}

/// Participant entity connected to or interacting with a session
#[derive(Component, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Actor {
    pub name: String,
    pub uuid: Uuid,
    pub roles: Vec<String>,
}

impl Actor {
    pub fn new(name: impl Into<String>, uuid: Uuid) -> Self {
        Self {
            name: name.into(),
            uuid,
            roles: Vec::new(),
        }
    }

    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.roles.push(role.into());
        self
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }
}

/// Snapshot of actors currently attached to a session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionActors {
    pub session_id: Uuid,
    pub actors: Vec<Actor>,
}

/// Backwards-compatibility metadata container for session actors
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActorMetaData {
    pub game_session_uuid: Uuid,
    pub actors: Vec<Actor>,
}

impl From<SessionActors> for ActorMetaData {
    fn from(sa: SessionActors) -> Self {
        Self {
            game_session_uuid: sa.session_id,
            actors: sa.actors,
        }
    }
}

impl From<ActorMetaData> for SessionActors {
    fn from(meta: ActorMetaData) -> Self {
        Self {
            session_id: meta.game_session_uuid,
            actors: meta.actors,
        }
    }
}

/// Generic session channel descriptor on the server
#[derive(Component, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Session {
    pub session_id: Uuid,
    pub channel_name: String,
    pub game_id: u16,
    pub name: String,
    pub state: SessionState,
    pub start_timestamp: Option<u64>,
    pub end_timestamp: Option<u64>,
}

impl Session {
    pub fn new(session_id: Uuid, channel_name: impl Into<String>) -> Self {
        let channel = channel_name.into();
        Self {
            session_id,
            name: channel.clone(),
            channel_name: channel,
            game_id: 0,
            state: SessionState::Active,
            start_timestamp: None,
            end_timestamp: None,
        }
    }

    pub fn start(&mut self) {
        if self.start_timestamp.is_none() {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs();
            self.start_timestamp = Some(timestamp);
        }
        self.state = SessionState::Active;
    }

    pub fn stop(&mut self) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        self.end_timestamp = Some(timestamp);
        self.state = SessionState::Finished;
    }

    pub fn pause(&mut self) {
        if self.start_timestamp.is_some() && self.end_timestamp.is_none() {
            self.state = SessionState::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.start_timestamp.is_some() && self.end_timestamp.is_none() {
            self.state = SessionState::Active;
        }
    }
}

/// Broadcast when a new session channel is established
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
pub struct SessionCreated {
    pub session: Session,
}

/// Event to register an actor into a session
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
pub struct RegisterActorEvent {
    pub session_id: Uuid,
    pub actor: Actor,
}

/// Event to unregister an actor from a session
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
pub struct UnregisterActorEvent {
    pub session_id: Uuid,
    pub actor_uuid: Uuid,
}

/// Broadcast when actors in a session change
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
pub struct SessionActorUpdateEvent {
    pub session_id: Uuid,
    pub actors: Vec<Actor>,
}

pub type GameActorUpdateEvent = SessionActorUpdateEvent;

/// Broadcast when session channel metadata or status updates
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
pub struct SessionUpdate {
    pub session: Session,
}

/// Event to signal session initialization on server
#[derive(Message, Debug, Clone)]
pub struct InitSessionEvent {
    pub game_id: u16,
    pub session_id: Uuid,
    pub initial_state: SessionState,
}

/// Event to signal session close / exit
#[derive(Message, Debug, Clone)]
pub struct CloseSessionEvent {
    pub session_id: Uuid,
}

#[derive(Message, Debug, Clone)]
pub struct StartSessionEvent {
    pub session_id: Uuid,
}

#[derive(Message, Debug, Clone)]
pub struct PauseSessionEvent {
    pub session_id: Uuid,
}

#[derive(Message, Debug, Clone)]
pub struct ResumeSessionEvent {
    pub session_id: Uuid,
}

#[derive(Message, Debug, Clone)]
pub struct FinishSessionEvent {
    pub session_id: Uuid,
}

// Aliases and transitional types for smooth migration away from common::game
pub type Game = ChannelApp;
pub type GameRegistry = ChannelRegistry;
pub type GameSession = Session;

#[derive(Message, Debug, Clone)]
pub struct GameSessionCreated {
    pub game_session: Session,
}

impl From<SessionCreated> for GameSessionCreated {
    fn from(sc: SessionCreated) -> Self {
        Self { game_session: sc.session }
    }
}

impl From<GameSessionCreated> for SessionCreated {
    fn from(gc: GameSessionCreated) -> Self {
        Self { session: gc.game_session }
    }
}

#[derive(Message, Debug, Clone)]
pub struct GameSessionUpdate {
    pub game_session: Session,
}

impl From<SessionUpdate> for GameSessionUpdate {
    fn from(su: SessionUpdate) -> Self {
        Self { game_session: su.session }
    }
}

impl From<GameSessionUpdate> for SessionUpdate {
    fn from(gu: GameSessionUpdate) -> Self {
        Self { session: gu.game_session }
    }
}

#[derive(Message, Debug, Clone)]
pub struct InitGameSessionEvent {
    pub game_id: u16,
    pub game_session_uuid: Uuid,
    pub initial_state: SessionState,
}

impl From<InitGameSessionEvent> for InitSessionEvent {
    fn from(e: InitGameSessionEvent) -> Self {
        Self {
            game_id: e.game_id,
            session_id: e.game_session_uuid,
            initial_state: e.initial_state,
        }
    }
}

#[derive(Message, Debug, Clone)]
pub struct ExitGameEvent {
    pub game_session_uuid: Uuid,
}

impl From<ExitGameEvent> for CloseSessionEvent {
    fn from(e: ExitGameEvent) -> Self {
        Self {
            session_id: e.game_session_uuid,
        }
    }
}




#[derive(Message, Debug, Clone)]
pub struct StartGameEvent {
    pub game_session_uuid: Uuid,
}

#[derive(Message, Debug, Clone)]
pub struct PauseGameEvent {
    pub game_session_uuid: Uuid,
}

#[derive(Message, Debug, Clone)]
pub struct ResumeGameEvent {
    pub game_session_uuid: Uuid,
}

#[derive(Message, Debug, Clone)]
pub struct FinishGameEvent {
    pub game_session_uuid: Uuid,
}
#[allow(non_upper_case_globals)]
pub const GameRegistryPlugin: SessionPlugin = SessionPlugin;

/// Generic session registration plugin
pub struct SessionPlugin;

impl Plugin for SessionPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<SessionState>()
            .add_message::<InitSessionEvent>()
            .add_message::<CloseSessionEvent>()
            .add_message::<StartSessionEvent>()
            .add_message::<PauseSessionEvent>()
            .add_message::<ResumeSessionEvent>()
            .add_message::<FinishSessionEvent>()
            .add_message::<RegisterActorEvent>()
            .add_message::<UnregisterActorEvent>()
            .add_message::<SessionActorUpdateEvent>()
            .add_message::<SessionCreated>()
            .add_message::<SessionUpdate>()
            .add_message::<GameSessionCreated>()
            .add_message::<GameSessionUpdate>()
            .add_message::<InitGameSessionEvent>()
            .add_message::<ExitGameEvent>()
            .add_message::<StartGameEvent>()
            .add_message::<PauseGameEvent>()
            .add_message::<ResumeGameEvent>()
            .add_message::<FinishGameEvent>()
            .init_resource::<ChannelRegistry>();
    }
}
