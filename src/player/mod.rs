pub mod input;
pub mod camera;
pub mod prediction;
pub mod ui;
pub mod cockpit;

use bevy::prelude::*;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(input::MobileInputPlugin)
           .add_plugins(camera::ZenCameraPlugin)
           .add_plugins(prediction::TrajectoryPlugin)
           .add_plugins(ui::HudPlugin)
           .add_plugins(cockpit::CockpitPlugin);
    }
}
