//! Data definitions loaded from RON. Everything the designer tunes lives here,
//! nothing here holds runtime state.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Perspective a playable character brings. Each role reveals a different
/// facet of the same card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Role {
    /// Sees whether the insight holds up and which tags it really carries.
    Strategist,
    /// Sees creative edge (award / viral potential).
    Creative,
    /// Sees client appeal.
    Account,
    /// Sees production cost and risk.
    Producer,
}

impl Role {
    pub const ALL: [Role; 4] = [Role::Strategist, Role::Creative, Role::Account, Role::Producer];

    pub fn label_pl(self) -> &'static str {
        match self {
            Role::Strategist => "Strateg",
            Role::Creative => "Kreacja",
            Role::Account => "Account",
            Role::Producer => "Producent",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Slot {
    Insight,
    Lens,
    Format,
}

impl Slot {
    pub const ALL: [Slot; 3] = [Slot::Insight, Slot::Lens, Slot::Format];
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardDef {
    pub id: String,
    pub slot: Slot,
    pub title: String,
    /// Creative edge. Revealed by `Role::Creative`.
    pub edge: i32,
    /// Client appeal. Revealed by `Role::Account`.
    pub appeal: i32,
    /// Production cost. Revealed by `Role::Producer`.
    pub cost: i32,
    /// Tags matched against persona hot buttons / peeves. Revealed by `Role::Strategist`.
    pub tags: Vec<String>,
    /// Hidden flaw (e.g. an outdated insight). Revealed by `Role::Strategist`.
    #[serde(default)]
    pub flaw: Option<String>,
    /// Flavour text per perspective.
    pub facets: BTreeMap<Role, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaDef {
    pub id: String,
    pub name: String,
    pub title: String,
    pub edge_weight: i32,
    pub appeal_weight: i32,
    /// Penalty per point of cost above the brief budget.
    pub cost_weight: i32,
    pub hot: Vec<String>,
    pub peeves: Vec<String>,
    /// What they say when they push back in a feedback round.
    pub objection: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Reveal {
    Hot { persona: String, tag: String },
    Peeve { persona: String, tag: String },
    Flavour,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RumorDef {
    pub id: String,
    pub text: String,
    /// False rumors cost you if you act on them.
    pub truth: bool,
    pub reveals: Reveal,
    /// Trust the teller needs before sharing it.
    pub min_trust: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColleagueDef {
    pub id: String,
    pub name: String,
    /// `Some` = playable, you can step into this person.
    pub role: Option<Role>,
    /// Roles this person opens up to faster.
    pub likes: Vec<Role>,
    pub rumors: Vec<String>,
    pub small_talk: Vec<String>,
    /// Desk position on the office floor (x, z), metres.
    pub desk: (f32, f32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefDef {
    pub client: String,
    pub product: String,
    pub task: String,
    pub budget: i32,
    pub personas: Vec<PersonaDef>,
    pub cards: Vec<CardDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tuning {
    pub day_minutes: u32,
    pub talk_minutes: u32,
    pub coffee_minutes: u32,
    pub study_minutes: u32,
    pub feedback_rounds: u32,
    pub accept_threshold: i32,
    pub award_edge: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub tuning: Tuning,
    pub brief: BriefDef,
    pub colleagues: Vec<ColleagueDef>,
    pub rumors: Vec<RumorDef>,
}

impl Content {
    pub fn from_ron(src: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(src)
    }

    /// Bundled demo content, so the game runs without an asset folder.
    pub fn demo() -> Self {
        Self::from_ron(include_str!("../content/demo.ron")).expect("bundled demo.ron must parse")
    }

    pub fn card(&self, id: &str) -> Option<&CardDef> {
        self.brief.cards.iter().find(|c| c.id == id)
    }
    pub fn persona(&self, id: &str) -> Option<&PersonaDef> {
        self.brief.personas.iter().find(|p| p.id == id)
    }
    pub fn colleague(&self, id: &str) -> Option<&ColleagueDef> {
        self.colleagues.iter().find(|c| c.id == id)
    }
    pub fn rumor(&self, id: &str) -> Option<&RumorDef> {
        self.rumors.iter().find(|r| r.id == id)
    }
}
