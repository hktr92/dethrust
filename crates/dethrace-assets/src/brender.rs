//! Shared BRender visual bridge for car and track scenes.
//!
//! BRender's vectors are right handed with X right, Y up, and the gallery
//! camera at positive Z looking toward the origin. Bevy uses the same axes.
//! BRender matrix34 stores basis vectors in rows for row-vector multiplication;
//! Bevy stores those basis vectors in columns for column-vector multiplication.
//! Face vertex order is retained: BRender `BrPlaneEquation` and Bevy both use
//! `(v1-v0) x (v2-v0)` for the front normal.
//! BRender shade tables, environment mapping, and dynamic lighting are deferred;
//! M1 uses unlit Bevy materials with source colors, opacity, UVs, and textures.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::image::{Image, ImageSampler};
use bevy::mesh::{Mesh, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, Face as CullFace, TextureDimension, TextureFormat};
use dethrace_core::collision::{StaticCollisionWorld, SurfaceIdentity};
use dethrace_core::vehicle::VehicleConfig;
use dethrace_formats::{
    act::{ActFile, Actor, ActorTransform},
    car_mechanics::CarMechanicsSpec,
    car_visual::{CarVisualSpec, VisualVariant},
    dat::{DatFile, Model},
    mat::{MatFile, Material},
    pix::{PixFile, PixelType, Pixelmap},
    race::{GalleryRoster, OpponentCatalog, RaceCatalog, initial_player},
    track_visual::TrackVisualSpec,
};

use crate::GameDir;

pub use dethrace_formats::car_mechanics::{MECHANICS_INERTIA_SCALE, MECHANICS_WORLD_SCALE};

fn key(name: &str) -> String {
    name.to_ascii_uppercase()
}

fn read(path: &std::path::Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn actor_model_names(actor: &ActFile) -> HashSet<String> {
    let mut needed = HashSet::new();
    let mut actors: Vec<_> = actor.roots.iter().collect();
    while let Some(node) = actors.pop() {
        if let Some(name) = &node.model {
            needed.insert(key(name));
        }
        actors.extend(&node.children);
    }
    needed
}

pub fn source_point(point: [f32; 3]) -> Vec3 {
    Vec3::from_array(point)
}

pub fn source_transform(transform: &ActorTransform) -> Transform {
    match transform {
        ActorTransform::Identity => Transform::IDENTITY,
        ActorTransform::Translation(p) => Transform::from_translation(source_point(*p)),
        ActorTransform::Matrix34(rows) => Transform::from_matrix(Mat4::from_cols(
            source_point(rows[0]).extend(0.0),
            source_point(rows[1]).extend(0.0),
            source_point(rows[2]).extend(0.0),
            source_point(rows[3]).extend(1.0),
        )),
    }
}

pub fn palette_from_pix(pal: &Pixelmap) -> Result<[[u8; 3]; 256], String> {
    if pal.pixel_type != PixelType::Rgbx888
        || pal.width != 1
        || pal.height != 256
        || pal.row_bytes != 4
        || pal.pixels.len() != 1024
    {
        return Err(format!("{} is not a 1x256 RGBX palette", pal.identifier));
    }
    let mut colors = [[0; 3]; 256];
    for (color, pixel) in colors.iter_mut().zip(pal.pixels.as_chunks::<4>().0.iter()) {
        // BRender block data is stored big endian as X,R,G,B.
        *color = [pixel[1], pixel[2], pixel[3]];
    }
    Ok(colors)
}

pub fn pixelmap_to_image(map: &Pixelmap, palette: &[[u8; 3]; 256]) -> Result<Image, String> {
    let width = usize::from(map.width);
    let height = usize::from(map.height);
    let stride = usize::from(map.row_bytes);
    let mut rgba = Vec::with_capacity(
        width
            .checked_mul(height)
            .and_then(|n| n.checked_mul(4))
            .ok_or("image size overflow")?,
    );
    for row in map.pixels.chunks_exact(stride).take(height) {
        for x in 0..width {
            let pixel = match map.pixel_type {
                PixelType::Index8 => {
                    let color = palette[usize::from(row[x])];
                    [color[0], color[1], color[2], 255]
                }
                PixelType::Rgb888 => {
                    let at = x * 3;
                    [row[at + 2], row[at + 1], row[at], 255]
                }
                PixelType::Rgbx888 => {
                    let at = x * 4;
                    [row[at + 1], row[at + 2], row[at + 3], 255]
                }
                PixelType::Rgba8888 => {
                    let at = x * 4;
                    [row[at + 1], row[at + 2], row[at + 3], row[at]]
                }
            };
            rgba.extend(pixel);
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: u32::from(map.width),
            height: u32::from(map.height),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    Ok(image)
}

pub struct VisualScene {
    pub actor: ActFile,
    pub pixelmaps: Vec<PixFile>,
    pub materials: Vec<MatFile>,
    pub models: Vec<DatFile>,
    pub palette: [[u8; 3]; 256],
}

/// Original player car visuals and source mechanics resolved from GENERAL.TXT.
pub struct PlayerCarSources {
    pub file: String,
    pub mechanics: CarMechanicsSpec,
    pub scene: VisualScene,
}

impl PlayerCarSources {
    pub fn initial(dir: &GameDir) -> Result<Self, String> {
        let general_path = dir.data_path("GENERAL.TXT")?;
        let (_, file) = initial_player(&read(&general_path)?)
            .map_err(|error| format!("{}: {error}", general_path.display()))?;
        Self::load(dir, &file)
    }

    pub fn load(dir: &GameDir, file: &str) -> Result<Self, String> {
        let path = dir.asset_path("CARS", file)?;
        let bytes = read(&path)?;
        let mechanics = CarMechanicsSpec::parse(&bytes)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let visual =
            CarVisualSpec::parse(&bytes).map_err(|error| format!("{}: {error}", path.display()))?;
        let scene = VisualScene::car(dir, &visual, VisualVariant::Low)?;
        Ok(Self {
            file: file.to_owned(),
            mechanics,
            scene,
        })
    }

    /// Converts source mechanics to the units used by the original mechanics loop.
    pub fn vehicle_config(&self) -> Result<VehicleConfig, String> {
        let principal = self
            .scene
            .actor
            .roots
            .first()
            .ok_or("principal car ACT has no root actor")?;
        let actor_offset = match &principal.transform {
            ActorTransform::Identity => [0.0; 3],
            ActorTransform::Translation(position) => *position,
            ActorTransform::Matrix34(rows) => rows[3],
        };
        let mut bounds = self.mechanics.bounds;
        for point in &mut bounds {
            for axis in 0..3 {
                point[axis] += actor_offset[axis];
            }
        }
        let ride_height = bounds[0][1] + 0.01;
        let config = VehicleConfig {
            mass: self.mechanics.mass,
            center_of_mass: self
                .mechanics
                .center_of_mass
                .map(|value| value * MECHANICS_WORLD_SCALE),
            principal_inertia: self
                .mechanics
                .principal_inertia
                .map(|value| value * MECHANICS_INERTIA_SCALE),
            wheel_positions: std::array::from_fn(|index| {
                let mut position = self.mechanics.wheel_positions[index];
                position[1] = ride_height;
                position.map(|value| value * MECHANICS_WORLD_SCALE)
            }),
            bounds: bounds.map(|point| point.map(|value| value * MECHANICS_WORLD_SCALE)),
            ride_height: ride_height * MECHANICS_WORLD_SCALE,
            suspension_travel: [
                self.mechanics.suspension_give[1] * MECHANICS_WORLD_SCALE,
                self.mechanics.suspension_give[0] * MECHANICS_WORLD_SCALE,
            ],
            suspension_damping: self.mechanics.damping,
            collision_world_scale: MECHANICS_WORLD_SCALE,
        };
        if !config.is_valid() {
            return Err(format!("{} has invalid converted mechanics", self.file));
        }
        Ok(config)
    }
}

pub struct TrackSources {
    pub spec: TrackVisualSpec,
    pub scene: VisualScene,
}

impl TrackSources {
    pub fn load(dir: &GameDir, race_name: &str) -> Result<Self, String> {
        let races_path = dir.data_path("RACES.TXT")?;
        let races = RaceCatalog::parse(&read(&races_path)?)
            .map_err(|e| format!("{}: {e}", races_path.display()))?;
        let (_, race) = races
            .find(race_name)
            .ok_or_else(|| format!("race {race_name} not found"))?;
        let track_path = dir.asset_path("RACES", &race.track_file)?;
        let spec = TrackVisualSpec::parse(&read(&track_path)?, &race.track_file)
            .map_err(|e| format!("{}: {e}", track_path.display()))?;
        let scene = VisualScene::load(
            dir,
            &spec.pixelmaps,
            &spec.materials,
            &spec.models,
            &spec.actor_file,
        )?;
        Ok(Self { spec, scene })
    }
}

pub fn build_collision_world(track: &TrackSources) -> Result<StaticCollisionWorld, String> {
    let mut models = HashMap::new();
    for model in track.scene.models.iter().flat_map(|file| &file.models) {
        models.entry(key(&model.identifier)).or_insert(model);
    }
    let mut materials = HashMap::new();
    for material in track
        .scene
        .materials
        .iter()
        .flat_map(|file| &file.materials)
    {
        materials
            .entry(key(&material.identifier))
            .or_insert(material);
    }

    fn add_actor(
        actor: &Actor,
        parent_transform: Mat4,
        parent_path: &str,
        models: &HashMap<String, &Model>,
        materials: &HashMap<String, &Material>,
        world: &mut StaticCollisionWorld,
    ) -> Result<(), String> {
        let actor_path = if parent_path.is_empty() {
            actor.identifier.clone()
        } else {
            format!("{parent_path}/{}", actor.identifier)
        };
        let transform = parent_transform * source_transform(&actor.transform).to_matrix();
        if let Some(model_name) = &actor.model {
            let model = models.get(&key(model_name)).ok_or_else(|| {
                format!("ACT actor {actor_path} references missing DAT model {model_name}")
            })?;
            let actor_material = actor
                .material
                .as_deref()
                .map(|name| {
                    materials.get(&key(name)).copied().ok_or_else(|| {
                        format!("ACT actor {actor_path} references missing MAT {name}")
                    })
                })
                .transpose()?;
            let model_materials = model
                .materials
                .iter()
                .map(|name| materials.get(&key(name)).copied())
                .collect::<Vec<_>>();
            let actor_id: Arc<str> = Arc::from(actor_path.as_str());
            let model_id: Arc<str> = Arc::from(model.identifier.as_str());

            for (face_index, face) in model.faces.iter().enumerate() {
                let material = if actor_material.is_some() {
                    actor_material
                } else if face.material_index == 0 {
                    None
                } else {
                    let material_index = usize::from(face.material_index) - 1;
                    Some(
                        model_materials
                            .get(material_index)
                            .and_then(|material| *material)
                            .ok_or_else(|| {
                                format!(
                                    "DAT model {} face {face_index} references missing material index {}",
                                    model.identifier, face.material_index
                                )
                            })?,
                    )
                };
                let mut vertices = [[0.0; 3]; 3];
                for (slot, index) in vertices.iter_mut().zip(face.vertices) {
                    let vertex = model.vertices.get(usize::from(index)).ok_or_else(|| {
                        format!(
                            "DAT model {} face {face_index} has invalid vertex index {index}",
                            model.identifier
                        )
                    })?;
                    *slot = transform
                        .transform_point3(Vec3::from_array(vertex.position))
                        .to_array();
                }
                let source = SurfaceIdentity {
                    actor_path: actor_id.clone(),
                    model: model_id.clone(),
                    face_index,
                    material: material.map(|material| Arc::from(material.identifier.as_str())),
                };
                let two_sided = material
                    .is_none_or(|material| material.two_sided() || material.always_visible());
                world.add_triangle(vertices, face.flags, two_sided, source);
            }
        }

        for child in &actor.children {
            add_actor(child, transform, &actor_path, models, materials, world)?;
        }
        Ok(())
    }

    let mut world = StaticCollisionWorld::default();
    for root in &track.scene.actor.roots {
        add_actor(root, Mat4::IDENTITY, "", &models, &materials, &mut world)?;
    }
    Ok(world)
}

pub struct GalleryEntry {
    pub name: String,
    pub scene: VisualScene,
    pub radius: f32,
}

pub struct GallerySources {
    pub entries: Vec<GalleryEntry>,
    pub roster: GalleryRoster,
}

impl GallerySources {
    pub fn load(dir: &GameDir, race_name: &str, seed: u64) -> Result<Self, String> {
        let races_path = dir.data_path("RACES.TXT")?;
        let opponents_path = dir.data_path("OPPONENT.TXT")?;
        let general_path = dir.data_path("GENERAL.TXT")?;
        let races = RaceCatalog::parse(&read(&races_path)?)
            .map_err(|e| format!("{}: {e}", races_path.display()))?;
        let opponents = OpponentCatalog::parse(&read(&opponents_path)?)
            .map_err(|e| format!("{}: {e}", opponents_path.display()))?;
        let (_, player_file) = initial_player(&read(&general_path)?)
            .map_err(|e| format!("{}: {e}", general_path.display()))?;
        let roster = GalleryRoster::resolve(&races, &opponents, race_name, &player_file, seed)
            .map_err(|e| e.to_string())?;
        let player_name = opponents
            .opponents
            .iter()
            .find(|op| op.car_file.eq_ignore_ascii_case(&player_file))
            .map_or_else(
                || player_file.trim_end_matches(".TXT").to_owned(),
                |op| op.name.clone(),
            );
        let mut entries = Vec::with_capacity(roster.opponents.len() + 1);
        for (name, file) in std::iter::once((player_name.as_str(), roster.player_car_file.as_str()))
            .chain(
                roster
                    .opponents
                    .iter()
                    .map(|op| (op.name.as_str(), op.car_file.as_str())),
            )
        {
            let scene = VisualScene::car_by_file(dir, file, VisualVariant::Low)?;
            let radius = scene.principal_radius()?;
            entries.push(GalleryEntry {
                name: name.to_owned(),
                scene,
                radius,
            });
        }
        Ok(Self { entries, roster })
    }
}

impl VisualScene {
    pub fn principal_radius(&self) -> Result<f32, String> {
        let model_name = self
            .actor
            .roots
            .first()
            .and_then(|actor| actor.model.as_ref())
            .ok_or("principal actor has no model")?;
        let model = self
            .models
            .iter()
            .flat_map(|file| &file.models)
            .find(|model| model.identifier.eq_ignore_ascii_case(model_name))
            .ok_or_else(|| format!("principal model {model_name} not loaded"))?;
        let radius = model
            .vertices
            .iter()
            .map(|vertex| source_point(vertex.position).length())
            .fold(0.0_f32, f32::max);
        if radius <= 0.0 || !radius.is_finite() {
            return Err(format!("principal model {model_name} has invalid radius"));
        }
        Ok(radius)
    }

    pub fn initial_car(dir: &GameDir) -> Result<Self, String> {
        let path = dir.data_path("GENERAL.TXT")?;
        let (_, file) =
            initial_player(&read(&path)?).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::car_by_file(dir, &file, VisualVariant::Low)
    }

    pub fn car_by_file(dir: &GameDir, file: &str, variant: VisualVariant) -> Result<Self, String> {
        let path = dir.asset_path("CARS", file)?;
        let spec =
            CarVisualSpec::parse(&read(&path)?).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::car(dir, &spec, variant)
    }

    pub fn car(
        dir: &GameDir,
        spec: &CarVisualSpec,
        variant: VisualVariant,
    ) -> Result<Self, String> {
        let actor = &spec.actors[spec.principal_actor].file;
        Self::load(
            dir,
            spec.pixelmaps_for(variant),
            spec.materials_for(variant),
            &spec.models,
            actor,
        )
    }

    pub fn load(
        dir: &GameDir,
        pixelmap_files: &[String],
        material_files: &[String],
        model_files: &[String],
        actor_file: &str,
    ) -> Result<Self, String> {
        let palette_path = dir.asset_path("REG/PALETTES", "DRRENDER.PAL")?;
        let palette_file = PixFile::parse(&read(&palette_path)?)
            .map_err(|e| format!("{}: {e}", palette_path.display()))?;
        let palette = palette_from_pix(palette_file.pixelmaps.first().ok_or("empty palette")?)?;
        let mut pixelmaps = Vec::with_capacity(pixelmap_files.len());
        for name in pixelmap_files {
            let path = dir
                .asset_path("REG/PIXELMAP", name)
                .or_else(|_| dir.asset_path("PIXELMAP", name))?;
            pixelmaps.push(
                PixFile::parse(&read(&path)?).map_err(|e| format!("{}: {e}", path.display()))?,
            );
        }
        let mut materials = Vec::with_capacity(material_files.len());
        for name in material_files {
            let path = dir.asset_path("MATERIAL", name)?;
            materials.push(
                MatFile::parse(&read(&path)?).map_err(|e| format!("{}: {e}", path.display()))?,
            );
        }
        let mut models = Vec::with_capacity(model_files.len());
        for name in model_files {
            let path = dir.asset_path("MODELS", name)?;
            models.push(
                DatFile::parse(&read(&path)?).map_err(|e| format!("{}: {e}", path.display()))?,
            );
        }
        let path = dir.asset_path("ACTORS", actor_file)?;
        let actor =
            ActFile::parse(&read(&path)?).map_err(|e| format!("{}: {e}", path.display()))?;
        let needed_models = actor_model_names(&actor);
        // Model material names are registry identifiers. Resolve any names
        // absent from the track's initial MAT group by their original files.
        let mut loaded_materials: HashSet<String> = materials
            .iter()
            .flat_map(|file| file.materials.iter().map(|mat| key(&mat.identifier)))
            .collect();
        for model in models.iter().flat_map(|file| &file.models) {
            if !needed_models.contains(&key(&model.identifier)) {
                continue;
            }
            for name in &model.materials {
                if loaded_materials.contains(&key(name)) {
                    continue;
                }
                let path = dir.asset_path("MATERIAL", name).map_err(|e| {
                    format!("model {} needs material {name}: {e}", model.identifier)
                })?;
                let file = MatFile::parse(&read(&path)?)
                    .map_err(|e| format!("{}: {e}", path.display()))?;
                for material in &file.materials {
                    loaded_materials.insert(key(&material.identifier));
                }
                if !loaded_materials.contains(&key(name)) {
                    return Err(format!(
                        "{} lacks material {name} for model {}",
                        path.display(),
                        model.identifier
                    ));
                }
                materials.push(file);
            }
        }
        // Some material maps (notably SKIDMARK.PIX) are loaded later by
        // world.c rather than listed in the track's initial PIX group.
        let mut loaded_maps: HashSet<String> = pixelmaps
            .iter()
            .flat_map(|file| file.pixelmaps.iter().map(|map| key(&map.identifier)))
            .collect();
        for material in materials.iter().flat_map(|file| &file.materials) {
            if let Some(map) = &material.colour_map
                && !loaded_maps.contains(&key(map))
            {
                let path = dir
                    .asset_path("REG/PIXELMAP", map)
                    .or_else(|_| dir.asset_path("PIXELMAP", map))
                    .map_err(|e| format!("material {} needs {map}: {e}", material.identifier))?;
                let file = PixFile::parse(&read(&path)?)
                    .map_err(|e| format!("{}: {e}", path.display()))?;
                for image in &file.pixelmaps {
                    loaded_maps.insert(key(&image.identifier));
                }
                if !loaded_maps.contains(&key(map)) {
                    return Err(format!(
                        "{} lacks pixelmap {map} for material {}",
                        path.display(),
                        material.identifier
                    ));
                }
                pixelmaps.push(file);
            }
        }
        actor
            .link(
                &models.iter().collect::<Vec<_>>(),
                &materials.iter().collect::<Vec<_>>(),
            )
            .map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Self {
            actor,
            pixelmaps,
            materials,
            models,
            palette,
        })
    }

    pub fn prepare(
        self,
        images: &mut Assets<Image>,
        materials: &mut Assets<StandardMaterial>,
        meshes: &mut Assets<Mesh>,
    ) -> Result<PreparedVisualScene, String> {
        let mut image_handles = HashMap::new();
        for pix in &self.pixelmaps {
            for map in &pix.pixelmaps {
                let name = key(&map.identifier);
                if let std::collections::hash_map::Entry::Vacant(entry) = image_handles.entry(name)
                {
                    entry.insert(images.add(pixelmap_to_image(map, &self.palette)?));
                }
            }
        }
        let mut material_handles = HashMap::new();
        let mut source_materials = HashMap::new();
        for mat_file in &self.materials {
            for mat in &mat_file.materials {
                let name = key(&mat.identifier);
                if material_handles.contains_key(&name) {
                    continue;
                }
                let texture = mat
                    .colour_map
                    .as_ref()
                    .map(|map| {
                        image_handles.get(&key(map)).cloned().ok_or_else(|| {
                            format!("material {} missing pixelmap {map}", mat.identifier)
                        })
                    })
                    .transpose()?;
                let color =
                    Color::srgba_u8(mat.colour[0], mat.colour[1], mat.colour[2], mat.opacity);
                let handle = materials.add(StandardMaterial {
                    base_color: color,
                    base_color_texture: texture,
                    unlit: true,
                    alpha_mode: if mat.opacity < 255 {
                        AlphaMode::Blend
                    } else {
                        AlphaMode::Opaque
                    },
                    cull_mode: if mat.two_sided() || mat.always_visible() {
                        None
                    } else {
                        Some(CullFace::Back)
                    },
                    double_sided: mat.two_sided(),
                    ..default()
                });
                material_handles.insert(name.clone(), handle);
                source_materials.insert(name, mat);
            }
        }
        let default_material = materials.add(StandardMaterial {
            unlit: true,
            cull_mode: Some(CullFace::Back),
            ..default()
        });
        let needed = actor_model_names(&self.actor);
        let mut model_groups = HashMap::new();
        for dat in &self.models {
            for model in &dat.models {
                let name = key(&model.identifier);
                if !needed.contains(&name) || model_groups.contains_key(&name) {
                    continue;
                }
                let groups = model_meshes(model, &source_materials, meshes)?;
                model_groups.insert(name, groups);
            }
        }
        Ok(PreparedVisualScene {
            roots: self.actor.roots,
            model_groups,
            material_handles,
            default_material,
        })
    }
}

struct MeshGroup {
    mesh: Handle<Mesh>,
    material: Option<String>,
}

type MeshBuffers = (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>);

fn model_meshes(
    model: &Model,
    materials: &HashMap<String, &Material>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<MeshGroup>, String> {
    let mut groups: HashMap<Option<String>, MeshBuffers> = HashMap::new();
    for face in &model.faces {
        let material = if face.material_index == 0 {
            None
        } else {
            Some(key(model
                .materials
                .get(usize::from(face.material_index) - 1)
                .ok_or_else(|| {
                    format!(
                        "model {} face material index out of range",
                        model.identifier
                    )
                })?))
        };
        let source = material
            .as_ref()
            .map(|name| {
                materials
                    .get(name)
                    .copied()
                    .ok_or_else(|| format!("model {} missing material {name}", model.identifier))
            })
            .transpose()?;
        let group = groups.entry(material).or_default();
        let p = face
            .vertices
            .map(|index| source_point(model.vertices[usize::from(index)].position));
        let normal = (p[1] - p[0])
            .cross(p[2] - p[0])
            .normalize_or_zero()
            .to_array();
        for &index in &face.vertices {
            let vertex = &model.vertices[usize::from(index)];
            group.0.push(source_point(vertex.position).to_array());
            group.1.push(normal);
            let uv = vertex.uv;
            let uv = source.map_or(uv, |mat| {
                [
                    uv[0] * mat.map_transform[0][0]
                        + uv[1] * mat.map_transform[1][0]
                        + mat.map_transform[2][0],
                    uv[0] * mat.map_transform[0][1]
                        + uv[1] * mat.map_transform[1][1]
                        + mat.map_transform[2][1],
                ]
            });
            group.2.push(uv);
        }
    }
    let mut out = Vec::with_capacity(groups.len());
    for (material, (positions, normals, uvs)) in groups {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        out.push(MeshGroup {
            mesh: meshes.add(mesh),
            material,
        });
    }
    Ok(out)
}

pub struct PreparedVisualScene {
    roots: Vec<Actor>,
    model_groups: HashMap<String, Vec<MeshGroup>>,
    material_handles: HashMap<String, Handle<StandardMaterial>>,
    default_material: Handle<StandardMaterial>,
}

impl PreparedVisualScene {
    pub fn spawn(&self, commands: &mut Commands) -> Result<Vec<Entity>, String> {
        self.roots
            .iter()
            .map(|actor| self.spawn_actor(commands, actor, None))
            .collect()
    }

    fn spawn_actor(
        &self,
        commands: &mut Commands,
        actor: &Actor,
        parent: Option<Entity>,
    ) -> Result<Entity, String> {
        let visibility = if actor.render_style == 1 {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if !matches!(actor.render_style, 0 | 1 | 4) {
            return Err(format!(
                "actor {} unsupported render style {}",
                actor.identifier, actor.render_style
            ));
        }
        let entity = commands
            .spawn((
                Name::new(actor.identifier.clone()),
                source_transform(&actor.transform),
                visibility,
            ))
            .id();
        if let Some(parent) = parent {
            commands.entity(parent).add_child(entity);
        }
        if actor.render_style != 1
            && let Some(model) = &actor.model
        {
            let groups = self
                .model_groups
                .get(&key(model))
                .ok_or_else(|| format!("actor {} missing model {model}", actor.identifier))?;
            for group in groups {
                let name = actor
                    .material
                    .as_ref()
                    .map_or(group.material.as_ref(), Some);
                let material = name
                    .map(|name| {
                        self.material_handles
                            .get(&key(name))
                            .cloned()
                            .ok_or_else(|| {
                                format!("actor {} missing material {name}", actor.identifier)
                            })
                    })
                    .transpose()?
                    .unwrap_or_else(|| self.default_material.clone());
                let mesh = commands
                    .spawn((
                        Mesh3d(group.mesh.clone()),
                        MeshMaterial3d(material),
                        Transform::IDENTITY,
                    ))
                    .id();
                commands.entity(entity).add_child(mesh);
            }
        }
        for child in &actor.children {
            self.spawn_actor(commands, child, Some(entity))?;
        }
        Ok(entity)
    }
}

#[cfg(test)]
mod tests {
    use super::{palette_from_pix, pixelmap_to_image, source_transform};
    use bevy::prelude::*;
    use dethrace_formats::{
        act::ActorTransform,
        pix::{PixelType, Pixelmap},
    };
    #[test]
    fn axes_matrix_and_palette_channels_match_source() {
        let matrix =
            ActorTransform::Matrix34([[0., 1., 0.], [-1., 0., 0.], [0., 0., 1.], [2., 3., 4.]]);
        let transformed = source_transform(&matrix)
            .to_matrix()
            .transform_point3(Vec3::X);
        assert!((transformed - Vec3::new(2., 4., 4.)).length() < 1e-5);
        let mut pixels = vec![0; 1024];
        pixels[4..8].copy_from_slice(&[0, 93, 3, 2]);
        let pal = Pixelmap {
            identifier: "PAL".into(),
            pixel_type: PixelType::Rgbx888,
            width: 1,
            height: 256,
            row_bytes: 4,
            origin_x: 0,
            origin_y: 0,
            mip_offset: 0,
            pixels,
        };
        let colors = palette_from_pix(&pal).unwrap();
        assert_eq!(colors[1], [93, 3, 2]);
        let map = Pixelmap {
            identifier: "MAP".into(),
            pixel_type: PixelType::Index8,
            width: 1,
            height: 1,
            row_bytes: 1,
            origin_x: 0,
            origin_y: 0,
            mip_offset: 0,
            pixels: vec![1],
        };
        assert_eq!(
            pixelmap_to_image(&map, &colors).unwrap().data.unwrap(),
            [93, 3, 2, 255]
        );
    }
}

#[cfg(test)]
mod collision_tests {
    use super::build_collision_world;
    use dethrace_formats::{
        act::{Actor, ActorTransform},
        dat::{DatFile, Face, Model, Vertex},
        mat::{MatFile, Material},
        track_visual::TrackVisualSpec,
    };

    #[test]
    fn collision_world_composes_actor_transforms_and_keeps_face_identity() {
        let track = super::TrackSources {
            spec: TrackVisualSpec {
                track_file: "TRACK.TXT".into(),
                start_position: [0.0; 3],
                start_yaw_degrees: 0.0,
                pixelmaps: vec![],
                materials: vec![],
                models: vec![],
                actor_file: "TRACK.ACT".into(),
            },
            scene: super::VisualScene {
                actor: dethrace_formats::act::ActFile {
                    roots: vec![Actor {
                        identifier: "ROOT".into(),
                        actor_type: 0,
                        render_style: 0,
                        transform: ActorTransform::Translation([10.0, 0.0, 0.0]),
                        model: None,
                        material: None,
                        bounds: None,
                        children: vec![Actor {
                            identifier: "ROAD".into(),
                            actor_type: 1,
                            render_style: 4,
                            transform: ActorTransform::Translation([0.0, 2.0, 0.0]),
                            model: Some("PLANE".into()),
                            material: None,
                            bounds: None,
                            children: vec![],
                        }],
                    }],
                },
                pixelmaps: vec![],
                materials: vec![MatFile {
                    materials: vec![Material {
                        identifier: "ROAD_MAT".into(),
                        colour: [0; 3],
                        opacity: 255,
                        ambient: 0.0,
                        diffuse: 0.0,
                        specular: 0.0,
                        power: 0.0,
                        flags: 0x0800,
                        map_transform: [[0.0; 2]; 3],
                        index_base: 0,
                        index_range: 0,
                        colour_map: None,
                        index_blend: None,
                        index_shade: None,
                        screendoor: None,
                        index_fog: None,
                    }],
                }],
                models: vec![DatFile {
                    models: vec![Model {
                        identifier: "PLANE".into(),
                        flags: 0,
                        pivot: None,
                        crease_angle: None,
                        radius: None,
                        bounds: None,
                        vertices: vec![
                            Vertex {
                                position: [0.0, 0.0, 0.0],
                                uv: [0.0; 2],
                            },
                            Vertex {
                                position: [1.0, 0.0, 0.0],
                                uv: [0.0; 2],
                            },
                            Vertex {
                                position: [0.0, 1.0, 0.0],
                                uv: [0.0; 2],
                            },
                        ],
                        faces: vec![Face {
                            vertices: [0, 1, 2],
                            smoothing: 0,
                            flags: 0x80,
                            material_index: 1,
                        }],
                        materials: vec!["ROAD_MAT".into()],
                    }],
                }],
                palette: [[0; 3]; 256],
            },
        };
        let world = build_collision_world(&track).unwrap();
        let (hit, source) = world
            .raycast_ground([10.2, 2.2, 1.0], [0.0, 0.0, -1.0], 2.0)
            .unwrap();
        assert_eq!(hit.point, [10.2, 2.2, 0.0]);
        assert_eq!(hit.normal, [0.0, 0.0, 1.0]);
        assert_eq!(source.actor_path.as_ref(), "ROOT/ROAD");
        assert_eq!(source.model.as_ref(), "PLANE");
        assert_eq!(source.face_index, 0);
        assert_eq!(source.material.as_deref(), Some("ROAD_MAT"));
        assert_eq!(world.triangles()[0].face_flags, 0x80);
        assert_eq!(
            world.bounds().unwrap(),
            [[10.0, 2.0, 0.0], [11.0, 3.0, 0.0]]
        );
    }
}
