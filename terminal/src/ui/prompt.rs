//! Shows the curator's interactive prompts as a modal and answers with the chosen option

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use common::curator_core::{ActorPrompt, ActorPromptResponse, ActorPromptResponseEvent};
use common::network::{NetworkMessage, ToServer};
use common::state::TerminalState;

use crate::net::ServerMessage;

/// The prompt waiting for an answer
#[derive(Resource, Default)]
struct ActivePrompt(Option<ActorPrompt>);

pub struct PromptPlugin;

impl Plugin for PromptPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActivePrompt>()
            .add_systems(OnEnter(TerminalState::Connecting), |mut prompt: ResMut<ActivePrompt>| prompt.0 = None)
            .add_systems(Update, (receive_prompts, forward_responses))
            .add_systems(EguiPrimaryContextPass, prompt_modal.run_if(|prompt: Res<ActivePrompt>| prompt.0.is_some()));
    }
}

fn receive_prompts(mut messages: MessageReader<ServerMessage>, mut active: ResMut<ActivePrompt>) {
    for ServerMessage(message) in messages.read() {
        if let NetworkMessage::CuratorPrompt(prompt) = message {
            info!("Prompt: '{}' [{:?}]", prompt.title, prompt.prompt_id);
            active.0 = Some(prompt.clone());
        }
    }
}

/// Every local prompt answer goes to the server, whose curator is waiting for it
fn forward_responses(mut responses: MessageReader<ActorPromptResponseEvent>, mut to_server: MessageWriter<ToServer>) {
    to_server.write_batch(responses.read().map(|r| ToServer(NetworkMessage::CuratorPromptResponse(r.response.clone()))));
}

fn prompt_modal(mut contexts: EguiContexts, mut active: ResMut<ActivePrompt>, mut responses: MessageWriter<ActorPromptResponseEvent>) {
    let Some(prompt) = &active.0 else { return };
    let Ok(ctx) = contexts.ctx_mut() else { return };

    let mut chosen = None;
    egui::Window::new(&prompt.title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.add_space(4.0);
            ui.label(&prompt.message);
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                for choice in &prompt.choices {
                    if ui.button(&choice.label).clicked() {
                        chosen = Some(choice.id.clone());
                    }
                }
            });
        });

    if let Some(choice) = chosen {
        info!("Prompt answered: {choice}");
        responses.write(ActorPromptResponseEvent {
            response: ActorPromptResponse {
                prompt_id: prompt.prompt_id,
                actor_id: None,
                selected_choice_id: choice,
                custom_payload: None,
            },
        });
        active.0 = None;
    }
}
