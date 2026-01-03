pub mod input;
pub mod camera;
pub mod prediction;
pub mod ui;
pub mod cockpit;
pub mod navigation;
pub mod audio;

use bevy::prelude::*;

pub mod starmap;
pub mod label_update;
pub mod interaction;

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
