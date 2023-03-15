// Configure clippy for Bevy usage
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::enum_glob_use)]

use bevy::{
    app::{ScheduleRunnerPlugin, ScheduleRunnerSettings},
    core::CorePlugin,
    prelude::*,
    utils::Duration,
};
use bevy::log::LogPlugin;

use game::GamePlugin;

mod game;
mod map;
mod ai;
mod player;
mod combat;
mod network;
mod templates;
mod item;
mod structure;
mod resource;
mod skill;
mod villager;

const TIMESTEP_10_PER_SECOND: f64 = 1.0 / 10.0;

pub fn setup() {
    App::new()
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs_f64(
            TIMESTEP_10_PER_SECOND,
        )))
        .add_plugin(CorePlugin::default())
        .add_plugin(ScheduleRunnerPlugin::default())
        .add_plugin(LogPlugin {
            level: bevy::log::Level::DEBUG,
            filter: "big_brain=warn,siege_perilous::ai=debug,siege_perilious::game=debug".into(),
        })        
        .add_plugin(GamePlugin)
        .run();
}
