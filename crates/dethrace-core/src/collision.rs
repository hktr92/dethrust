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
        if origin.iter().any(|value| !value.is_finite())
            || direction.iter().any(|value| !value.is_finite())
            || !max_distance.is_finite()
            || max_distance < 0.0
        {
            return None;
        }
        let direction_length_squared = dot(direction, direction);
        if direction_length_squared <= DEGENERATE_AREA_SQUARED {
            return None;
        }
        let direction = scale(direction, direction_length_squared.sqrt().recip());
        let mut closest = max_distance;
        let mut hit = None;

        for triangle in &self.triangles {
            if triangle
                .source
                .material
                .as_deref()
                .is_some_and(|name| name.starts_with('!'))
            {
                continue;
            }
            let [a, b, c] = triangle.vertices;
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

            let mut normal = triangle.normal;
            if dot(normal, direction) > 0.0 {
                normal = scale(normal, -1.0);
            }
            closest = distance;
            hit = Some((
                RayHit {
                    point: add(origin, scale(direction, distance)),
                    normal,
                    distance,
                },
                &triangle.source,
            ));
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
