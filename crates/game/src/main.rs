//! Agency Sim: 3D office front-end for the `sim` crate.
//!
//! You step into colleagues (Tab), walk around (WASD), talk (E), take someone
//! for coffee (F, then E at the machine), study brief cards from your current
//! perspective at the board, then pitch.

mod ui;
mod world;

use bevy::prelude::*;
use sim::{Content, Pitch, Sim, Slot};
use std::collections::BTreeMap;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Mode {
    #[default]
    Office,
    Board,
    Journal,
    Pitch,
}

#[derive(Resource)]
pub struct Game {
    pub sim: Sim,
    /// Colleague id you are currently playing.
    pub active: String,
    /// Colleague id following you to the coffee machine.
    pub follower: Option<String>,
    pub picks: BTreeMap<Slot, String>,
    /// Card selected on the board.
    pub cursor: Option<String>,
    pub pitch: Option<Pitch>,
    pub log: Vec<String>,
    pub seed: u64,
}

impl Game {
    pub fn new(seed: u64) -> Self {
        let sim = Sim::new(Content::demo(), seed);
        let active = sim
            .content
            .colleagues
            .iter()
            .find(|c| c.role.is_some())
            .map(|c| c.id.clone())
            .expect("content needs a playable colleague");
        let brief = &sim.content.brief;
        let log = vec![format!("Brief: {} · {}. {}", brief.client, brief.product, brief.task)];
        Self { sim, active, follower: None, picks: BTreeMap::new(), cursor: None, pitch: None, log, seed }
    }

    pub fn say(&mut self, line: impl Into<String>) {
        self.log.push(line.into());
        if self.log.len() > 40 {
            self.log.remove(0);
        }
    }

    pub fn name_of(&self, id: &str) -> String {
        self.sim.content.colleague(id).map(|c| c.name.clone()).unwrap_or_else(|| id.into())
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Agency Sim".into(),
                resolution: (1280, 800).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(Game::new(42))
        .insert_resource(ClearColor(Color::srgb(0.08, 0.08, 0.1)))
        .init_state::<Mode>()
        .add_plugins((world::WorldPlugin, ui::UiPlugin))
        .run();
}
