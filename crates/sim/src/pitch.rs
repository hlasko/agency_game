//! The pitch: client committee scores the idea, then feedback rounds where you
//! concede (lose edge), defend (gamble on what you know) or reframe (use gossip).

use crate::content::{CardDef, Content, PersonaDef};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub struct Idea {
    pub cards: Vec<String>,
    pub titles: Vec<String>,
    pub edge: i32,
    pub appeal: i32,
    pub cost: i32,
    pub tags: BTreeSet<String>,
    pub flaws: Vec<String>,
}

/// Satisfaction lost per peeve tag the idea touches. Hidden landmines, gossip reveals them.
const PEEVE_PENALTY: i32 = 5;

/// Appeal lost per hidden flaw the client spots.
const FLAW_PENALTY: i32 = 3;

impl Idea {
    pub fn from_cards(cards: &[CardDef]) -> Self {
        let flaws: Vec<String> = cards.iter().filter_map(|c| c.flaw.clone()).collect();
        Self {
            cards: cards.iter().map(|c| c.id.clone()).collect(),
            titles: cards.iter().map(|c| c.title.clone()).collect(),
            edge: cards.iter().map(|c| c.edge).sum(),
            appeal: cards.iter().map(|c| c.appeal).sum::<i32>() - FLAW_PENALTY * flaws.len() as i32,
            cost: cards.iter().map(|c| c.cost).sum(),
            tags: cards.iter().flat_map(|c| c.tags.iter().cloned()).collect(),
            flaws,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Response {
    /// Give in: edge -1, appeal +1 for everyone, objector calms down.
    Concede,
    /// Hold the line. Odds grow with true facts you know about the objector.
    Defend,
    /// Use a believed hot button. Works if the rumor was true, backfires if not.
    Reframe,
}

#[derive(Debug, Clone)]
pub struct Objection {
    pub persona_id: String,
    pub who: String,
    pub text: String,
    pub can_reframe: bool,
    pub defend_odds: f32,
}

#[derive(Debug, Clone)]
pub struct Outcome {
    pub accepted: bool,
    pub avg_satisfaction: f32,
    pub edge: i32,
    pub appeal: i32,
    pub award_shot: bool,
    pub money: i32,
    pub reputation: i32,
    pub headline: String,
}

pub struct Pitch {
    pub idea: Idea,
    personas: Vec<PersonaDef>,
    budget: i32,
    threshold: i32,
    award_edge: i32,
    max_rounds: u32,
    round: u32,
    mods: BTreeMap<String, i32>,
    /// (persona, tag, truth, used)
    believed: Vec<(String, String, bool, bool)>,
    true_facts: BTreeMap<String, u32>,
    rng: ChaCha8Rng,
    pub log: Vec<String>,
    objection: Option<Objection>,
    outcome: Option<Outcome>,
}

impl Pitch {
    pub(crate) fn new(
        content: &Content,
        idea: Idea,
        believed: Vec<(String, String, bool)>,
        true_facts: BTreeMap<String, u32>,
        seed: u64,
    ) -> Self {
        let t = &content.tuning;
        let mut p = Self {
            idea,
            personas: content.brief.personas.clone(),
            budget: content.brief.budget,
            threshold: t.accept_threshold,
            award_edge: t.award_edge,
            max_rounds: t.feedback_rounds,
            round: 0,
            mods: BTreeMap::new(),
            believed: believed.into_iter().map(|(a, b, c)| (a, b, c, false)).collect(),
            true_facts,
            rng: ChaCha8Rng::seed_from_u64(seed),
            log: Vec::new(),
            objection: None,
            outcome: None,
        };
        for f in p.idea.flaws.clone() {
            p.log.push(format!("Klient marszczy brwi: {f}"));
        }
        p.advance();
        p
    }

    pub fn satisfaction(&self, persona: &PersonaDef) -> i32 {
        let i = &self.idea;
        let w = (persona.appeal_weight + persona.edge_weight).max(1);
        let base = (i.appeal * persona.appeal_weight + i.edge * persona.edge_weight) / w;
        let hot = persona.hot.iter().filter(|t| i.tags.contains(*t)).count() as i32;
        let peeve = persona.peeves.iter().filter(|t| i.tags.contains(*t)).count() as i32;
        let over = (i.cost - self.budget).max(0);
        base + 2 * hot - PEEVE_PENALTY * peeve - persona.cost_weight * over
            + self.mods.get(&persona.id).copied().unwrap_or(0)
    }

    /// (name, title, satisfaction) per committee member.
    pub fn committee(&self) -> Vec<(String, String, i32)> {
        self.personas
            .iter()
            .map(|p| (p.name.clone(), p.title.clone(), self.satisfaction(p)))
            .collect()
    }

    fn avg(&self) -> f32 {
        let s: i32 = self.personas.iter().map(|p| self.satisfaction(p)).sum();
        s as f32 / self.personas.len().max(1) as f32
    }

    fn happy(&self) -> bool {
        self.avg() >= self.threshold as f32 && self.personas.iter().all(|p| self.satisfaction(p) >= 0)
    }

    pub fn objection(&self) -> Option<&Objection> {
        self.objection.as_ref()
    }
    pub fn outcome(&self) -> Option<&Outcome> {
        self.outcome.as_ref()
    }
    pub fn round(&self) -> (u32, u32) {
        (self.round, self.max_rounds)
    }

    fn defend_odds(&self, persona: &str) -> f32 {
        let facts = self.true_facts.get(persona).copied().unwrap_or(0) as f32;
        (0.3 + 0.2 * facts).min(0.9)
    }

    fn advance(&mut self) {
        if self.round >= self.max_rounds || self.happy() {
            self.finish();
            return;
        }
        let worst = self
            .personas
            .iter()
            .min_by_key(|p| self.satisfaction(p))
            .expect("brief needs personas")
            .clone();
        self.objection = Some(Objection {
            can_reframe: self.believed.iter().any(|b| b.0 == worst.id && !b.3),
            defend_odds: self.defend_odds(&worst.id),
            persona_id: worst.id.clone(),
            who: format!("{} ({})", worst.name, worst.title),
            text: worst.objection.clone(),
        });
    }

    pub fn respond(&mut self, r: Response) -> Result<String, &'static str> {
        let obj = self.objection.clone().ok_or("pitch zakończony")?;
        let id = obj.persona_id.clone();
        let msg = match r {
            Response::Concede => {
                self.idea.edge -= 1;
                self.idea.appeal += 1;
                *self.mods.entry(id).or_default() += 1;
                "Ustępujecie. Pomysł robi się bezpieczniejszy… i trochę bardziej płaski.".to_string()
            }
            Response::Defend => {
                if self.rng.random_bool(obj.defend_odds as f64) {
                    *self.mods.entry(id).or_default() += 2;
                    "Bronicie pomysłu i trafiacie w punkt. Klient kiwa głową.".to_string()
                } else {
                    *self.mods.entry(id).or_default() -= 2;
                    "Obrona nie przekonuje. Atmosfera gęstnieje.".to_string()
                }
            }
            Response::Reframe => {
                let Some(b) = self.believed.iter_mut().find(|b| b.0 == id && !b.3) else {
                    return Err("brak plotki, na której można oprzeć reframe");
                };
                b.3 = true;
                let (tag, truth) = (b.1.clone(), b.2);
                if truth {
                    *self.mods.entry(id).or_default() += 3;
                    format!("Przeformułowujecie pomysł przez „{tag}”. Trafione, plotka była prawdziwa.")
                } else {
                    *self.mods.entry(id).or_default() -= 2;
                    format!("Gracie na „{tag}”… i cisza. Plotka była fałszywa.")
                }
            }
        };
        self.log.push(msg.clone());
        self.round += 1;
        self.objection = None;
        self.advance();
        Ok(msg)
    }

    fn finish(&mut self) {
        self.objection = None;
        let accepted = self.happy();
        let edge = self.idea.edge;
        let appeal = self.idea.appeal;
        let award_shot = accepted && edge >= self.award_edge;
        let headline = match (accepted, award_shot) {
            (true, true) => "Wygrany pitch i pomysł z szansą na nagrodę. Zespół idzie na piwo.",
            (true, false) => "Wygrany pitch. Klient zadowolony, ale nikt o tym nie będzie mówił.",
            (false, _) => "Przegrany pitch. Klient „wróci do was z feedbackiem”.",
        }
        .to_string();
        self.outcome = Some(Outcome {
            accepted,
            avg_satisfaction: self.avg(),
            edge,
            appeal,
            award_shot,
            money: if accepted { 10 + appeal } else { 0 },
            reputation: if accepted { edge } else { -1 },
            headline,
        });
    }
}
