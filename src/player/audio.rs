use bevy::prelude::*;
use crate::player::camera::{ZenCamera, Velocity};
use crate::universe::{Mass, Radius};
use crate::universe::physics::GRID_SIZE;
use big_space::GridCell;
use rand::seq::SliceRandom;

pub struct ZenAudioPlugin;

impl Plugin for ZenAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioAssets>()
           .init_resource::<ChimeTimer>()
           .add_systems(Startup, setup_audio_assets)
           .add_systems(Update, (setup_engine_audio, update_engine_hum, proximity_chimes));
    }
}

#[derive(Resource, Default)]
struct AudioAssets {
    engine_hum: Handle<AudioSource>,
    chime: Handle<AudioSource>,
    loaded: bool,
}

#[derive(Component)]
struct EngineHumAudio;

#[derive(Resource)]
struct ChimeTimer(Timer);

impl Default for ChimeTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(0.5, TimerMode::Repeating)) // Check every 0.5s for efficiency
    }
}

fn setup_audio_assets(mut assets: ResMut<AudioAssets>, asset_server: Res<AssetServer>) {
    assets.engine_hum = asset_server.load("audio/engine_hum.ogg");
    assets.chime = asset_server.load("audio/chime.ogg");
    assets.loaded = true;
}

// Ensure Engine Audio is spawned on the camera once
fn setup_engine_audio(
    mut commands: Commands,
    assets: Res<AudioAssets>,
    q_camera: Query<Entity, (With<ZenCamera>, Without<EngineHumAudio>)>,
) {
    if !assets.loaded { return; }
    
    if let Ok(entity) = q_camera.get_single() {
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                AudioPlayer::new(assets.engine_hum.clone()),
                PlaybackSettings {
                    mode: bevy::audio::PlaybackMode::Loop,
                    volume: bevy::audio::Volume::new(0.0), // Start silent
                    ..default()
                },
                EngineHumAudio,
            ));
        });
    }
}

fn update_engine_hum(
    q_ship: Query<(&Velocity, &ZenCamera)>,
    mut q_audio: Query<(&mut AudioSink, &mut AudioPlayer), With<EngineHumAudio>>,
) {
    if let Ok((velocity, camera_settings)) = q_ship.get_single() {
        if let Ok((sink, _player)) = q_audio.get_single_mut() {
            let speed = velocity.0.length();
            let max_speed = camera_settings.max_speed;
            
            // Map speed (0 -> 10000) to Volume (0.05 -> 0.4)
            // Map speed to Speed/Pitch (0.8 -> 1.2)
            let ratio = (speed / max_speed).clamp(0.0, 1.0);
            
            let target_volume = 0.05 + ratio * 0.35;
            let target_pitch = 0.8 + ratio * 0.4;
            
            sink.set_volume(target_volume);
            sink.set_speed(target_pitch);
        }
    }
}

fn proximity_chimes(
    mut commands: Commands,
    mut timer: ResMut<ChimeTimer>,
    time: Res<Time>,
    assets: Res<AudioAssets>,
    q_ship: Query<(&GridCell<i64>, &Transform), With<ZenCamera>>,
    q_mass: Query<(&GridCell<i64>, &Transform, &Mass, &Radius)>,
) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }
    
    let Ok((ship_cell, ship_pos)) = q_ship.get_single() else { return; };
    
    // Pentatonic Scale Multipliers (Major Pentatonic)
    // 1.0, 1.125 (9/8), 1.25 (5/4), 1.5 (3/2), 1.66 (5/3) roughly
    let pitches = [0.5, 0.75, 1.0, 1.125, 1.25, 1.5]; 
    
    for (body_cell, body_tf, _mass, _radius) in q_mass.iter() {
         let cell_diff = *body_cell - *ship_cell;
            let large_diff = Vec3::new(
                cell_diff.x as f32 * GRID_SIZE, 
                cell_diff.y as f32 * GRID_SIZE,
                cell_diff.z as f32 * GRID_SIZE,
            );
            
        let body_rel_pos = body_tf.translation + large_diff;
        let dist_sq = (body_rel_pos - ship_pos.translation).length_squared();
        
        // 5000 units sensing radius -> 25,000,000 sq
        if dist_sq < 25_000_000.0 {
            // Chance to play chime based on distance? 
            // Or just random ping if close.
            let mut rng = rand::thread_rng();
            if rand::Rng::gen_bool(&mut rng, 0.1) { // 10% chance per check (every 0.5s) -> sparse chimes
                let pitch = *pitches.choose(&mut rng).unwrap_or(&1.0);
                
                commands.spawn((
                    AudioPlayer::new(assets.chime.clone()),
                    PlaybackSettings {
                        mode: bevy::audio::PlaybackMode::Despawn, // Play once and die
                        volume: bevy::audio::Volume::new(0.3),
                        speed: pitch,
                        ..default()
                    }
                ));
                // Only one chime per tick max to prevent spam
                break; 
            }
        }
    }
}
