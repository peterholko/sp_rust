use bevy::prelude::*;
use big_brain::{
    evaluators::{Evaluator, LinearEvaluator, PowerEvaluator},
    prelude::*,
};

pub mod npc;
pub mod villager;

pub struct AIPlugin;

impl Plugin for AIPlugin {
    fn build(&self, app: &mut App) {


        app.add_plugins(BigBrainPlugin::new(PreUpdate))
            .add_systems(Update, npc::nearby_target_system)
            .add_systems(
                PreUpdate,
                (
                    villager::move_to_water_source_action_system.in_set(BigBrainSet::Actions),
                    villager::move_to_food_action_system.in_set(BigBrainSet::Actions),
                    villager::find_drink_system.in_set(BigBrainSet::Actions),
                    villager::move_to_shelter_system.in_set(BigBrainSet::Actions),
                    villager::transfer_drink_system.in_set(BigBrainSet::Actions),
                    villager::drink_action_system.in_set(BigBrainSet::Actions),
                    villager::find_food_system.in_set(BigBrainSet::Actions),
                    villager::transfer_food_system.in_set(BigBrainSet::Actions),
                    villager::eat_action_system.in_set(BigBrainSet::Actions),
                    villager::sleep_action_system.in_set(BigBrainSet::Actions),
                    villager::find_shelter_system.in_set(BigBrainSet::Actions),
                    villager::move_to_shelter_system.in_set(BigBrainSet::Actions),
                    villager::sleep_action_system.in_set(BigBrainSet::Actions),
                    villager::process_order_system.in_set(BigBrainSet::Actions),
                    npc::attack_target_system.in_set(BigBrainSet::Actions),
                    npc::target_scorer_system.in_set(BigBrainSet::Scorers),
                    villager::flee_system.in_set(BigBrainSet::Actions)
                )
            )
            .add_systems(
                PreUpdate,
                (
                    villager::enemy_distance_scorer_system.in_set(BigBrainSet::Scorers),
                    villager::thirsty_scorer_system.in_set(BigBrainSet::Scorers),
                    villager::find_drink_scorer_system.in_set(BigBrainSet::Scorers),
                    villager::drink_distance_scorer_system.in_set(BigBrainSet::Scorers),
                    villager::transfer_drink_scorer_system.in_set(BigBrainSet::Scorers),
                    villager::has_drink_scorer_system.in_set(BigBrainSet::Scorers),
                    villager::hungry_scorer_system.in_set(BigBrainSet::Scorers),
                    villager::find_food_scorer_system.in_set(BigBrainSet::Scorers),
                    villager::food_distance_scorer_system.in_set(BigBrainSet::Scorers),
                    villager::transfer_food_scorer_system.in_set(BigBrainSet::Scorers),
                    villager::has_food_scorer_system.in_set(BigBrainSet::Scorers),
                    villager::drowsy_scorer_system.in_set(BigBrainSet::Scorers),
                    villager::find_shelter_scorer_system.in_set(BigBrainSet::Scorers),
                    villager::shelter_distance_scorer_system.in_set(BigBrainSet::Scorers),
                    villager::near_shelter_scorer_system.in_set(BigBrainSet::Scorers),
                    villager::morale_scorer_system.in_set(BigBrainSet::Scorers),
                )
            );

        let linear = LinearEvaluator::new_inversed();
        debug!("linear: {:?}", linear.evaluate(0.1));
        debug!("linear: {:?}", linear.evaluate(0.25));
        debug!("linear: {:?}", linear.evaluate(0.5));
        debug!("linear: {:?}", linear.evaluate(0.75));
        debug!("linear: {:?}", linear.evaluate(1.0));
        debug!("linear: {:?}", linear.evaluate(2.0));

        let evaluator = PowerEvaluator::new(2.0);
        debug!("evaluator: {:?}", evaluator.evaluate(0.1));
        debug!("evaluator: {:?}", evaluator.evaluate(0.2));
        debug!("evaluator: {:?}", evaluator.evaluate(0.5));
        debug!("evaluator: {:?}", evaluator.evaluate(0.75));
        debug!("evaluator: {:?}", evaluator.evaluate(0.99));
        debug!("evaluator: {:?}", evaluator.evaluate(1.0));
        debug!("evaluator: {:?}", evaluator.evaluate(2.0));

        let evaluator = PowerEvaluator::new(4.0);
        debug!("evaluator: {:?}", evaluator.evaluate(0.1));
        debug!("evaluator: {:?}", evaluator.evaluate(0.2));
        debug!("evaluator: {:?}", evaluator.evaluate(0.5));
        debug!("evaluator: {:?}", evaluator.evaluate(0.75));
        debug!("evaluator: {:?}", evaluator.evaluate(0.99));
        debug!("evaluator: {:?}", evaluator.evaluate(1.0));
        debug!("evaluator: {:?}", evaluator.evaluate(2.0));
    }
}