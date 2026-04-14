use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use bevy::window::PrimaryWindow;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

pub struct RecorderPlugin;

impl Plugin for RecorderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RecordingState>()
            .init_resource::<RecordingDirectory>()
            .init_resource::<RecordingBlinkDuration>()
            .add_systems(
                Update,
                (
                    toggle_recording,
                    capture_frame,
                    blink_indicator,
                    cleanup_recording_message,
                ),
            );
    }
}

#[derive(Resource, Default)]
pub struct RecordingDirectory(pub String);

#[derive(Resource, Default)]
pub struct RecordingBlinkDuration(pub f32);

#[derive(Component)]
struct FrameRepeat(u32);

#[derive(Resource)]
struct RecordingState {
    is_recording: bool,
    sender: Option<Sender<(Vec<u8>, u32)>>,
    ui_root: Option<Entity>,
    blink_timer: Timer,
    time_accumulator: f32, // Accumulate real time to sync frames
    capture_timer: Timer,  // Cap capture rate
}

impl Default for RecordingState {
    fn default() -> Self {
        Self {
            is_recording: false,
            sender: None,
            ui_root: None,
            blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
            time_accumulator: 0.0,
            capture_timer: Timer::from_seconds(1.0 / 30.0, TimerMode::Repeating),
        }
    }
}

fn toggle_recording(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<RecordingState>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    rec_dir: Res<RecordingDirectory>,
    blink_duration: Res<RecordingBlinkDuration>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        if state.is_recording {
            // Stop recording
            state.is_recording = false;
            state.sender = None; // Dropping sender closes channel

            // Remove recording UI
            if let Some(entity) = state.ui_root {
                commands.entity(entity).despawn_recursive();
                state.ui_root = None;
            }

            // Show "Saved" message
            commands.spawn((
                Text::new("Recording Saved!"),
                TextFont {
                    font_size: 40.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(10.0),
                    right: Val::Px(10.0),
                    ..default()
                },
                RecordingSavedMessage {
                    timer: Timer::from_seconds(5.0, TimerMode::Once),
                },
            ));

            info!("Recording Stopped");
        } else {
            // Start recording
            let window = window_query.single();
            let width = window.resolution.physical_width() as u32;
            let height = window.resolution.physical_height() as u32;

            info!("Starting Recording: {}x{}", width, height);
            let (tx, rx) = channel::<(Vec<u8>, u32)>();
            state.sender = Some(tx);
            state.is_recording = true;
            state.time_accumulator = 0.0;
            state.capture_timer.reset();

            // Update blink timer
            if blink_duration.0 > 0.0 {
                state
                    .blink_timer
                    .set_duration(std::time::Duration::from_secs_f32(blink_duration.0));
                state.blink_timer.reset();
            }

            let output_dir = rec_dir.0.clone();
            thread::spawn(move || {
                encode_video(output_dir, width, height, rx);
            });

            // Spawn Recording UI (Red Dot)
            let id = commands
                .spawn((
                    Node {
                        width: Val::Px(20.0),
                        height: Val::Px(20.0),
                        position_type: PositionType::Absolute,
                        top: Val::Px(20.0),
                        right: Val::Px(20.0),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(1.0, 0.0, 0.0)),
                    BorderRadius::all(Val::Px(10.0)),
                ))
                .id();
            state.ui_root = Some(id);
        }
    }
}

fn blink_indicator(
    time: Res<Time>,
    mut state: ResMut<RecordingState>,
    mut query: Query<&mut Visibility>,
) {
    if state.is_recording {
        state.blink_timer.tick(time.delta());
        if state.blink_timer.finished() {
            if let Some(entity) = state.ui_root {
                if let Ok(mut vis) = query.get_mut(entity) {
                    *vis = match *vis {
                        Visibility::Inherited | Visibility::Visible => Visibility::Hidden,
                        Visibility::Hidden => Visibility::Visible,
                    };
                }
            }
        }
    }
}

fn capture_frame(
    mut commands: Commands,
    mut state: ResMut<RecordingState>,
    time: Res<Time>,
    window_query: Query<Entity, With<PrimaryWindow>>,
) {
    if state.is_recording {
        const TARGET_FPS: f32 = 30.0;
        const TARGET_DT: f32 = 1.0 / TARGET_FPS;

        // We still use capture_timer to rate-limit screenshot requests loosely,
        // but time_accumulator ensures precise sync.
        state.capture_timer.tick(time.delta());

        if state.capture_timer.finished() {
            state.time_accumulator += time.delta_secs();

            // If we have accumulated enough time for at least one frame
            if state.time_accumulator >= TARGET_DT {
                let frames_to_record = (state.time_accumulator / TARGET_DT).floor() as u32;
                state.time_accumulator -= frames_to_record as f32 * TARGET_DT;

                if let Ok(window_entity) = window_query.get_single() {
                    commands
                        .spawn((
                            Screenshot::window(window_entity),
                            FrameRepeat(frames_to_record),
                        ))
                        .observe(on_screenshot_captured);
                }
            }
        }
    }
}

fn on_screenshot_captured(
    trigger: Trigger<ScreenshotCaptured>,
    _commands: Commands,
    query: Query<&FrameRepeat>,
    state: Res<RecordingState>,
) {
    if let Some(sender) = &state.sender {
        // Retrieve the repeats from the component on the trigger entity
        let repeats = if let Ok(frame_repeat) = query.get(trigger.entity()) {
            frame_repeat.0
        } else {
            1
        };

        // ScreenshotCaptured wraps the image in the first field (tuple struct)
        let image = &trigger.event().0;
        let _ = sender.send((image.data.clone(), repeats));
    }
}

fn encode_video(output_dir: String, width: u32, height: u32, rx: Receiver<(Vec<u8>, u32)>) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let path = std::path::Path::new(&output_dir).join(format!("recording_{}.mp4", timestamp));
    let output_file = path.to_string_lossy().to_string();

    info!(
        "FFmpeg Encoding Thread Started (Hardware Accel): {}",
        output_file
    );

    // ffmpeg -f rawvideo -pixel_format bgra -video_size WxH -framerate 30 -i - -c:v h264_videotoolbox -b:v 10000k -pix_fmt yuv420p ...
    let mut child = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "rawvideo",
            "-pixel_format",
            "bgra",
            "-video_size",
            &format!("{}x{}", width, height),
            "-framerate",
            "30",
            "-i",
            "-",
            "-c:v",
            "h264_videotoolbox", // Hardware encoding
            "-b:v",
            "10000k", // Fixed high bitrate
            "-pix_fmt",
            "yuv420p",
            &output_file,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to spawn ffmpeg");

    if let Some(mut stdin) = child.stdin.take() {
        for (frame_data, repeats) in rx {
            for _ in 0..repeats {
                if let Err(e) = stdin.write_all(&frame_data) {
                    error!("Failed to write to ffmpeg stdin: {}", e);
                    break;
                }
            }
        }
    }

    let status = child.wait();
    info!("Recording saved: {:?} (Status: {:?})", output_file, status);
}

#[derive(Component)]
struct RecordingSavedMessage {
    timer: Timer,
}

fn cleanup_recording_message(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut RecordingSavedMessage)>,
) {
    for (entity, mut message) in query.iter_mut() {
        message.timer.tick(time.delta());
        if message.timer.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}
