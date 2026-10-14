//! HUD, name labels, interaction prompts, and the Board / Journal / Pitch overlays.

use crate::world::{Colleague, MainCamera, BOARD_POS, COFFEE_POS, STATION_RANGE, TALK_RANGE};
use crate::{Game, Mode};
use bevy::prelude::*;
use sim::{Response, Role, Slot};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_ui)
            .add_systems(
                Update,
                (
                    office_input.run_if(in_state(Mode::Office)),
                    board_input.run_if(in_state(Mode::Board)),
                    journal_input.run_if(in_state(Mode::Journal)),
                    pitch_input.run_if(in_state(Mode::Pitch)),
                    (update_hud, update_labels, update_overlay),
                )
                    .chain(),
            );
    }
}

#[derive(Resource, Clone)]
struct UiFont {
    regular: Handle<Font>,
    medium: Handle<Font>,
}

#[derive(Component)]
struct HudTop;
#[derive(Component)]
struct HudBottom;
#[derive(Component)]
struct OverlayPanel;
#[derive(Component)]
struct OverlayText;
#[derive(Component)]
struct Label(String);

const INK: Color = Color::srgb(0.95, 0.94, 0.90);
const PANEL: Color = Color::srgba(0.07, 0.07, 0.09, 0.88);

fn tf(font: &Handle<Font>, size: f32) -> TextFont {
    TextFont { font: FontSource::from(font.clone()), font_size: FontSize::Px(size), ..default() }
}

fn spawn_ui(mut commands: Commands, mut fonts: ResMut<Assets<Font>>, game: Res<Game>) {
    // Embedded: the default Bevy font has no Polish diacritics, and embedding
    // means the binary runs from any working directory.
    let font = UiFont {
        regular: fonts.add(Font::from_bytes(include_bytes!("../assets/fonts/Poppins-Regular.ttf").to_vec())),
        medium: fonts.add(Font::from_bytes(include_bytes!("../assets/fonts/Poppins-Medium.ttf").to_vec())),
    };

    commands.spawn((
        HudTop,
        Text::new(""),
        tf(&font.medium, 18.0),
        TextColor(INK),
        BackgroundColor(PANEL),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            padding: UiRect::all(px(10)),
            ..default()
        },
    ));
    commands.spawn((
        HudBottom,
        Text::new(""),
        tf(&font.regular, 16.0),
        TextColor(INK),
        BackgroundColor(PANEL),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            left: px(12),
            right: px(12),
            padding: UiRect::all(px(12)),
            ..default()
        },
    ));
    commands
        .spawn((
            OverlayPanel,
            BackgroundColor(PANEL),
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                top: px(150),
                bottom: px(130),
                left: percent(12),
                right: percent(12),
                padding: UiRect::all(px(20)),
                overflow: Overflow::clip(),
                ..default()
            },
        ))
        .with_children(|b| {
            b.spawn((OverlayText, Text::new(""), tf(&font.regular, 16.0), TextColor(INK)));
        });

    for c in &game.sim.content.colleagues {
        commands.spawn((
            Label(c.id.clone()),
            Text::new(c.name.clone()),
            tf(&font.medium, 14.0),
            TextColor(INK),
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
            Node {
                position_type: PositionType::Absolute,
                padding: UiRect::axes(px(6), px(2)),
                ..default()
            },
        ));
    }
    commands.insert_resource(font);
}

// ---------------------------------------------------------------- helpers

struct Nearby {
    colleague: Option<String>,
    at_coffee: bool,
    at_board: bool,
    follower_close: bool,
}

fn nearby(game: &Game, bodies: &Query<(&Colleague, &Transform)>) -> Nearby {
    let me = bodies.iter().find(|(c, _)| c.id == game.active).map(|(_, t)| t.translation).unwrap_or_default();
    let colleague = bodies
        .iter()
        .filter(|(c, _)| c.id != game.active)
        .map(|(c, t)| (c.id.clone(), t.translation.distance(me)))
        .filter(|(_, d)| *d < TALK_RANGE)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id);
    let follower_close = game
        .follower
        .as_ref()
        .and_then(|f| bodies.iter().find(|(c, _)| &c.id == f))
        .is_some_and(|(_, t)| t.translation.distance(COFFEE_POS) < STATION_RANGE + 1.0);
    Nearby {
        colleague,
        at_coffee: me.distance(COFFEE_POS) < STATION_RANGE,
        at_board: me.distance(BOARD_POS) < STATION_RANGE,
        follower_close,
    }
}

/// Board order: Insight, Lens, Format, in content order. Index = key 1..9.
fn board_cards(game: &Game) -> Vec<String> {
    Slot::ALL
        .iter()
        .flat_map(|s| game.sim.content.brief.cards.iter().filter(move |c| c.slot == *s))
        .map(|c| c.id.clone())
        .collect()
}

fn digit(keys: &ButtonInput<KeyCode>) -> Option<usize> {
    const D: [KeyCode; 9] = [
        KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5,
        KeyCode::Digit6, KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9,
    ];
    D.iter().position(|k| keys.just_pressed(*k))
}

fn active_role(game: &Game) -> Option<Role> {
    game.sim.content.colleague(&game.active).and_then(|c| c.role)
}

fn slot_pl(s: Slot) -> &'static str {
    match s {
        Slot::Insight => "Insight",
        Slot::Lens => "Kąt",
        Slot::Format => "Format",
    }
}

fn bar(v: i32, max: i32) -> String {
    let n = v.clamp(0, max) as usize;
    format!("[{}{}]", "=".repeat(n), "-".repeat(max as usize - n))
}

// ---------------------------------------------------------------- input

fn office_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut game: ResMut<Game>,
    bodies: Query<(&Colleague, &Transform)>,
    mut next: ResMut<NextState<Mode>>,
) {
    let near = nearby(&game, &bodies);
    let me = game.active.clone();

    if keys.just_pressed(KeyCode::KeyE) {
        if near.at_board {
            next.set(Mode::Board);
        } else if near.at_coffee && near.follower_close {
            let who = game.follower.take().expect("follower_close implies follower");
            match game.sim.talk(&me, &who, true) {
                Ok(o) => {
                    let name = game.name_of(&who);
                    game.say(format!("[kawa · {name} · zaufanie {}] {}", o.trust_now, o.line));
                }
                Err(e) => game.say(e.to_string()),
            }
        } else if let Some(who) = near.colleague.clone() {
            match game.sim.talk(&me, &who, false) {
                Ok(o) => {
                    let name = game.name_of(&who);
                    game.say(format!("{name}: {}", o.line));
                }
                Err(e) => game.say(e.to_string()),
            }
        }
    }
    if keys.just_pressed(KeyCode::KeyF) {
        if game.follower.is_some() {
            game.follower = None;
            game.say("Dobra, innym razem.");
        } else if let Some(who) = near.colleague {
            let name = game.name_of(&who);
            game.follower = Some(who);
            game.say(format!("{name}: Kawa? Jasne, idę z tobą."));
        }
    }
    if keys.just_pressed(KeyCode::KeyJ) {
        next.set(Mode::Journal);
    }
    if keys.just_pressed(KeyCode::KeyP) {
        let picks: Vec<&str> = game.picks.values().map(String::as_str).collect();
        match game.sim.start_pitch(&picks, game.seed) {
            Ok(p) => {
                game.pitch = Some(p);
                game.follower = None;
                next.set(Mode::Pitch);
            }
            Err(e) => game.say(format!("{e}. Wybierz karty przy tablicy (cyfra, potem Spacja).")),
        }
    }
}

/// Digit selects a card (and studies it from your current perspective if that
/// view is still hidden). Space puts the selected card into the pitch.
fn board_input(keys: Res<ButtonInput<KeyCode>>, mut game: ResMut<Game>, mut next: ResMut<NextState<Mode>>) {
    if keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::KeyE) {
        next.set(Mode::Office);
        return;
    }
    if keys.just_pressed(KeyCode::Space) {
        let Some(id) = game.cursor.clone() else { return };
        let slot = game.sim.content.card(&id).map(|c| c.slot).expect("board card exists");
        if game.picks.get(&slot) == Some(&id) {
            game.picks.remove(&slot);
        } else {
            game.picks.insert(slot, id);
        }
        return;
    }
    let Some(i) = digit(&keys) else { return };
    let cards = board_cards(&game);
    let Some(id) = cards.get(i).cloned() else { return };
    game.cursor = Some(id.clone());
    let me = game.active.clone();
    match game.sim.study(&me, &id) {
        Ok(f) => game.say(format!("[{}] {} · {}", f.role.label_pl(), f.text, f.reveal)),
        Err(sim::SimError::AlreadyStudied) => {}
        Err(e) => game.say(e.to_string()),
    }
}

fn journal_input(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<Mode>>) {
    if keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::KeyJ) {
        next.set(Mode::Office);
    }
}

fn pitch_input(keys: Res<ButtonInput<KeyCode>>, mut game: ResMut<Game>, mut next: ResMut<NextState<Mode>>) {
    let Some(p) = game.pitch.as_mut() else { return };
    if p.outcome().is_some() {
        if keys.just_pressed(KeyCode::KeyR) {
            let seed = game.seed.wrapping_add(1);
            *game = Game::new(seed);
            next.set(Mode::Office);
        }
        return;
    }
    let r = match digit(&keys) {
        Some(0) => Response::Concede,
        Some(1) => Response::Defend,
        Some(2) => Response::Reframe,
        _ => return,
    };
    let _ = p.respond(r); // result lands in p.log, shown in the overlay
}

// ---------------------------------------------------------------- rendering

fn update_hud(
    game: Res<Game>,
    mode: Res<State<Mode>>,
    bodies: Query<(&Colleague, &Transform)>,
    mut top: Query<&mut Text, (With<HudTop>, Without<HudBottom>)>,
    mut bottom: Query<&mut Text, (With<HudBottom>, Without<HudTop>)>,
) {
    let (h, m) = game.sim.clock();
    let role = active_role(&game).map(|r| r.label_pl()).unwrap_or("");
    let picks: Vec<String> = Slot::ALL
        .iter()
        .map(|s| match game.picks.get(s).and_then(|id| game.sim.content.card(id)) {
            Some(c) => format!("{}: {}", slot_pl(*s), c.title),
            None => format!("{}: -", slot_pl(*s)),
        })
        .collect();
    if let Ok(mut t) = top.single_mut() {
        t.0 = format!(
            "{h:02}:{m:02}  ·  pitch o 17:00 ({} min)\nGrasz jako: {} ({role})\n{}",
            game.sim.minutes_left(),
            game.name_of(&game.active),
            picks.join("\n"),
        );
    }

    let near = nearby(&game, &bodies);
    let mut prompt = Vec::new();
    match mode.get() {
        Mode::Office => {
            if near.at_board {
                prompt.push("E - tablica z briefem".to_string());
            }
            if near.at_coffee && near.follower_close {
                let f = game.follower.as_deref().map(|f| game.name_of(f)).unwrap_or_default();
                prompt.push(format!("E - kawa ({f})"));
            } else if let Some(c) = &near.colleague {
                prompt.push(format!("E - rozmowa ({})", game.name_of(c)));
                if game.follower.is_none() {
                    prompt.push("F - weź na kawę".into());
                }
            }
            if game.follower.is_some() && !near.follower_close {
                prompt.push("idź do ekspresu · F - odpuść".into());
            }
            prompt.push("WASD - chodź · Tab - zmień postać · J - notes · P - pitch".into());
        }
        Mode::Board => prompt.push("1-9 - wybierz kartę (analizuje ją obecna postać, 30 min) · Spacja - dodaj/usuń z pitchu · Esc".into()),
        Mode::Journal => prompt.push("J / Esc - zamknij".into()),
        Mode::Pitch => prompt.push("1 - ustąp · 2 - broń · 3 - przeformułuj (plotka) · R - nowa gra po wyniku".into()),
    }
    if let Ok(mut t) = bottom.single_mut() {
        let last: Vec<&str> = game.log.iter().rev().take(3).rev().map(String::as_str).collect();
        t.0 = format!("{}\n\n{}", last.join("\n"), prompt.join("   |   "));
    }
}

fn update_labels(
    game: Res<Game>,
    mode: Res<State<Mode>>,
    cam: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    bodies: Query<(&Colleague, &GlobalTransform)>,
    mut labels: Query<(&Label, &mut Node, &mut Text, &mut Visibility)>,
) {
    let Ok((camera, cam_t)) = cam.single() else { return };
    for (label, mut node, mut text, mut vis) in &mut labels {
        let Some((c, t)) = bodies.iter().find(|(c, _)| c.id == label.0) else { continue };
        if *mode.get() != Mode::Office {
            *vis = Visibility::Hidden;
            continue;
        }
        match camera.world_to_viewport(cam_t, t.translation() + Vec3::Y * 2.1) {
            Ok(p) => {
                *vis = Visibility::Inherited;
                node.left = px(p.x - 40.0);
                node.top = px(p.y);
            }
            Err(_) => *vis = Visibility::Hidden,
        }
        let def = game.sim.content.colleague(&c.id);
        let role = def.and_then(|d| d.role).map(|r| r.label_pl()).unwrap_or("biuro");
        let name = def.map(|d| d.name.as_str()).unwrap_or("?");
        let tag = if c.id == game.active {
            " (ty)".to_string()
        } else if game.follower.as_deref() == Some(c.id.as_str()) {
            " -> kawa".to_string()
        } else {
            format!(" · zaufanie {}", game.sim.trust(&c.id))
        };
        text.0 = format!("{name} [{role}]{tag}");
    }
}

fn update_overlay(
    game: Res<Game>,
    mode: Res<State<Mode>>,
    mut panel: Query<&mut Visibility, With<OverlayPanel>>,
    mut text: Query<&mut Text, With<OverlayText>>,
) {
    let Ok(mut vis) = panel.single_mut() else { return };
    let Ok(mut text) = text.single_mut() else { return };
    let body = match mode.get() {
        Mode::Office => {
            *vis = Visibility::Hidden;
            return;
        }
        Mode::Board => board_text(&game),
        Mode::Journal => journal_text(&game),
        Mode::Pitch => pitch_text(&game),
    };
    *vis = Visibility::Inherited;
    text.0 = body;
}

fn board_text(game: &Game) -> String {
    let role = active_role(game).map(|r| r.label_pl()).unwrap_or("");
    let brief = &game.sim.content.brief;
    let mut s = format!(
        "TABLICA · {} / {} · budżet {}\nPatrzysz jako: {role}. Każda rola widzi inną prawdę o karcie.\n",
        brief.client, brief.product, brief.budget
    );
    let mut last_slot = None;
    for (i, id) in board_cards(game).iter().enumerate() {
        let Some(v) = game.sim.card_view(id) else { continue };
        if v.slot != last_slot {
            s.push_str(&format!("\n{}\n", slot_pl(v.slot.expect("card has slot")).to_uppercase()));
            last_slot = v.slot;
        }
        let q = |o: Option<i32>| o.map(|x| x.to_string()).unwrap_or_else(|| "?".into());
        let picked = if game.picks.values().any(|p| p == id) { "  << DO PITCHU" } else { "" };
        let cursor = if game.cursor.as_deref() == Some(id.as_str()) { "> " } else { "  " };
        s.push_str(&format!(
            "{cursor}{}. {}   edge {} · appeal {} · koszt {}{}\n",
            i + 1, v.title, q(v.edge), q(v.appeal), q(v.cost), picked
        ));
        if let Some(tags) = &v.tags {
            s.push_str(&format!("     tagi: {}\n", tags.join(", ")));
        }
        if let Some(Some(flaw)) = &v.flaw {
            s.push_str(&format!("     ! {flaw}\n"));
        }
        for f in &v.facets {
            s.push_str(&format!("     {}: {}\n", f.role.label_pl(), f.text));
        }
    }
    s
}

fn journal_text(game: &Game) -> String {
    let mut s = String::from("NOTES · plotki zebrane w biurze\n\n");
    let rumors = game.sim.rumors();
    if rumors.is_empty() {
        s.push_str("Nic jeszcze nie wiesz. Pogadaj z ludźmi, a najlepiej weź kogoś na kawę.\n");
    }
    for r in rumors {
        let status = if r.confirmed { "POTWIERDZONE" } else { "jedno źródło" };
        s.push_str(&format!("- {}\n    od: {} · {status}\n", r.text, r.sources.join(", ")));
    }
    s.push_str("\nZAUFANIE\n");
    for c in &game.sim.content.colleagues {
        let likes: Vec<&str> = c.likes.iter().map(|r| r.label_pl()).collect();
        s.push_str(&format!(
            "{}: {}   (lubi rozmowy z: {})\n",
            c.name,
            game.sim.trust(&c.id),
            likes.join(", ")
        ));
    }
    s
}

fn pitch_text(game: &Game) -> String {
    let Some(p) = &game.pitch else { return String::new() };
    let mut s = format!("PITCH · {}\n\n", p.idea.titles.join("  +  "));
    s.push_str(&format!("Edge {} · Appeal {} · Koszt {}\n\n", p.idea.edge, p.idea.appeal, p.idea.cost));
    for (name, title, sat) in p.committee() {
        s.push_str(&format!("{name} ({title})  {}  {sat}\n", bar(sat, 12)));
    }
    s.push('\n');
    for l in &p.log {
        s.push_str(&format!("> {l}\n"));
    }
    let (round, max) = p.round();
    if let Some(o) = p.objection() {
        s.push_str(&format!("\nRunda feedbacku {}/{}\n{}: „{}”\n\n", round + 1, max, o.who, o.text));
        s.push_str("1 - Ustąp (edge -1, appeal +1)\n");
        s.push_str(&format!("2 - Broń pomysłu (szansa {:.0}%, rośnie z wiedzą o tej osobie)\n", o.defend_odds * 100.0));
        if o.can_reframe {
            s.push_str("3 - Przeformułuj, grając na plotce (jeśli fałszywa, odbije się)\n");
        } else {
            s.push_str("3 - (brak plotki o tej osobie)\n");
        }
    }
    if let Some(o) = p.outcome() {
        s.push_str(&format!(
            "\n{}\nŚrednie zadowolenie {:.1} · kasa +{} · reputacja {:+}{}\n\nR - nowy dzień",
            o.headline,
            o.avg_satisfaction,
            o.money,
            o.reputation,
            if o.award_shot { " · SZANSA NA NAGRODĘ" } else { "" }
        ));
    }
    s
}
