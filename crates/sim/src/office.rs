//! The office day: studying cards from different perspectives, talking to
//! colleagues, coffee chats, gossip. Produces the knowledge the pitch runs on.

use crate::content::{Content, Reveal, Role, Slot};
use crate::pitch::{Idea, Pitch};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimError {
    OutOfTime,
    Unknown(String),
    NotPlayable(String),
    SameAsSelf,
    AlreadyStudied,
    MissingSlot(Slot),
}

impl std::fmt::Display for SimError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimError::OutOfTime => write!(f, "Nie ma już czasu, pora na pitch."),
            SimError::Unknown(id) => write!(f, "Nieznany identyfikator: {id}"),
            SimError::NotPlayable(id) => write!(f, "{id} nie jest grywalną postacią"),
            SimError::SameAsSelf => write!(f, "Rozmowa z samym sobą nic nie wnosi."),
            SimError::AlreadyStudied => write!(f, "Ta perspektywa jest już odkryta."),
            SimError::MissingSlot(s) => write!(f, "Brakuje karty w slocie {s:?}"),
        }
    }
}
impl std::error::Error for SimError {}

#[derive(Debug, Clone)]
pub struct Heard {
    pub rumor: String,
    pub from: String,
}

/// What one perspective sees on a card.
#[derive(Debug, Clone)]
pub struct FacetView {
    pub role: Role,
    pub text: String,
    /// The hard number or fact this role uncovers.
    pub reveal: String,
}

#[derive(Debug, Clone, Default)]
pub struct CardView {
    pub id: String,
    pub title: String,
    pub slot: Option<Slot>,
    pub facets: Vec<FacetView>,
    pub edge: Option<i32>,
    pub appeal: Option<i32>,
    pub cost: Option<i32>,
    pub tags: Option<Vec<String>>,
    /// `Some(None)` = checked, no flaw. `None` = not checked yet.
    pub flaw: Option<Option<String>>,
}

#[derive(Debug, Clone)]
pub struct RumorView {
    pub id: String,
    pub text: String,
    pub sources: Vec<String>,
    /// Heard from at least two different people.
    pub confirmed: bool,
}

#[derive(Debug, Clone)]
pub struct TalkOutcome {
    pub line: String,
    pub rumor: Option<String>,
    pub trust_gain: i32,
    pub trust_now: i32,
}

pub struct Sim {
    pub content: Content,
    rng: ChaCha8Rng,
    minute: u32,
    trust: BTreeMap<String, i32>,
    heard: Vec<Heard>,
    studied: BTreeSet<(String, Role)>,
    small_talk_idx: BTreeMap<String, usize>,
}

impl Sim {
    pub fn new(content: Content, seed: u64) -> Self {
        let trust = content.colleagues.iter().map(|c| (c.id.clone(), 0)).collect();
        Self {
            content,
            rng: ChaCha8Rng::seed_from_u64(seed),
            minute: 0,
            trust,
            heard: Vec::new(),
            studied: BTreeSet::new(),
            small_talk_idx: BTreeMap::new(),
        }
    }

    pub fn minutes_left(&self) -> u32 {
        self.content.tuning.day_minutes.saturating_sub(self.minute)
    }

    /// Wall clock, day starts at 9:00.
    pub fn clock(&self) -> (u32, u32) {
        let t = 9 * 60 + self.minute;
        (t / 60, t % 60)
    }

    pub fn trust(&self, colleague: &str) -> i32 {
        self.trust.get(colleague).copied().unwrap_or(0)
    }

    fn spend(&mut self, minutes: u32) -> Result<(), SimError> {
        if self.minutes_left() < minutes {
            return Err(SimError::OutOfTime);
        }
        self.minute += minutes;
        Ok(())
    }

    fn role_of(&self, colleague: &str) -> Result<Role, SimError> {
        let c = self
            .content
            .colleague(colleague)
            .ok_or_else(|| SimError::Unknown(colleague.into()))?;
        c.role.ok_or_else(|| SimError::NotPlayable(colleague.into()))
    }

    /// The character you currently play studies a card on the board.
    pub fn study(&mut self, as_colleague: &str, card_id: &str) -> Result<FacetView, SimError> {
        let role = self.role_of(as_colleague)?;
        let card = self
            .content
            .card(card_id)
            .ok_or_else(|| SimError::Unknown(card_id.into()))?
            .clone();
        if self.studied.contains(&(card_id.to_string(), role)) {
            return Err(SimError::AlreadyStudied);
        }
        self.spend(self.content.tuning.study_minutes)?;
        self.studied.insert((card_id.to_string(), role));
        Ok(facet(&card, role))
    }

    pub fn card_view(&self, card_id: &str) -> Option<CardView> {
        let card = self.content.card(card_id)?;
        let seen = |r: Role| self.studied.contains(&(card_id.to_string(), r));
        Some(CardView {
            id: card.id.clone(),
            title: card.title.clone(),
            slot: Some(card.slot),
            facets: Role::ALL.iter().filter(|r| seen(**r)).map(|r| facet(card, *r)).collect(),
            edge: seen(Role::Creative).then_some(card.edge),
            appeal: seen(Role::Account).then_some(card.appeal),
            cost: seen(Role::Producer).then_some(card.cost),
            tags: seen(Role::Strategist).then(|| card.tags.clone()),
            flaw: seen(Role::Strategist).then(|| card.flaw.clone()),
        })
    }

    pub fn cards_in(&self, slot: Slot) -> Vec<CardView> {
        self.content
            .brief
            .cards
            .iter()
            .filter(|c| c.slot == slot)
            .filter_map(|c| self.card_view(&c.id))
            .collect()
    }

    /// Talk to a colleague. `coffee` = you went to the machine together:
    /// costs more time, builds trust faster.
    pub fn talk(&mut self, as_colleague: &str, to: &str, coffee: bool) -> Result<TalkOutcome, SimError> {
        if as_colleague == to {
            return Err(SimError::SameAsSelf);
        }
        let my_role = self.role_of(as_colleague)?;
        let other = self
            .content
            .colleague(to)
            .ok_or_else(|| SimError::Unknown(to.into()))?
            .clone();
        let t = &self.content.tuning;
        self.spend(if coffee { t.coffee_minutes } else { t.talk_minutes })?;

        let mut gain = if other.likes.contains(&my_role) { 2 } else { 1 };
        if coffee {
            gain += 1;
        }
        let trust = self.trust.entry(to.to_string()).or_insert(0);
        *trust += gain;
        let trust_now = *trust;

        // Share the first rumor this colleague hasn't told us yet and is willing to.
        let already: BTreeSet<&str> = self
            .heard
            .iter()
            .filter(|h| h.from == to)
            .map(|h| h.rumor.as_str())
            .collect();
        let next = other
            .rumors
            .iter()
            .filter(|id| !already.contains(id.as_str()))
            .filter_map(|id| self.content.rumor(id))
            .find(|r| r.min_trust <= trust_now)
            .map(|r| (r.id.clone(), r.text.clone()));

        let line = match &next {
            Some((_, text)) => {
                let lead = ["Słuchaj…", "Między nami…", "Nie ode mnie to wiesz, ale…"];
                use rand::RngExt;
                let pick = lead[self.rng.random_range(0..lead.len())];
                format!("{pick} {text}")
            }
            None => {
                let i = self.small_talk_idx.entry(to.to_string()).or_insert(0);
                let line = other
                    .small_talk
                    .get(*i % other.small_talk.len().max(1))
                    .cloned()
                    .unwrap_or_else(|| "…".into());
                *i += 1;
                line
            }
        };
        let rumor = next.map(|(id, _)| {
            self.heard.push(Heard { rumor: id.clone(), from: to.to_string() });
            id
        });
        Ok(TalkOutcome { line, rumor, trust_gain: gain, trust_now })
    }

    pub fn rumors(&self) -> Vec<RumorView> {
        let mut by_id: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for h in &self.heard {
            by_id.entry(&h.rumor).or_default().insert(&h.from);
        }
        by_id
            .into_iter()
            .filter_map(|(id, from)| {
                let r = self.content.rumor(id)?;
                let sources: Vec<String> = from
                    .iter()
                    .filter_map(|f| self.content.colleague(f).map(|c| c.name.clone()))
                    .collect();
                Some(RumorView {
                    id: id.to_string(),
                    text: r.text.clone(),
                    confirmed: sources.len() >= 2,
                    sources,
                })
            })
            .collect()
    }

    /// Start the pitch with one card per slot. Consumes the day's knowledge.
    pub fn start_pitch(&self, picks: &[&str], seed: u64) -> Result<Pitch, SimError> {
        let mut chosen = Vec::new();
        for slot in Slot::ALL {
            let card = picks
                .iter()
                .filter_map(|id| self.content.card(id))
                .find(|c| c.slot == slot)
                .ok_or(SimError::MissingSlot(slot))?;
            chosen.push(card.clone());
        }
        let idea = Idea::from_cards(&chosen);

        // What the team actually learned about each persona.
        let mut believed = Vec::new();
        let mut true_facts: BTreeMap<String, u32> = BTreeMap::new();
        let mut seen = BTreeSet::new();
        for h in &self.heard {
            if !seen.insert(h.rumor.clone()) {
                continue;
            }
            let Some(r) = self.content.rumor(&h.rumor) else { continue };
            match &r.reveals {
                Reveal::Hot { persona, tag } => {
                    believed.push((persona.clone(), tag.clone(), r.truth));
                    if r.truth {
                        *true_facts.entry(persona.clone()).or_default() += 1;
                    }
                }
                Reveal::Peeve { persona, .. } if r.truth => {
                    *true_facts.entry(persona.clone()).or_default() += 1;
                }
                _ => {}
            }
        }
        Ok(Pitch::new(&self.content, idea, believed, true_facts, seed))
    }
}

fn facet(card: &crate::content::CardDef, role: Role) -> FacetView {
    let reveal = match role {
        Role::Creative => format!("Edge: {}", card.edge),
        Role::Account => format!("Appeal: {}", card.appeal),
        Role::Producer => format!("Koszt: {}", card.cost),
        Role::Strategist => match &card.flaw {
            Some(f) => format!("Tagi: {} · UWAGA: {f}", card.tags.join(", ")),
            None => format!("Tagi: {}", card.tags.join(", ")),
        },
    };
    FacetView {
        role,
        text: card.facets.get(&role).cloned().unwrap_or_default(),
        reveal,
    }
}
