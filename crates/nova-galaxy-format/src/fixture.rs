//! A small, valid galaxy for tests, built in memory. Tests break one file
//! at a time with [`MemSource::with`] to provoke one problem.
//!
//! It holds the content files only. The media files its asset manifest
//! names aren't part of the format, so a crate that reads media adds them.

use crate::source::MemSource;

/// Two linked systems in one region, a station, a storyline with one
/// mission offered there and the news it releases, a small fleet with its asset manifest (see
/// [`fleet`]), and a start in the skiff in `alpha`. The manifest names `media/export/pack/hull.glb`,
/// `surface.png` and `LICENSE.txt`, which aren't included.
#[must_use]
pub fn galaxy() -> MemSource {
    fleet(MemSource::default())
        .with("regions/rim.md", "---\nname: Rim\nharshness: 3\ncolor: \"#335577\"\n---\n")
        .with("factions/league.md", "---\nname: League\nkind: minor\ncolor: \"#aa8800\"\n---\n")
        .with(
            "systems/alpha.md",
            "---\nname: Alpha\nregion: rim\ngovernment: league\nmap: [0, 0]\njump_points:\n\
             \x20 - id: to-beta\n    position: [100, 0]\n    to: beta/to-alpha\n    stability: stable\n\
             ---\nA quiet place.\n",
        )
        .with(
            "systems/beta.md",
            "---\nname: Beta\nregion: rim\nmap: [200, 50]\njump_points:\n\
             \x20 - id: to-alpha\n    position: [-100, 0]\n    to: alpha/to-beta\n    stability: unstable\n\
             \x20   owner: league\n---\n",
        )
        .with(
            "stellars/alpha-yard.md",
            "---\nname: Alpha Yard\nkind: station\nsystem: alpha\nposition: [0, 0]\nowner: league\n\
             visual: {asset: spire}\n---\n",
        )
        .with(
            "stellars/alpha-prime.md",
            "---\nname: Alpha Prime\nkind: planet\nsystem: alpha\nposition: [50, 0]\n\
             visual: {asset: surface}\n---\n",
        )
        .with(
            "starts/first-flight.md",
            "---\nname: First flight\nsystem: alpha\nship: skiff\ncash: 1000\n---\n",
        )
        .with("storylines/errands.md", "---\ntitle: Errands\nkind: minor\nfaction: league\n---\n")
        .with(
            "characters/clerk.md",
            "---\nname: Clerk\npronouns: they/them\nrole: Clerk\nhome: alpha-yard\n\
             storylines: [errands]\n---\n",
        )
        .with(
            "missions/parcel.md",
            "---\ntitle: Parcel\nstoryline: errands\nact: open\n\
             offer:\n  at: alpha-yard\n  giver: clerk\n  dialog: dialog/errands.ink#parcel\n\
             objective:\n  deliver:\n    cargo: Parcel\n    tons: 1\n    to: alpha-yard\n\
             on_success:\n  flags: [parcel_done]\n  news: [parcel-delivered]\n---\n",
        )
        .with(
            "news/parcel-delivered.md",
            "---\nheadline: Parcel delivered\nfrom: alpha\n---\nThe yard has its parcel.\n",
        )
        .with(
            "dialog/errands.ink",
            "VAR parcel_done = false\n-> walkthrough\n=== walkthrough ===\n~ parcel_done = parcel > 0\n\
             + [Parcel] -> parcel\n+ [Stop] -> END\n=== scene_end ===\n{ walkthrough:\n    -> walkthrough\n}\n\
             -> END\n=== parcel ===\nA parcel.\n* [Take it] #outcome:accept\n-> scene_end\n",
        )
}

/// An asset entry for a CC0 file downloaded from an example page.
pub const CC0: &str = "licence: CC0-1.0\nlicence_file: media/export/pack/LICENSE.txt\n\
                       author: Tester\nsource_url: https://example.com/pack\nretrieved: 2026-10-30\n";

/// A skiff (a civilian shuttle whose model, painted `primary` on its
/// `hull` material, is scaled to 8 m long), an
/// armed variant with a gun-mount kit, a blaster for its nose, a livery,
/// a planet texture, a placeholder station model and a default budget.
fn fleet(source: MemSource) -> MemSource {
    source
        .with(
            "assets/hull.md",
            &format!("---\nkind: model\nplaceholder: false\nfiles: [media/export/pack/hull.glb]\n\
                      paint: {{primary: [hull]}}\n{CC0}---\n"),
        )
        .with(
            "assets/surface.md",
            &format!("---\nkind: texture\nplaceholder: false\nfiles: [media/export/pack/surface.png]\n{CC0}---\n"),
        )
        .with("assets/spire.md", "---\nkind: model\nplaceholder: true\n---\n")
        .with(
            "budgets/default.md",
            "---\nmodel: {triangles: 20000, kib: 4096}\ntexture: {kib: 4096, side: 4096}\n---\n",
        )
        .with(
            "liveries/league-paint.md",
            "---\nname: League paint\nfaction: league\n\
             palette: {primary: \"#ddccaa\", secondary: \"#aa8800\", trim: \"#222222\"}\n---\n",
        )
        .with(
            "outfits/blaster.md",
            "---\nname: Blaster\nmount: fixed\ndamage: 5\nrange: 300\nreload: 0.5\nprice: 100\n---\n",
        )
        .with(
            "mods/gun-mount.md",
            "---\nname: Gun mount\nfits: [shuttle]\nfitted_at: lawless\nprice: 100\n\
             effects: {cargo: -2, attention: 2}\n\
             hardpoints:\n  - id: bed\n    mount: turret\n    position: [0, -1]\n---\n",
        )
        .with(
            "ships/skiff.md",
            "---\nname: Skiff\nstyle: civilian\nrole: shuttle\nmodel: hull\nlivery: league-paint\n\
             size: {length: 8, beam: 4, collision_radius: 4}\nstats: {hull: 50, cargo: 10}\n\
             hardpoints:\n  - id: nose\n    mount: fixed\n    position: [0, 3]\n    outfit: blaster\n---\n",
        )
        .with(
            "ships/armed-skiff.md",
            "---\nname: Armed skiff\nbase: skiff\nmods: [gun-mount]\n---\n",
        )
}
