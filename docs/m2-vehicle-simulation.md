# M2 vehicle simulation comparison

The M2 pass uses the canonical `.reference/dethrace` source (`car.c`,
`controls.c`, and the physics timing definitions) to compare the mechanics path
with the original BLKEAGLE on Maim Street. The repeatable trace lives in the
ignored `original_player_spawn` integration test and reads the user's installed
track and car data. Its numbers below are from Dethrust running those assets;
they are not measurements from the original executable.

## Time and units

The simulation advances at the reference 40 ms mechanics step. `advance_frame`
queues excess frame time and runs at most five fixed steps per update. Gravity
is `[0, -10, 0]` mechanics units/s². Integration does not use render delta.

Track positions and collision vertices stay in original source-world units.
At the mechanics boundary, car positions, bounds, wheel positions, suspension
travel, and centre of mass use the original `6.9` scale; principal inertia uses
`47.61`. The M1 bridge converts these values once and retains parsed mechanics
values such as mass, maximum curvature, gear data, and grip angles.

## Contact, suspension, tyres, and drive

Each of the four original wheel positions casts a downward ground query using
the parsed track faces. The query uses a two mechanics-unit reach and a half-unit
sweep margin. Front/rear travel and ride height come from the car mechanics.
Spring and damping rates use the upstream factor of five, mass split by axle
lever arms, and the parsed damping value. Each normal force is applied at the
wheel lever arm, so suspension compression and contact load create body pitch
and roll. Missing wheel contacts contribute no suspension force.

The current tyre model applies rear-wheel drive, steering geometry at the front,
rolling resistance, per-wheel braking, a rear handbrake, and a combined
longitudinal/lateral force limit. It derives base grip from the original grip
angles and reduces rear lateral grip toward the parsed compression grip as slip
rises. It uses no material-specific grip or speed downforce. Engine revs follow
wheel speed; the model has no separate engine inertia state.

The chassis uses the parsed bounds as an oriented box sampled at eight corners
and six face centres. It sweeps those 14 points through translation and splits
larger rotations into angular segments. The earliest face contact receives
penetration correction and an impulse with fixed friction. Original triangle
flags, one-sided faces, and special material prefixes are respected. A sweep
start overlap deeper than the segment cannot hide a nearer face; a hull wholly
behind a one-sided face does not count as embedded in that backface. This is a
sampled static-track collision model, not a full polyhedron solver. It does not
collide with other cars.

## Original-asset trace

The ignored Maim Street test settles BLKEAGLE for 10 seconds, then runs the
listed controls for fixed 40 ms steps. Speeds are magnitudes in source-world
units/s. Compression values are mechanics units in source wheel order.

| Maneuver | Dethrust trace |
| --- | --- |
| Standing throttle | 3.225 at 1.6 s; 6.151 at 4.8 s |
| Low-speed full steering + throttle | 2.213 speed; −73.53° yaw; 0.04° pitch, 0.48° roll; 4 slipping wheels |
| High-speed full steering + throttle | 6.151 to 6.348 speed over 1.6 s; −20.82° yaw; 0.01° pitch, 0.41° roll; 2 slipping wheels |
| High-speed handbrake turn | 6.151 to 2.104 speed over 1.6 s; −32.76° yaw; 4 slipping wheels |
| Full braking | From 6.151, stopped below 0.1 in 43 steps (1.72 s), over 5.208 source units |
| Reverse | −3.184 speed after 1.6 s of reverse input |
| Ordinary-speed window impact | 10.000 to 0.022 speed in one step against original `BRNWINDW.MAT` face |

Low- and high-speed steering traces include four-wheel suspension compression
respectively `[0.07698, 0.06318, 0.07443, 0.06064]` and
`[0.07505, 0.06334, 0.07459, 0.06288]`. The trace also asserts ten seconds of
neutral settling stays within one source unit of the resolved Maim start, with
all wheels grounded and low residual motion.

## Differences and M2 boundary

Dethrace's `ControlCar1` through `ControlCar5` ramp curvature and turn speed
using control-frame time, vehicle speed, and self-centering. M2 currently maps
normalized steering directly to parsed maximum curvature at each mechanics
step. The reference also evolves engine/brake response in `CalcEngineForce`
and engine state in `DoRevs`; M2 uses wheel-speed revs without engine inertia
and applies parsed initial-plus-increase brake force together. Its tyre-force
limit is deliberately simpler than `CalcForce`, with no surface-specific grip
or speed downforce. These are model differences, not Maim-specific tuning.

Manual M2 recovery immediately restores a collision-clear recent safe pose (or
the start pose), raises it if needed, and clears velocity and drivetrain state.
It also recovers automatically after three seconds upside-down or below the
track bounds. The original schedules recovery with a delay and career/network
rules such as cost and cancellation; those rules remain outside M2.

The comparison fixed a real chassis-query error: a distant start overlap could
win at distance zero and suppress a face the car reached during the current
sweep. The query now ignores overlap deeper than that sweep length. A second
regression covers a one-sided triangle wholly behind the chassis. The opt-in
Maim test exercises both cases through the original track geometry.
