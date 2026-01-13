use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Realistic Black Hole Simulation".to_string(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MaterialPlugin::<BlackHoleMaterial>::default())
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, setup)
        .add_systems(Update, (update_material, move_camera))
        .run();
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct BlackHoleMaterial {
    #[uniform(0)]
    camera_pos: Vec3,
    #[uniform(1)]
    camera_forward: Vec3,
    #[uniform(2)]
    camera_right: Vec3,
    #[uniform(3)]
    camera_up: Vec3,
    #[uniform(4)]
    time: f32,
}

impl Material for BlackHoleMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/black_hole.wgsl".into()
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
        descriptor.primitive.cull_mode = None; // Render both sides so we can be inside the cube
        Ok(())
    }
}

// Marker for our camera
#[derive(Component)]
struct MainCamera;

// Marker for our material entity to update it
#[derive(Component)]
struct BlackHoleEntity;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<BlackHoleMaterial>>,
) {
    // 1. Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
        MainCamera,
    ));

    // 2. The "Screen" Volume
    // We stick the camera inside a Cube. The shader will calculate rays.
    // Note: If the camera leaves the cube, the effect vanishes.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(20.0, 20.0, 20.0))),
        MeshMaterial3d(materials.add(BlackHoleMaterial {
            camera_pos: Vec3::ZERO,
            camera_forward: Vec3::NEG_Z,
            camera_right: Vec3::X,
            camera_up: Vec3::Y,
            time: 0.0,
        })),
        Transform::from_translation(Vec3::ZERO), // Centered
        BlackHoleEntity,
    ));

    println!("--------------------------------------------------");
    println!("CONTROLS:");
    println!("  WASD: Move Forward/Back/Left/Right");
    println!("  Q/E:  Move Down/Up");
    println!("  Arrows: Rotate Camera");
    println!("--------------------------------------------------");
}

fn update_material(
    time: Res<Time>,
    mut materials: ResMut<Assets<BlackHoleMaterial>>,
    q_mat: Query<&MeshMaterial3d<BlackHoleMaterial>, With<BlackHoleEntity>>,
    q_cam: Query<&Transform, With<MainCamera>>,
    mut q_cube: Query<&mut Transform, (With<BlackHoleEntity>, Without<MainCamera>)>,
) {
    let Ok(cam_transform) = q_cam.get_single() else {
        return;
    };
    let Ok(mat_handle) = q_mat.get_single() else {
        return;
    };
    let Some(material) = materials.get_mut(mat_handle) else {
        return;
    };

    // Update Uniforms
    material.camera_pos = cam_transform.translation;
    material.camera_forward = cam_transform.forward().as_vec3();
    material.camera_right = cam_transform.right().as_vec3();
    material.camera_up = cam_transform.up().as_vec3();
    material.time = time.elapsed_secs();

    // Keep cube centered on camera
    if let Ok(mut cube_tf) = q_cube.get_single_mut() {
        cube_tf.translation = cam_transform.translation;
    }
}

fn move_camera(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<MainCamera>>,
) {
    let Ok(mut transform) = query.get_single_mut() else {
        return;
    };
    let speed = 10.0;
    let rotate_speed = 1.5;
    let dt = time.delta_secs();

    // Translation
    if keyboard_input.pressed(KeyCode::KeyW) {
        let fwd = transform.forward().as_vec3();
        transform.translation += fwd * speed * dt;
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        let fwd = transform.forward().as_vec3();
        transform.translation -= fwd * speed * dt;
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        let right = transform.right().as_vec3();
        transform.translation -= right * speed * dt;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        let right = transform.right().as_vec3();
        transform.translation += right * speed * dt;
    }
    if keyboard_input.pressed(KeyCode::KeyQ) {
        transform.translation.y -= speed * dt;
    }
    if keyboard_input.pressed(KeyCode::KeyE) {
        transform.translation.y += speed * dt;
    }

    // Rotation
    if keyboard_input.pressed(KeyCode::ArrowLeft) {
        transform.rotate_y(rotate_speed * dt);
    }
    if keyboard_input.pressed(KeyCode::ArrowRight) {
        transform.rotate_y(-rotate_speed * dt);
    }
    if keyboard_input.pressed(KeyCode::ArrowUp) {
        transform.rotate_local_x(rotate_speed * dt);
    }
    if keyboard_input.pressed(KeyCode::ArrowDown) {
        transform.rotate_local_x(-rotate_speed * dt);
    }
}
