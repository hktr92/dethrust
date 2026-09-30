use std::sync::Arc;

const DEGENERATE_AREA_SQUARED: f32 = 1.0e-12;
const INTERSECTION_EPSILON: f32 = 1.0e-6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceIdentity {
    /// Slash-separated identifiers from the original ACT hierarchy.
    pub actor_path: Arc<str>,
    pub model: Arc<str>,
    pub face_index: usize,
    pub material: Option<Arc<str>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CollisionTriangle {
    pub vertices: [[f32; 3]; 3],
    pub normal: [f32; 3],
    pub face_flags: u8,
    pub two_sided: bool,
    pub source: SurfaceIdentity,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RayHit {
    pub point: [f32; 3],
    pub normal: [f32; 3],
    pub distance: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChassisHit {
    pub point: [f32; 3],
    pub normal: [f32; 3],
    pub distance: f32,
    pub penetration: f32,
    pub triangle_index: usize,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct StaticCollisionWorld {
    triangles: Vec<CollisionTriangle>,
    bounds: Option<[[f32; 3]; 2]>,
    skipped_triangles: usize,
}

impl StaticCollisionWorld {
    pub fn add_triangle(
        &mut self,
        vertices: [[f32; 3]; 3],
        face_flags: u8,
        two_sided: bool,
        source: SurfaceIdentity,
    ) -> bool {
        if vertices.iter().flatten().any(|value| !value.is_finite()) {
            self.skipped_triangles += 1;
            return false;
        }
        let edge1 = subtract(vertices[1], vertices[0]);
        let edge2 = subtract(vertices[2], vertices[0]);
        let cross = cross(edge1, edge2);
        let area_squared = dot(cross, cross);
        if !area_squared.is_finite() || area_squared <= DEGENERATE_AREA_SQUARED {
            self.skipped_triangles += 1;
            return false;
        }
        let inverse_area = area_squared.sqrt().recip();
        let normal = scale(cross, inverse_area);
        for point in vertices {
            self.extend_bounds(point);
        }
        self.triangles.push(CollisionTriangle {
            vertices,
            normal,
            face_flags,
            two_sided,
            source,
        });
        true
    }

    /// Downward wheel/ground style ray using Dethrace's one-sided and `!` material rules.
    /// Face flag 0x80 is retained: upstream only enables that filter during the chassis pass.
    pub fn raycast_ground(
        &self,
        origin: [f32; 3],
        direction: [f32; 3],
        max_distance: f32,
    ) -> Option<(RayHit, &SurfaceIdentity)> {
        let (hit, index, _) = self.raycast(origin, direction, max_distance, false)?;
        Some((hit, &self.triangles[index].source))
    }

    /// Sweeps a chassis support point against collidable track faces, including start overlap.
    pub fn raycast_chassis(
        &self,
        origin: [f32; 3],
        direction: [f32; 3],
        max_distance: f32,
    ) -> Option<ChassisHit> {
        let (hit, triangle_index, penetration) =
            self.raycast(origin, direction, max_distance, true)?;
        Some(ChassisHit {
            point: hit.point,
            normal: hit.normal,
            distance: hit.distance,
            penetration,
            triangle_index,
        })
    }

    fn raycast(
        &self,
        origin: [f32; 3],
        direction: [f32; 3],
        max_distance: f32,
        chassis: bool,
    ) -> Option<(RayHit, usize, f32)> {
        if origin.iter().any(|value| !value.is_finite())
            || direction.iter().any(|value| !value.is_finite())
            || !max_distance.is_finite()
            || max_distance < 0.0
        {
            return None;
        }
        let direction_length_squared = dot(direction, direction);
        if direction_length_squared <= DEGENERATE_AREA_SQUARED && !chassis {
            return None;
        }
        let has_direction = direction_length_squared > DEGENERATE_AREA_SQUARED;
        let direction = if has_direction {
            scale(direction, direction_length_squared.sqrt().recip())
        } else {
            [0.0; 3]
        };
        // ponytail: each support ray scans every face; add a BVH if track collision becomes frame-bound.
        let mut closest = max_distance;
        let mut hit = None;
        let mut penetration = 0.0;

        for (index, triangle) in self.triangles.iter().enumerate() {
            if triangle
                .source
                .material
                .as_deref()
                .is_some_and(|name| name.starts_with('!'))
                || (chassis && triangle.face_flags & 0x80 != 0)
            {
                continue;
            }
            let [a, b, c] = triangle.vertices;

            if chassis {
                let signed_distance = dot(subtract(origin, a), triangle.normal);
                let (normal, depth) = if triangle.two_sided {
                    (triangle.normal, 0.0)
                } else if signed_distance < 0.0 {
                    (triangle.normal, -signed_distance)
                } else {
                    (triangle.normal, 0.0)
                };
                if depth > INTERSECTION_EPSILON {
                    let point = add(origin, scale(normal, depth));
                    if point_in_triangle(point, [a, b, c]) && depth > penetration {
                        closest = 0.0;
                        penetration = depth;
                        hit = Some((
                            RayHit {
                                point,
                                normal,
                                distance: 0.0,
                            },
                            index,
                            depth,
                        ));
                    }
                }
            }

            if !has_direction {
                continue;
            }
            let edge1 = subtract(b, a);
            let edge2 = subtract(c, a);
            let p = cross(direction, edge2);
            let determinant = dot(edge1, p);
            if if triangle.two_sided {
                determinant.abs() <= INTERSECTION_EPSILON
            } else {
                determinant <= INTERSECTION_EPSILON
            } {
                continue;
            }

            let inverse_determinant = determinant.recip();
            let from_a = subtract(origin, a);
            let u = dot(from_a, p) * inverse_determinant;
            if !(-INTERSECTION_EPSILON..=1.0 + INTERSECTION_EPSILON).contains(&u) {
                continue;
            }
            let q = cross(from_a, edge1);
            let v = dot(direction, q) * inverse_determinant;
            if v < -INTERSECTION_EPSILON || u + v > 1.0 + INTERSECTION_EPSILON {
                continue;
            }
            let distance = dot(edge2, q) * inverse_determinant;
            if distance < 0.0 || distance > closest {
                continue;
            }

            let replaces_hit = match hit {
                None => true,
                Some((previous, _, previous_penetration)) => {
                    previous_penetration == 0.0 && distance < previous.distance
                }
            };
            if replaces_hit {
                let mut normal = triangle.normal;
                if dot(normal, direction) > 0.0 {
                    normal = scale(normal, -1.0);
                }
                closest = distance;
                penetration = 0.0;
                hit = Some((
                    RayHit {
                        point: add(origin, scale(direction, distance)),
                        normal,
                        distance,
                    },
                    index,
                    0.0,
                ));
            }
        }
        hit
    }

    pub fn triangles(&self) -> &[CollisionTriangle] {
        &self.triangles
    }

    pub fn bounds(&self) -> Option<[[f32; 3]; 2]> {
        self.bounds
    }

    pub fn skipped_triangle_count(&self) -> usize {
        self.skipped_triangles
    }

    fn extend_bounds(&mut self, point: [f32; 3]) {
        let Some(bounds) = &mut self.bounds else {
            self.bounds = Some([point, point]);
            return;
        };
        for axis in 0..3 {
            bounds[0][axis] = bounds[0][axis].min(point[axis]);
            bounds[1][axis] = bounds[1][axis].max(point[axis]);
        }
    }
}

fn point_in_triangle(point: [f32; 3], triangle: [[f32; 3]; 3]) -> bool {
    let [a, b, c] = triangle;
    let v0 = subtract(c, a);
    let v1 = subtract(b, a);
    let v2 = subtract(point, a);
    let dot00 = dot(v0, v0);
    let dot01 = dot(v0, v1);
    let dot02 = dot(v0, v2);
    let dot11 = dot(v1, v1);
    let dot12 = dot(v1, v2);
    let denominator = dot00 * dot11 - dot01 * dot01;
    if !denominator.is_finite() || denominator.abs() <= DEGENERATE_AREA_SQUARED {
        return false;
    }
    let inverse = denominator.recip();
    let u = (dot11 * dot02 - dot01 * dot12) * inverse;
    let v = (dot00 * dot12 - dot01 * dot02) * inverse;
    u >= -INTERSECTION_EPSILON && v >= -INTERSECTION_EPSILON && u + v <= 1.0 + INTERSECTION_EPSILON
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn subtract(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(vector: [f32; 3], factor: f32) -> [f32; 3] {
    [vector[0] * factor, vector[1] * factor, vector[2] * factor]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[cfg(test)]
mod tests {
    use super::{StaticCollisionWorld, SurfaceIdentity};
    use std::sync::Arc;

    fn source(material: Option<&str>) -> SurfaceIdentity {
        SurfaceIdentity {
            actor_path: Arc::from("TRACK/ROAD"),
            model: Arc::from("ROAD_MODEL"),
            face_index: 3,
            material: material.map(Arc::from),
        }
    }

    #[test]
    fn ray_hits_nearest_one_sided_ground_triangle_and_reports_identity() {
        let mut world = StaticCollisionWorld::default();
        world.add_triangle(
            [[-1.0, 0.0, -1.0], [0.0, 0.0, 1.0], [1.0, 0.0, -1.0]],
            0,
            false,
            source(Some("ROAD")),
        );
        world.add_triangle(
            [[-1.0, -1.0, -1.0], [0.0, -1.0, 1.0], [1.0, -1.0, -1.0]],
            0,
            false,
            source(Some("LOWER")),
        );

        let (hit, identity) = world
            .raycast_ground([0.0, 2.0, 0.0], [0.0, -2.0, 0.0], 5.0)
            .unwrap();
        assert_eq!(hit.point, [0.0, 0.0, 0.0]);
        assert_eq!(hit.normal, [0.0, 1.0, 0.0]);
        assert_eq!(hit.distance, 2.0);
        assert_eq!(identity.material.as_deref(), Some("ROAD"));
        assert_eq!(identity.actor_path.as_ref(), "TRACK/ROAD");
        assert_eq!(
            world.bounds().unwrap(),
            [[-1.0, -1.0, -1.0], [1.0, 0.0, 1.0]]
        );
    }

    #[test]
    fn wall_rays_obey_winding_unless_material_is_two_sided() {
        let wall = [[0.0, -1.0, -1.0], [0.0, 1.0, -1.0], [0.0, 1.0, 1.0]];
        let mut one_sided = StaticCollisionWorld::default();
        one_sided.add_triangle(wall, 0, false, source(None));
        assert!(
            one_sided
                .raycast_ground([-2.0, 0.0, 0.0], [1.0, 0.0, 0.0], 4.0)
                .is_none()
        );
        assert!(
            one_sided
                .raycast_ground([2.0, 0.0, 0.0], [-1.0, 0.0, 0.0], 4.0)
                .is_some()
        );

        let mut two_sided = StaticCollisionWorld::default();
        two_sided.add_triangle(wall, 0, true, source(Some("GLASS")));
        let (hit, _) = two_sided
            .raycast_ground([-2.0, 0.0, 0.0], [1.0, 0.0, 0.0], 4.0)
            .unwrap();
        assert_eq!(hit.normal, [-1.0, 0.0, 0.0]);
    }

    #[test]
    fn excludes_special_materials_and_skips_degenerate_triangles() {
        let mut world = StaticCollisionWorld::default();
        world.add_triangle(
            [[-1.0, 0.0, -1.0], [0.0, 0.0, 1.0], [1.0, 0.0, -1.0]],
            0,
            false,
            source(Some("!WATER")),
        );
        assert!(!world.add_triangle(
            [[0.0; 3], [1.0; 3], [2.0; 3]],
            0,
            false,
            source(Some("ROAD")),
        ));
        assert_eq!(world.skipped_triangle_count(), 1);
        assert!(
            world
                .raycast_ground([0.0, 1.0, 0.0], [0.0, -1.0, 0.0], 2.0)
                .is_none()
        );
    }

    #[test]
    fn chassis_rays_skip_flagged_faces_and_return_triangle_identity() {
        let mut world = StaticCollisionWorld::default();
        let wall = [[0.0, -2.0, -2.0], [0.0, 2.0, -2.0], [0.0, 2.0, 2.0]];
        world.add_triangle(wall, 0x80, true, source(Some("NO_COLLISION")));
        world.add_triangle(wall, 0, true, source(Some("!INVISIBLE")));
        world.add_triangle(wall, 0, true, source(Some("WALL")));

        let hit = world
            .raycast_chassis([-1.0, 0.0, 0.0], [2.0, 0.0, 0.0], 2.0)
            .unwrap();
        assert_eq!(hit.triangle_index, 2);
        assert_eq!(
            world.triangles()[hit.triangle_index]
                .source
                .material
                .as_deref(),
            Some("WALL")
        );
        assert_eq!(hit.normal, [-1.0, 0.0, 0.0]);
    }

    #[test]
    fn chassis_rays_report_and_filter_start_penetration() {
        let mut world = StaticCollisionWorld::default();
        let wall = [[0.0, -2.0, -2.0], [0.0, 2.0, -2.0], [0.0, 2.0, 2.0]];
        world.add_triangle(wall, 0, false, source(Some("WALL")));
        let hit = world
            .raycast_chassis([-0.25, 0.0, 0.0], [0.0; 3], 0.0)
            .unwrap();
        assert!((hit.penetration - 0.25).abs() < 1.0e-6);
        assert_eq!(hit.point[0], 0.0);
        assert_eq!(hit.normal, [1.0, 0.0, 0.0]);

        let mut no_collision = StaticCollisionWorld::default();
        no_collision.add_triangle(wall, 0x80, false, source(Some("WALL")));
        assert!(
            no_collision
                .raycast_chassis([-0.25, 0.0, 0.0], [0.0; 3], 0.0)
                .is_none()
        );
    }

    #[test]
    fn rejects_invalid_rays_without_panicking() {
        assert!(
            StaticCollisionWorld::default()
                .raycast_ground([f32::NAN, 0.0, 0.0], [0.0, -1.0, 0.0], 1.0)
                .is_none()
        );
        assert!(
            StaticCollisionWorld::default()
                .raycast_ground([0.0; 3], [0.0; 3], 1.0)
                .is_none()
        );
    }
}
