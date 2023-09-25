// Configure clippy for Bevy usage
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::enum_glob_use)]

use bevy::log::LogPlugin;
use bevy::{
    app::{ScheduleRunnerPlugin, ScheduleRunnerSettings},
    core::{TaskPoolPlugin, TypeRegistrationPlugin, FrameCountPlugin},
    prelude::*,
    utils::Duration,
};

use game::GamePlugin;

mod account;
mod combat;
mod components;
mod encounter;
mod experiment;
mod game;
mod item;
mod map;
mod network;
mod obj;
mod player;
mod plugins;
mod recipe;
mod resource;
mod skill;
mod structure;
mod templates;
mod villager;

const TIMESTEP_10_PER_SECOND: f64 = 1.0 / 10.0;

pub fn setup() {
    App::new()
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs_f64(
            TIMESTEP_10_PER_SECOND,
        )))
        .add_plugin(TaskPoolPlugin::default())
        .add_plugin(TypeRegistrationPlugin::default())
        .add_plugin(FrameCountPlugin::default())
        .add_plugin(ScheduleRunnerPlugin::default())
        .add_plugin(LogPlugin {
            level: bevy::log::Level::DEBUG,
            filter: "big_brain=trace,siege_perilous::ai=debug,siege_perilious::game=debug".into(),
        })
        .add_plugin(GamePlugin)
        .run();
}
