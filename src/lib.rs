// Configure clippy for Bevy usage
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::enum_glob_use)]

use bevy::log::LogPlugin;
use bevy::{
    app::{ScheduleRunnerPlugin},
    core::{TaskPoolPlugin, TypeRegistrationPlugin, FrameCountPlugin},
    prelude::*,
    utils::Duration,
};

use bevy_save::SavePlugins;
use event::{MapEvents, GameEvents};
use game::{GamePlugin, Position, Merchant};
use item::Items;

mod account;
mod combat;
mod components;
mod effect;
mod event;
mod encounter;
mod experiment;
mod game;
mod item;
mod ids;
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
mod terrain_feature;

const TIMESTEP_10_PER_SECOND: f64 = 1.0 / 10.0;

pub fn setup() {
    App::new()
        .add_plugins(TaskPoolPlugin::default())
        .add_plugins(TypeRegistrationPlugin::default())
        .add_plugins(FrameCountPlugin::default())
        .add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            TIMESTEP_10_PER_SECOND,
        )))
        .add_plugins(LogPlugin {
            level: bevy::log::Level::INFO,
            filter: "big_brain=debug,siege_perilous::ai=info,siege_perilous::plugins::ai=info,siege_perilious::game=debug".into(),
        })
        .add_plugins(GamePlugin)
        .add_plugins(SavePlugins)
        .register_type::<Position>()
        .register_type::<Merchant>()
        .register_type::<Items>()
        .register_type::<MapEvents>()
        .register_type::<GameEvents>()
        .run();
}
