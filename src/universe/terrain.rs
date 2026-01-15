use crate::universe::PlanetDetails;
use crate::universe::materials::PlanetMaterial;
// use crate::universe::planet_baker::DirtyPlanetTexture; // Unused
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use big_space::FloatingOrigin;
use futures_lite::future;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (update_planet_lod, spawn_terrain_patches));
    }
}

#[derive(Component)]
pub struct PlanetTerrain {
    pub details: PlanetDetails,
    pub radius: f32,
    pub max_depth: u8,
    pub split_factor: f32,
    pub roots: [QuadTreeNode; 6],
}

impl PlanetTerrain {
    pub fn new(details: PlanetDetails, radius: f32, max_depth: u8, split_factor: f32) -> Self {
        Self {
            details,
            radius,
            max_depth,
            split_factor,
            roots: [
                QuadTreeNode::new(CubeFace::Right, 0, Vec2::ZERO, 1.0),
                QuadTreeNode::new(CubeFace::Left, 0, Vec2::ZERO, 1.0),
                QuadTreeNode::new(CubeFace::Top, 0, Vec2::ZERO, 1.0),
                QuadTreeNode::new(CubeFace::Bottom, 0, Vec2::ZERO, 1.0),
                QuadTreeNode::new(CubeFace::Front, 0, Vec2::ZERO, 1.0),
                QuadTreeNode::new(CubeFace::Back, 0, Vec2::ZERO, 1.0),
            ],
        }
    }
}

pub struct QuadTreeNode {
    pub children: Option<Box<[QuadTreeNode; 4]>>,
    pub entity: Option<Entity>, // Handle to the mesh entity (Leaf only)
    pub face: CubeFace,
    pub depth: u8,
    pub start_coord: Vec2, // 0.0 - 1.0 Top-Left
    pub size: f32,         // 0.0 - 1.0 Scale
}

impl QuadTreeNode {
    pub fn new(face: CubeFace, depth: u8, start_coord: Vec2, size: f32) -> Self {
        Self {
            children: None,
            entity: None,
            face,
            depth,
            start_coord,
            size,
        }
    }

    pub fn split(&mut self) {
        if self.children.is_some() {
            return;
        }
        let half_size = self.size * 0.5;
        self.children = Some(Box::new([
            // Top-Left
            QuadTreeNode::new(self.face, self.depth + 1, self.start_coord, half_size),
            // Top-Right
            QuadTreeNode::new(
                self.face,
                self.depth + 1,
                self.start_coord + Vec2::new(half_size, 0.0),
                half_size,
            ),
            // Bottom-Left
            QuadTreeNode::new(
                self.face,
                self.depth + 1,
                self.start_coord + Vec2::new(0.0, half_size),
                half_size,
            ),
            // Bottom-Right
            QuadTreeNode::new(
                self.face,
                self.depth + 1,
                self.start_coord + Vec2::new(half_size, half_size),
                half_size,
            ),
        ]));
    }

    pub fn merge(&mut self, commands: &mut Commands) {
        if let Some(children) = self.children.take() {
            for mut child in children.into_iter() {
                child.recursive_despawn(commands);
            }
        }
    }

    pub fn recursive_despawn(&mut self, commands: &mut Commands) {
        if let Some(entity) = self.entity.take() {
            commands.entity(entity).despawn_recursive();
        }
        if let Some(children) = self.children.take() {
            for mut child in children.into_iter() {
                child.recursive_despawn(commands);
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum CubeFace {
    Right,
    Left,
    Top,
    Bottom,
    Front,
    Back,
}

#[derive(Component)]
pub struct TerrainPatchTask(Task<Mesh>);

fn update_planet_lod(
    mut commands: Commands,
    mut q_terrain: Query<(Entity, &GlobalTransform, &mut PlanetTerrain)>,
    q_camera: Query<&GlobalTransform, With<FloatingOrigin>>,
) {
    let Ok(cam_transform) = q_camera.get_single() else {
        return;
    };
    let cam_pos = cam_transform.translation();

    for (p_entity, p_transform, mut terrain) in q_terrain.iter_mut() {
        let p_pos = p_transform.translation();
        // Simple distance check to center for now.
        // Improvement: Distance to closest point on AABB.
        let _dist = p_pos.distance(cam_pos);
        let local_cam_pos = p_transform.affine().inverse().transform_point3(cam_pos);

        let radius = terrain.radius;
        let max_depth = terrain.max_depth;
        let split_factor = terrain.split_factor;

        // Process each face root
        for root in terrain.roots.iter_mut() {
            process_lod_node(
                &mut commands,
                p_entity,
                root,
                local_cam_pos,
                radius,
                max_depth,
                split_factor,
            );
        }
    }
}

fn process_lod_node(
    commands: &mut Commands,
    parent: Entity,
    node: &mut QuadTreeNode,
    local_cam_pos: Vec3,
    planet_radius: f32,
    max_depth: u8,
    split_factor: f32,
) {
    // 1. Calculate Center of this node on sphere
    let center_uv = node.start_coord + Vec2::splat(node.size * 0.5);
    let center_pos_cube = get_cube_point(node.face, center_uv);
    let center_pos_sphere = center_pos_cube.normalize() * planet_radius;

    // 2. Distance Check
    let dist = local_cam_pos.distance(center_pos_sphere);

    // Node Size in world units (arc length approx)
    let node_arc_len = (node.size * std::f32::consts::PI * 0.5) * planet_radius;

    // Split Condition
    let should_split = dist < node_arc_len * split_factor && node.depth < max_depth;

    if should_split {
        // Ensure Split
        if node.children.is_none() {
            node.split();
            // If we just split, we must despawn OUR mesh if we had one
            if let Some(entity) = node.entity.take() {
                commands.entity(entity).despawn_recursive();
            }
        }

        // Recurse
        if let Some(children) = node.children.as_mut() {
            for child in children.iter_mut() {
                process_lod_node(
                    commands,
                    parent,
                    child,
                    local_cam_pos,
                    planet_radius,
                    max_depth,
                    split_factor,
                );
            }
        }
    } else {
        // Ensure Merge (Leaf)
        if node.children.is_some() {
            node.merge(commands);
        }

        // Ensure Mesh Exists
        if node.entity.is_none() {
            // Spawn Mesh Task
            spawn_patch_request(commands, parent, node, planet_radius);
        }
    }
}

fn spawn_patch_request(
    commands: &mut Commands,
    parent: Entity,
    node: &mut QuadTreeNode,
    radius: f32,
) {
    let thread_pool = AsyncComputeTaskPool::get();

    let face = node.face;
    let start = node.start_coord;
    let size = node.size;
    let radius = radius;

    // Clone necessary data

    let task = thread_pool.spawn(async move { generate_patch_mesh(face, start, size, radius) });

    let entity = commands.spawn(TerrainPatchTask(task)).id();
    commands.entity(parent).add_child(entity);
    node.entity = Some(entity);
}

fn generate_patch_mesh(face: CubeFace, start_uv: Vec2, size_uv: f32, radius: f32) -> Mesh {
    let resolution = 16;
    let step = size_uv / resolution as f32;

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new(); // Global UVs for texture
    let mut indices: Vec<u32> = Vec::new();

    for y in 0..=resolution {
        for x in 0..=resolution {
            let u_local = x as f32 * step;
            let v_local = y as f32 * step;

            let uv_face = start_uv + Vec2::new(u_local, v_local);

            // Cube to Sphere
            let cube_point = get_cube_point(face, uv_face);
            let sphere_dir = cube_point.normalize();
            let pos = sphere_dir * radius;

            // TODO: Height Noise here!

            positions.push(pos.to_array());
            normals.push(sphere_dir.to_array());

            // UV Mapping (Equirectangular approximation or Cubemap UVs?)
            // For now, let's map 0-1 based on face, or use triplanar logic in shader.
            // Let's just pass the Face UV for debugging logic, or remapped global UV.
            uvs.push(uv_face.to_array());
        }
    }

    for y in 0..resolution {
        for x in 0..resolution {
            let i = x + y * (resolution + 1);

            indices.push(i);
            indices.push(i + resolution + 1);
            indices.push(i + 1);

            indices.push(i + 1);
            indices.push(i + resolution + 1);
            indices.push(i + resolution + 2);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn get_cube_point(face: CubeFace, uv: Vec2) -> Vec3 {
    let u = (uv.x * 2.0) - 1.0;
    let v = (uv.y * 2.0) - 1.0; // Invert Y? -1 is top? Depend on UV. Let's assume standard.

    match face {
        CubeFace::Front => Vec3::new(u, -v, 1.0),
        CubeFace::Back => Vec3::new(-u, -v, -1.0),
        CubeFace::Right => Vec3::new(1.0, -v, -u),
        CubeFace::Left => Vec3::new(-1.0, -v, u),
        CubeFace::Top => Vec3::new(u, 1.0, v),
        CubeFace::Bottom => Vec3::new(u, -1.0, -v),
    }
}

fn spawn_terrain_patches(
    mut commands: Commands,
    mut q_tasks: Query<(Entity, &mut TerrainPatchTask)>,
    mut meshes: ResMut<Assets<Mesh>>,
    q_parents: Query<&Parent>,
    q_materials_planet: Query<&MeshMaterial3d<PlanetMaterial>>,
    q_materials_std: Query<&MeshMaterial3d<StandardMaterial>>,
) {
    for (entity, mut task) in q_tasks.iter_mut() {
        if let Some(mesh) = future::block_on(future::poll_once(&mut task.0)) {
            // Task Done
            let mesh_handle = meshes.add(mesh);

            let mut found_material = false;

            if let Ok(parent) = q_parents.get(entity) {
                // Try PlanetMaterial first
                if let Ok(mat) = q_materials_planet.get(parent.get()) {
                    commands.entity(entity).insert(mat.clone());
                    found_material = true;
                }
                // Try StandardMaterial (GPU Baker)
                else if let Ok(mat) = q_materials_std.get(parent.get()) {
                    commands.entity(entity).insert(mat.clone());
                    found_material = true;
                }
            }

            if found_material {
                commands
                    .entity(entity)
                    .insert(Mesh3d(mesh_handle))
                    .remove::<TerrainPatchTask>();
            } else {
                // Warn? Or just wait?
                // If parent doesn't have material yet (Baker is slow), we should probably WAIT instead of despawning.
                // So we just don't remove the task? No, task is consumed.
                // We should insert the mesh and wait for material?
                // Let's insert the mesh. It will be invisible without material.
                commands
                    .entity(entity)
                    .insert(Mesh3d(mesh_handle))
                    .remove::<TerrainPatchTask>();
            }
        }
    }
}
