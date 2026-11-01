use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};
use common::curator_core::{ActorPrompt, ActorPromptRequestEvent, ActorPromptResponse, ActorPromptResponseEvent};
use common::state::TerminalState;

#[derive(Resource, Default)]
pub struct ActivePromptState {
    pub active_prompt: Option<ActorPrompt>,
}

pub struct CuratorPromptPlugin;

impl Plugin for CuratorPromptPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActivePromptState>();
        app.add_systems(Update, receive_actor_prompts);
        app.add_systems(EguiPrimaryContextPass, render_prompt_modal);
        app.add_systems(OnEnter(TerminalState::Connecting), clear_prompt_on_disconnect);
    }
}

fn receive_actor_prompts(
    mut prompt_reader: MessageReader<ActorPromptRequestEvent>,
    mut prompt_state: ResMut<ActivePromptState>,
) {
    for req in prompt_reader.read() {
        info!("Received interactive prompt: '{}' [{:?}]", req.prompt.title, req.prompt.prompt_id);
        prompt_state.active_prompt = Some(req.prompt.clone());
    }
}

fn clear_prompt_on_disconnect(mut prompt_state: ResMut<ActivePromptState>) {
    prompt_state.active_prompt = None;
}

fn render_prompt_modal(
    mut contexts: EguiContexts,
    mut prompt_state: ResMut<ActivePromptState>,
    mut response_writer: MessageWriter<ActorPromptResponseEvent>,
) {
    let Some(ref prompt) = prompt_state.active_prompt else {
        return;
    };

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let mut selected_choice: Option<String> = None;

    egui::Window::new(&prompt.title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.add_space(4.0);
            ui.label(&prompt.message);
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                for choice in &prompt.choices {
                    if ui.button(&choice.label).clicked() {
                        selected_choice = Some(choice.id.clone());
                    }
                }
            });
        });

    if let Some(choice_id) = selected_choice {
        info!("Prompt choice selected: {}", choice_id);
        response_writer.write(ActorPromptResponseEvent {
            response: ActorPromptResponse {
                prompt_id: prompt.prompt_id,
                actor_id: None,
                selected_choice_id: choice_id,
                custom_payload: None,
            },
        });
        prompt_state.active_prompt = None;
    }
}
