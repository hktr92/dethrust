# Milestone 2 vehicle and collision reference audit

This audit uses the canonical checkout at .reference/dethrace and the existing
M1 parsers and asset bridge. No M2 simulation code existed at the start of this
run. The existing default workspace tests and opt-in original-asset tests pass.

## Update and control flow

The original main loop calls PollCarControls, then ControlOurCar, then
ApplyPhysicsToCars (mainloop.c). PollCarControls writes digital and joystick
state. ControlOurCar calls one of ControlCar1 through ControlCar5 with the
render-frame duration in seconds. Those functions update steering curvature
and accelerator force; they do not integrate the world pose.

ApplyPhysicsToCars advances mechanics on PHYSICS_STEP_TIME boundaries. Each
step calls MoveAndCollideCar, which updates the engine and forces, wheel and
tyre response, angular pose, translation, and wall collision. For a driven car,
the relevant sequence is CalcEngineForce, CalcForce, DoRevs, RotateCar,
TranslateCar, then CollideCarWithWall. CalcForce accumulates gravity, wheel
spring and damping forces, tyre forces, and their torques about the centre of
mass. This order is behavioral evidence; M2 should reproduce the mechanics
with explicit Rust state and an input boundary, not preserve these C routines.

CalcEngineForce uses throttle and a gear state. Deceleration can request
reverse once speed is low; otherwise it contributes braking. DoRevs selects
forward or reverse gear and updates engine speed. The legacy control struct
also has a brake bit that forces the gear to neutral, but it has no distinct
handbrake field. M2 therefore needs an explicit handbrake input and a small,
testable mapping to the vehicle model.

## Mechanics data needed by M2

ReadMechanicsData in loading.c reads a versioned section from the player car
definition. The M2 subset is:

| Source values | Use in M2 |
| --- | --- |
| Four wheel positions, centre-of-mass offset, body bounds, and up to six extra points | Vehicle pose, wheel placement, chassis bounds, and contact queries |
| Mass and body length, width, and height | Linear acceleration and the original loader's principal inertia calculation |
| Front/rear suspension give, damping, and body ride-height reference | Two axle suspension tunes; SetCarSuspGiveAndHeight derives the travel, spring, and damping state |
| Steering curve limit and three grip-angle values | Steering limits and base tyre force limits |
| Versioned rolling resistance, gear count, speed/revs ratio input, engine-force input, and brake settings | Basic forward/reverse drive, coast, and braking |
| Version 2+ friction ellipticity and downforce speed | Combined tyre-force limit and speed-dependent load where the parsed track surface supports it |

The reader derives max curvature, principal inertia, grip coefficients, brake
force defaults, and drivetrain ratios from those values. It scales mechanics
coordinates through WORLD_SCALE (6.9); preserve source values in the parser
and perform that conversion once at the mechanics-to-simulation boundary.
Suspension travel and spring/damper coefficients are finalized by
SetCarSuspGiveAndHeight after the text is read.

Do not parse the adjacent cockpit, damage, powerup, AI, network, or career
sections for M2. Additional engine or surface data belongs in the subset only
if the selected Maim Street drive model needs it.

## Wheel contact and suspension

The original car stores four wheel positions and queries the world from those
positions with MultiFindFloorInBoxM. The query checks the candidate faces
around the car and returns a distance, normal, and material index per wheel.
CalcForce uses the front or rear suspension travel, previous compression, and
damping to calculate a nonnegative spring force. It applies each contact force
at the wheel location, so the offset from centre of mass also produces pitch
and roll torque. Gravity is approximately mass times 10 in the original
simulation units, subject to the race gravity multiplier.

M2 should keep four wheel states with the query result, grounded flag,
compression and travel, and normal. A testable force step should combine
those contacts with gravity and apply the resulting force and torque to
VehicleState. Tyre forces should consume the same contact and surface data;
they should not move a Bevy transform.

## Maim Street static collision source

M1 resolves Maim Street through CITYA1.TXT and loads its linked
CITYANW1.ACT/CITYANW1.DAT graph. The original LoadTrack installs that ACT as
gTrack_actor, and GetFacesInBox gathers nearby faces from that actor for wheel
and chassis queries. The additional CITYA1X actor path does not provide the
static street geometry in the supplied data.

The original code does not treat all rendered faces as identical collision
surfaces. Face flags can exclude a face from ground queries; material flags
control one-sided tests; material identifiers select surface behavior; and
special material prefixes receive separate handling. Keep the source actor,
model, face, and material identity on collision triangles. Build the M2 query
world from the already parsed M1 graph and retain these source references for
debugging. Apply the relevant face and material rules rather than inferring
collision solely from visibility.

The upstream chassis path broad-phases against an expanded/swept vehicle
bounds box, tests candidate world faces, then repeats wall response while
penetration remains. It uses the same static track actor as wheel queries.
M2 needs explicit ground ray/shape queries plus chassis overlap or sweep
queries through one CollisionWorld boundary. Dynamic car-to-car response is
outside this milestone.

## Time and coordinates

The reference defines PHYSICS_STEP_TIME as 40 ms and PHYSICS_STEP_COUNT as
five. ApplyPhysicsToCars advances at 0.04 seconds per mechanics step and may
run up to five catch-up steps in one outer update. Input and steering are
sampled from the render-frame duration; mechanics integration uses the fixed
step. The harness can opt into a per-frame variant, which is not the normal
game path.

Start M2 at the reference 40 ms step (25 Hz), with the duration owned by a
testable simulation configuration. Use the same duration for each core step,
keep render delta out of vehicle integration, and bound catch-up work to five
steps per outer update. Tests should call the simulation step directly. If
later stability or handling comparisons justify a different frequency, record
the evidence and keep the rate configurable.

BRender source and M1 coordinates use X right, Y up, and Z depth; the M1 bridge
preserves these axes and maps the serialized matrix basis into Bevy columns.
The car's local forward is -Z; Maim Street's 180-degree start yaw points it
toward +Z. Track start coordinates remain in source-world units. Mechanics
coordinates use the reference's 6.9 scale during dynamics, so M2 must keep
track positions, car presentation transforms, and mechanics conversion
consistent without scattering scale factors through rendering code.

## Selected M2 architecture

Use a Dethrust-owned vehicle simulation and a Dethrust-owned static collision
query layer. Keep DriverInput, VehicleState, VehicleConfig, and the direct
fixed-step API in a presentation-independent boundary. Build the Maim Street
static world once from the neutral M1 ACT/DAT/MAT data. The Bevy game layer
adapts its queries and schedules the fixed step; a stable simulation root owns
the vehicle pose while the car actor hierarchy remains presentation below it.

This matches the original split between wheel forces, tyre response, rotation,
and collision while keeping each operation inspectable and testable. The
parsed source faces and materials also provide the identity and filtering
needed for Carmageddon-specific contacts.

| Option | Assessment |
| --- | --- |
| Dethrust vehicle integration and collision queries | Selected. It preserves the custom four-wheel model, source-face rules, neutral DriverInput path, and direct tests without a new dependency. |
| Dethrust vehicle model with a physics crate only for static queries | Possible if measured query cost or robust shape sweeps justify it. Keep that behind CollisionWorld; current M1 data and requirements do not establish a need for a dependency. |
| Broader rigid-body integration through a physics crate | Not selected. Letting a second solver own gravity, chassis contacts, and motion would compete with the custom suspension and tyre forces that define the M2 handling path. |

Human input is the only M2 input source. Human, AI, replay, and network
controllers should eventually supply the same DriverInput to one simulation.
Input systems produce neutral input and discrete commands; simulation state
drives presentation after the fixed step. Do not create a parallel Bevy
transform controller.

## Explicit deferrals

- Opponent vehicles and car-to-car collision
- Pedestrians and race/checkpoint progression
- Damage, crushing, and damage-derived handling
- Powerups, scoring, credits, and Action Replay
- Save/load, final HUD, and final audio
- Full original recovery economics and career rules
