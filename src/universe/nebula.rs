use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
// use bevy::render::view::RenderLayers; // Unused

pub struct NebulaPlugin;

impl Plugin for NebulaPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<NebulaMaterial>::default())
            .add_systems(Startup, spawn_nebula_volume)
            .add_systems(Update, update_nebula_position);
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct NebulaMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
    #[uniform(0)]
    pub density_scale: f32,
    #[uniform(0)]
    pub noise_scale: f32,
    #[uniform(0)]
    pub absorption: f32,
}

impl Material for NebulaMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/nebula.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

#[derive(Component)]
struct NebulaVolume;

fn spawn_nebula_volume(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<NebulaMaterial>>,
) {
    // Huge box encapsulating the camera view
    // Increase size significantly to ensure we don't clip
    let size = 100_000.0;
    let mesh = meshes.add(Cuboid::from_size(Vec3::splat(size)));

    let material = materials.add(NebulaMaterial {
        color: LinearRgba::from(Color::srgb(0.5, 0.0, 0.8)),
        density_scale: 1.0,
        noise_scale: 0.00005, // Very small scale for huge universe feel
        absorption: 0.002,
    });

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, 0.0, 0.0),
        NebulaVolume,
        // Make sure it doesn't cast shadows or interact weirdly
        // Not Pickable?
    ));
}

fn update_nebula_position(
    mut q_nebula: Query<&mut Transform, With<NebulaVolume>>,
    q_camera: Query<&GlobalTransform, With<Camera3d>>,
) {
    let Ok(cam_t) = q_camera.get_single() else {
        return;
    };
    let cam_pos = cam_t.translation();

    for mut t in q_nebula.iter_mut() {
        // Keep the nebula volume centered on camera so we never fly out of it
        t.translation = cam_pos;
    }
}
