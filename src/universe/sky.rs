use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use big_space::{FloatingOrigin, GridCell};
use crate::universe::physics::GRID_SIZE;

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<StarfieldMaterial>::default())
           .add_systems(Update, (spawn_sky_sphere, update_sky_position));
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct StarfieldMaterial {
    #[uniform(0)]
    pub galactic_pos: Vec3,
    #[uniform(0)]
    pub time: f32,
}

impl Material for StarfieldMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/starfield.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }
    
    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline<Self>,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::render::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

#[derive(Component)]
struct SkySphere;

fn spawn_sky_sphere(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StarfieldMaterial>>,
    q_ship: Query<Entity, (With<FloatingOrigin>, Without<SkySphere>)>,
) {
    if let Ok(ship_entity) = q_ship.get_single() {
         commands.entity(ship_entity).with_children(|parent| {
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(50_000.0).mesh().ico(3).unwrap())), // Huge radius
                MeshMaterial3d(materials.add(StarfieldMaterial {
                    galactic_pos: Vec3::ZERO,
                    time: 0.0,
                })),
                SkySphere,
            ));
        });
    }
}

fn update_sky_position(
    q_camera: Query<(&GridCell<i64>, &Transform), With<FloatingOrigin>>,
    mut materials: ResMut<Assets<StarfieldMaterial>>,
    q_sky: Query<&MeshMaterial3d<StarfieldMaterial>, With<SkySphere>>,
    time: Res<Time>,
) {
    let Ok((cell, transform)) = q_camera.get_single() else { return; };
    let Ok(sky_mat_handle) = q_sky.get_single() else { return; };
    let Some(material) = materials.get_mut(sky_mat_handle) else { return; };

    // Convert GridCell + Local Pos to a "Rough Global Pos" for noise
    // We can just use the Cell coordinates as the primary driver since float precision 
    // at this scale is fine for noise offsets.
    let global_x = cell.x as f32 * GRID_SIZE + transform.translation.x;
    let global_y = cell.y as f32 * GRID_SIZE + transform.translation.y;
    let global_z = cell.z as f32 * GRID_SIZE + transform.translation.z;
    
    material.galactic_pos = Vec3::new(global_x, global_y, global_z);
    material.time = time.elapsed_secs();
}
