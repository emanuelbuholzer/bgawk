use std::fs;
use crate::objects::AttractionFactor;
use crate::ui::ZoomFactor;
use avian2d::prelude::Gravity;
use bevy::app::{App, Plugin, PostStartup};
use bevy::prelude::{Commands, Query, Res, ResMut, Resource, Window};
use bevy::time::{Time, Virtual};
use clap::Parser;
use figment::{
    providers::{Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use xdg::BaseDirectories;
use crate::lapis::Lapis;

pub struct ConfigPlugin;

#[derive(Parser, Debug, Resource, Serialize, Deserialize)]
#[command(version, about, long_about = None)]
pub struct Config {
    #[arg(long, default_value_t = false)]
    pub pause: bool,

    #[arg(long, default_value_t = 0.0)]
    pub gravity_x: f32,

    #[arg(long, default_value_t = 0.0)]
    pub gravity_y: f32,

    #[arg(long, default_value_t = 0.01)]
    pub attraction: f32,

    #[arg(long, default_value_t = 1.0)]
    pub zoom: f32,

    #[arg(long, short)]
    pub file: Option<String>
}

impl Plugin for ConfigPlugin {
    fn build(&self, app: &mut App) {
        let xdg_dirs = BaseDirectories::with_prefix("bgawk").unwrap();
        let config_path = xdg_dirs.place_config_file("config.toml").unwrap();

        let config: Config = Figment::new()
            .merge(Serialized::defaults(Config::parse()))
            .merge(Toml::file(config_path))
            .extract()
            .unwrap();

        app.insert_resource(config)
            .add_systems(PostStartup, configure);
    }
}

fn configure(
    config: Res<Config>,
    mut time: ResMut<Time<Virtual>>,
    mut gravity: ResMut<Gravity>,
    mut attraction_factor: ResMut<AttractionFactor>,
    mut zoom_factor: ResMut<ZoomFactor>,
    mut win: Query<&mut Window>,
    mut lapis: ResMut<Lapis>,
    mut commands: Commands,
) {
    if config.pause {
        time.pause();
    }

    gravity.0.x = config.gravity_x;
    gravity.0.y = config.gravity_y;

    attraction_factor.0 = config.attraction;

    zoom_factor.0 = config.zoom;
    win.single_mut().resolution.set_scale_factor(config.zoom);

    if config.file.is_some() {
        let path = config.file.clone().unwrap();
        match fs::read_to_string(path) {
            Ok(input) => {
                lapis.eval(&input, &mut commands); 
            }
            Err(err) => {
                println!("Error reading config file: {}", err);
            }
        }
    }
}
