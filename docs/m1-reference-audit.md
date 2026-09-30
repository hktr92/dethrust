# Milestone 1 reference audit

The behavioral reference is `.reference/dethrace`, including its BRender 1.3.2
submodule. The GOG game data used for this audit is supplied separately through
`CARMAGEDDON_DIR`; neither tree is a runtime or Git dependency.

## End-race Damage Gallery

`racesumm.c` calls `DoEndRaceSummary2` after the first summary. It builds the
gallery with `BuildWrecks`, runs `DoInterfaceScreen`, draws each frame with
`DamageScrnDraw`, and restores the car actors with `DisposeWrecks`. The gallery
uses FLIC ID 320 (`SUM2STIL.FLI`) as its still background. IDs 321/322 are the
button highlight FLICs (`BGBUT8GL.FLI`/`BGBUT8FL.FLI`), 323/324 are the Done
and Back pushes (`DNBUT8IN.FLI`/`BKBUT8IN.FLI`), and 325/326 change Back while
zooming (`BKBUTOFF.FLI`/`BKBUTON.FLI`). See `flicplay.c`'s FLIC table and
`DoEndRaceSummary2`'s interface specification. The button anchors are (9,174)
and (247,174) in 320x200 coordinates.

The 320x200 `gGraf_data[0]` entry in `grafdata.c`, interpreted through
`tGraf_data` in `dr_types.h`, gives the 3D rectangle `(11,20,298,149)` and
the selected-name box `left=83, right=238, top=178, bottom=191`, with text
baseline 181. The 3D area is cleared to grey and marked with a 15-pixel grid.
The camera has aspect 2, field of view 55 degrees, and starts at `(0,0,2.2)`.

`BuildWrecks` uses the current player's principal actor followed by loaded
opponent cars (`GetCarCount`/`GetCarSpec`). For zero-based position `n`, it sets
`x = 1.5 * ((n % 3) - 1)`, `y = -1.2 * ((n / 3) - 0.5)`, and `z = 0`.
Each car's scale is `0.47 / radius`, where radius is the greatest distance
from the principal model origin to a vertex. `SpinWrecks` rotates non-custom
cars about Y by `0.05 * elapsed_milliseconds` degrees; a modern scene can keep
rotation and scale separate to avoid the legacy matrix normalization loop.

The gallery starts with car 0 selected. `DamageScrnLeft/Right` wrap within a
row; `Up/Down` select the nearest column in the neighboring row and can enter
the button row. `DamageScrnGoHead` zooms or activates Done. `ClickDamage`
selects with a scene ray, then uses a rolling-ball mouse rotation while zoomed.
`DamageScrnDraw` moves the camera over one second toward the selected actor,
with its Z offset at -1.45 before adding the normal 2.2 camera distance.
`GetDriverName` displays the player name or selected opponent's name. Back
returns from zoom; Done exits. The legacy idle path also auto-zooms the first
car after 1.5 seconds. These behaviors are in `racesumm.c:563-1307`.

## Maim Street and its cars

`loading.c:2552-2634` opens `RACES.TXT` for the ordinary single-player race
list. Its first race is `Maim Street`; the adjacent race metadata names
`CITYA1.TXT`. `LoadRaceInfo` reads that track filename. The race file lives
under `DATA/RACES/`. Original text lines beginning with `@` are encoded;
`utility.c:93-178` and `GetALineWithNoPossibleService` define the decode step.
Raw text search of the shipped `RACES.TXT` therefore cannot find the race name.

There is **no fixed Maim Street opponent set in `RACES.TXT`**. For single-player
play, `structur.c:362-420` computes a rank band from the selected race's
suggested rank, reads five desired strength ratings from `gOpponent_mix`, and
randomly chooses eligible unpicked entries in `OPPONENT.TXT`. It excludes the
current player's car and limits auto-scum to one. `loading.c:3309-3327` then
uses each chosen opponent's `car_file_name` to load `CARS/*.TXT`. The debug
gallery must make its rank/selection seed explicit and resolve filenames from
these records, rather than assume one historical playthrough's random draw.
For the player, `GENERAL.TXT` supplies the two basic car names, read in
`loading.c:480-481`; the first is the starting Max car. The player label in a
direct debug scene needs an explicit default because no player profile exists.

`LoadCar` reads the resolution-independent `CARS/<name>.TXT` and a
resolution-dependent file under the active graphics data directory, falling
back to the first basic car there. Its visual section near
`loading.c:2215-2335` selects one of the pixelmap and material sets, reads
shade tables, models, and actor variants. Each variant has a distance and
`ACTORS/<name>.ACT`; the zero-distance variant is the principal car actor.
`LinkModelsToActor` resolves actor model names against the models loaded for
that car. Cockpit, mechanics, damage, and driver HUD sections precede the
visual section, but are not needed to render it.

## Maim Street track visuals

`world.c:2600-2780` opens `DATA/RACES/CITYA1.TXT`, reads its version and
skips start-grid/checkpoint fields before the visual groups. The supplied file
is version 6. Its normal-resolution group names PIX files including
`NYSKY1.PIX`, `CITYA2.PIX`, and `NYHORIZN.PIX`; MAT files include
`GRIDDY.MAT`, `STADY.MAT`, `WATTY.MAT`, and `CITYA2.MAT`.
The main model is `CITYANW1.DAT`, and the main actor is `CITYANW1.ACT`.
The file also names an alternate lower-detail group, an additional actor
`CITYA1X.ACT`, a sky texture, and later gameplay systems. The visual path is
the chosen PIX set, MAT set, main DAT, main ACT, and any additional visible
models/actors actually referenced by the chosen track presentation.
`LoadNPixelmaps`, `LoadNMaterials`, `LoadNTrackModels`, and `BrActorLoad`
provide the loading order. `world.c` later links the track actor's model names,
processes columns, and mounts the hierarchy under the universe actor.

The version-6 normal-resolution `CITYA1.TXT` dependency list resolved from
`RACES.TXT` is:

- PIX: `NYSKY1.PIX`, `CITYA2.PIX`, `NYHORIZN.PIX`, `DRKSCRN.PIX`, `FOGSCRN.PIX`
- MAT: `GRIDDY.MAT`, `STADY.MAT`, `WATTY.MAT`, `NYSKY1.MAT`,
  `CITYA2.MAT`, `DRKSCRN.MAT`, `FOGSCRN.MAT`, `SKIDMARK.MAT`
- DAT: `CITYANW1.DAT`
- ACT: `CITYANW1.ACT`

The start position is `(159.94, -8.5, -225.67)` with yaw `180` degrees.
`CITYA1X.ACT`/`.DAT` name an additional-object save path in `world.c`; their
shipped files contain no static street geometry. The normal group above is the
visual graph needed for the static viewer. Every named file exists in the
supplied original installation.

## Source formats and scope

| Input | Canonical loader | M1 subset | Deferred |
| --- | --- | --- | --- |
| Encoded TXT | `utility.c` line decode; `loading.c` and `world.c` readers | Race name/track, basic player car, opponent records and selection data, visual file lists and actor variant names | Race progression, cockpit, mechanics, checkpoints, damage, AI |
| PIX | BRender `core/pixelmap/pmfile.c`, `BrPixelmapLoadMany` | Checked chunks, identifiers, indexed pixels/palette, multiple maps | Shade-table lighting and unsupported pixel types unless target assets require them |
| MAT | BRender `core/v1db/v1dbfile.c`, `BrMaterialLoadMany` | Colour, flags, texture reference, mapping/transparency data used by target assets | Exact indexed shade-table renderer |
| DAT | BRender `core/v1db/v1dbfile.c`, `BrModelLoadMany` | Vertices, faces, UVs, face materials, model names and bounds | Collision and crush deformation |
| ACT | BRender `core/v1db/v1dbfile.c`, `BrActorLoadMany` | Names, hierarchy, local transforms, model/material references, render style | BRender global registries and nonvisual actor behavior |
| FLI/FLC | `flicplay.c`; already parsed in `dethrace-formats` | Gallery background and button visuals | Original palette lighting effects |

The BRender loaders share chunk and structured-field machinery in
`core/fw/datafile.c` and `core/fw/genfile.c`. Neutral parsed objects should be
linked once by source identifiers; Bevy conversion and a single coordinate
mapping belong in `dethrace-assets`.
