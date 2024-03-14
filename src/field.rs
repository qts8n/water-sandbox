use bevy::prelude::*;

const STARTING_BG_COLOR: Color = Color::rgb(0.1, 0., 0.15);

const STARTING_LIGHT_COLOR: Color = Color::rgb(1., 1., 1.);
const STARTING_LIGHT_BRIGHTNESS: f32 = 1000.;


pub struct FieldPlugin;


impl Plugin for FieldPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(ClearColor(STARTING_BG_COLOR))
            .insert_resource(AmbientLight {
                color: STARTING_LIGHT_COLOR,
                brightness: STARTING_LIGHT_BRIGHTNESS,
            });
    }
}
