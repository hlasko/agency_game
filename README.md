# Agency Sim

Symulator agencji reklamowej. Rust + Bevy 0.20. Prototyp jednego dnia przed pitchem.

## Uruchomienie

```bash
cargo run -p game            # gra (pierwsza kompilacja Bevy: kilka–kilkanaście minut)
cargo test -p sim            # testy logiki
cargo run -p sim --example balance   # przegląd wszystkich 27 kombinacji kart
```

Wymaga Rust ≥ 1.97.1. Na Linuksie: `libudev-dev libasound2-dev libwayland-dev libxkbcommon-dev`.

## Aplikacja na macOS

Na Macu (Rust ≥ 1.97.1, Xcode Command Line Tools: `xcode-select --install`):

```bash
./scripts/bundle-macos.sh             # universal (Apple Silicon + Intel), ~2x dłużej
ARCH=native ./scripts/bundle-macos.sh # tylko architektura tego Maca
```

Wynik: `dist/Agency Sim.app` i `dist/AgencySim-<wersja>.dmg`. Aplikacja ma podpis ad-hoc, bez konta deweloperskiego Apple. Na własnym Macu odpala się normalnie. Pobrana z internetu (np. z CI) przy pierwszym uruchomieniu wymaga prawego kliknięcia i „Otwórz” albo `xattr -dr com.apple.quarantine "Agency Sim.app"`.

Bez Rusta lokalnie: wypchnij repo na GitHub i uruchom workflow **macOS app** (Actions → Run workflow). `.dmg` pojawi się jako artefakt joba.

## Sterowanie

| Klawisz | Akcja |
|---|---|
| WASD / strzałki | chodzenie |
| Tab | wcielenie się w następną postać (Strateg → Kreacja → Account → Producent) |
| E | rozmowa z osobą obok · tablica z briefem · kawa przy ekspresie |
| F | weź osobę na kawę (idzie za tobą, potem E przy ekspresie) |
| J | notes: plotki, źródła, zaufanie |
| P | pitch (po wybraniu 3 kart) |
| Tablica: 1–9, Spacja | wybór karty (analizuje ją obecna postać) · dodanie do pitchu |
| Pitch: 1 / 2 / 3 | ustąp · broń · przeformułuj na bazie plotki |

## Rdzeń

- **Perspektywy.** Każda karta ma 4 warstwy prawdy. Kreacja widzi edge, Account appeal, Producent koszt, Strateg tagi i ukryte wady (np. nieaktualne dane). Żeby znać kartę w całości, trzeba ją obejrzeć czterema postaciami. Każde spojrzenie kosztuje 30 min.
- **Plotki.** Koledzy wiedzą rzeczy o komitecie klienta. Mówią, gdy zaufanie przekroczy próg. Zaufanie rośnie szybciej przy kawie i gdy rozmawiasz rolą, którą dana osoba lubi. Część plotek jest fałszywa. Potwierdzenie = to samo z dwóch źródeł.
- **Pitch.** Trzy persony z ukrytymi wagami, hot buttonami i „minami”. Rundy feedbacku: ustąpienie obcina edge, obrona to rzut kością z szansą rosnącą z prawdziwą wiedzą, reframe gra na plotce (fałszywa = odbija się).
- **Napięcie.** Bezpieczne pomysły wygrywają bez nagrody. Nagrodę daje tylko kombinacja, którą da się znaleźć, wiedząc, czego kto nie znosi.

## Architektura

```
crates/sim    czysta logika, zero zależności od silnika, deterministyczna (seed), testy
  content/demo.ron   cały content i tuning: brief, karty, persony, koledzy, plotki
crates/game   Bevy: biuro 3D, ruch, kamera, HUD, overlaye (tablica, notes, pitch)
```

Nowy brief = nowy plik RON. Balans sprawdzasz przykładem `balance`, bez odpalania gry.

## Następne kroki

1. Opcja „przerób kartę” w feedbacku (wymiana jednej karty za czas), bo dziś ustąpienie jest za słabe wobec „miny”.
2. NPC z własnym harmonogramem (kawa o 11, palarnia o 14). Plotki zależne od miejsca i pory.
3. Warstwa makro: kolejne briefy, wypalenie zespołu, reputacja przechodzi między dniami.
4. Modele postaci (glTF) zamiast kapsuł, animacje chodu.
