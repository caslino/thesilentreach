pub mod audio;
pub mod camera;
pub mod cockpit;
pub mod input;
pub mod navigation;
pub mod prediction;
pub mod ui;

use bevy::prelude::*;

pub mod interaction;
pub mod label_update;
pub mod starmap;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(input::MobileInputPlugin)
            .add_plugins(camera::ZenCameraPlugin)
            .add_plugins(prediction::TrajectoryPlugin)
            .add_plugins(ui::HudPlugin)
            .add_plugins(label_update::LabelUpdatePlugin) // Sync In-World Labels
            .add_plugins(interaction::SystemConsolePlugin) // System Console & Renaming
            // .add_plugins(ui::DiscoveryUiPlugin) // REMOVED
            .add_plugins(starmap::StarMapPlugin)
            .add_plugins(cockpit::CockpitPlugin)
            .add_plugins(navigation::NavigationPlugin)
            .add_plugins(audio::ZenAudioPlugin);
    }
}
