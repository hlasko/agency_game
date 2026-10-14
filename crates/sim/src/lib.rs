//! Pure game logic, no engine. Deterministic given a seed, testable from CLI.

pub mod content;
pub mod office;
pub mod pitch;

pub use content::{Content, Role, Slot};
pub use office::{CardView, RumorView, Sim, SimError, TalkOutcome};
pub use pitch::{Objection, Outcome, Pitch, Response};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_content_parses() {
        let c = Content::demo();
        assert_eq!(c.brief.personas.len(), 3);
        for col in &c.colleagues {
            for r in &col.rumors {
                assert!(c.rumor(r).is_some(), "{} references missing rumor {r}", col.id);
            }
        }
    }

    #[test]
    fn perspective_reveals_only_its_stat() {
        let mut sim = Sim::new(Content::demo(), 1);
        sim.study("bartek", "lens_irony").unwrap(); // Creative
        let v = sim.card_view("lens_irony").unwrap();
        assert_eq!(v.edge, Some(4));
        assert_eq!(v.appeal, None);
        assert_eq!(sim.study("bartek", "lens_irony").unwrap_err(), SimError::AlreadyStudied);
    }

    #[test]
    fn coffee_builds_trust_and_unlocks_gossip() {
        let mut sim = Sim::new(Content::demo(), 1);
        // Ola likes strategists; coffee as Kasia: +3 trust, unlocks min_trust 1..2 rumors.
        let o = sim.talk("kasia", "ola", true).unwrap();
        assert_eq!(o.trust_gain, 3);
        assert!(o.rumor.is_some());
    }

    #[test]
    fn confirmation_needs_two_sources() {
        let mut sim = Sim::new(Content::demo(), 1);
        sim.talk("kasia", "ola", true).unwrap();
        sim.talk("bartek", "wojtek", true).unwrap();
        let r = sim.rumors().into_iter().find(|r| r.id == "r_henryk_irony").unwrap();
        assert!(r.confirmed, "{r:?}");
    }

    #[test]
    fn safe_idea_wins_without_award_smart_idea_gets_award() {
        let sim = Sim::new(Content::demo(), 1);
        let safe = sim.start_pitch(&["ins_grandma", "lens_tribute", "fmt_celeb"], 7).unwrap();
        let o = safe.outcome().expect("safe idea is bought straight away");
        assert!(o.accepted && !o.award_shot, "{o:?}");

        let smart = sim.start_pitch(&["ins_sunday", "lens_tribute", "fmt_ooh"], 7).unwrap();
        let o = smart.outcome().expect("smart idea is bought straight away");
        assert!(o.accepted && o.award_shot, "{o:?}");
    }

    #[test]
    fn edgy_idea_triggers_feedback_and_false_gossip_backfires() {
        let mut sim = Sim::new(Content::demo(), 1);
        // Hear the false TikTok rumor about Kowal from Bartek.
        sim.talk("kasia", "bartek", false).unwrap();
        let mut p = sim.start_pitch(&["ins_sunday", "lens_irony", "fmt_tiktok"], 7).unwrap();
        assert!(p.outcome().is_none(), "edgy idea should face objections");
        // Answer until it ends; reframe on Kowal must backfire.
        while let Some(obj) = p.objection().cloned() {
            if obj.persona_id == "zdzislaw" && obj.can_reframe {
                let msg = p.respond(Response::Reframe).unwrap();
                assert!(msg.contains("fałszywa"), "{msg}");
            } else {
                p.respond(Response::Concede).unwrap();
            }
        }
        assert!(p.outcome().is_some());
    }

    #[test]
    fn day_runs_out() {
        let mut sim = Sim::new(Content::demo(), 1);
        let mut n = 0;
        while sim.talk("ola", "basia", true).is_ok() {
            n += 1;
        }
        assert_eq!(n, 16);
        assert_eq!(sim.clock(), (17, 0));
    }
}
