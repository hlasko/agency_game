//! Brute-force every card combination and print what the committee does with it
//! (no feedback rounds played). Run: `cargo run -p sim --example balance`

use sim::{Content, Sim, Slot};

fn main() {
    let sim = Sim::new(Content::demo(), 0);
    let by_slot = |s: Slot| -> Vec<String> {
        sim.content.brief.cards.iter().filter(|c| c.slot == s).map(|c| c.id.clone()).collect()
    };
    let mut rows = Vec::new();
    for i in by_slot(Slot::Insight) {
        for l in by_slot(Slot::Lens) {
            for f in by_slot(Slot::Format) {
                let p = sim.start_pitch(&[&i, &l, &f], 0).unwrap();
                let sats: Vec<i32> = p.committee().iter().map(|c| c.2).collect();
                let verdict = match p.outcome() {
                    Some(o) if o.award_shot => "WIN+AWARD",
                    Some(o) if o.accepted => "WIN",
                    Some(_) => "LOSS",
                    None => "feedback",
                };
                rows.push((p.idea.edge, format!("{i:12} {l:15} {f:11} edge {:>2} appeal {:>2} cost {} sats {:?} -> {verdict}",
                    p.idea.edge, p.idea.appeal, p.idea.cost, sats)));
            }
        }
    }
    rows.sort_by_key(|r| -r.0);
    for (_, r) in rows {
        println!("{r}");
    }
}
