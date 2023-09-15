use bevy::prelude::*;
use big_brain::{prelude::*, evaluators::{PowerEvaluator, Evaluator, LinearEvaluator}};

pub mod npc;
pub mod villager;

pub struct AIPlugin;

impl Plugin for AIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(BigBrainPlugin)
            .add_system(npc::nearby_target_system)
            .add_system_to_stage(
                BigBrainStage::Actions,
                villager::move_to_water_source_action_system,
            )
            .add_system_to_stage(
                BigBrainStage::Actions,
                villager::move_to_food_source_action_system,
            )
            // Actions
            .add_system_to_stage(BigBrainStage::Actions, villager::find_drink_system)
            .add_system_to_stage(BigBrainStage::Actions, villager::move_to_sleep_pos_action_system)
            .add_system_to_stage(BigBrainStage::Actions, villager::transfer_drink_system)
            .add_system_to_stage(BigBrainStage::Actions, villager::drink_action_system)
            .add_system_to_stage(BigBrainStage::Actions, villager::eat_action_system)
            .add_system_to_stage(BigBrainStage::Actions, villager::sleep_action_system)
            .add_system_to_stage(BigBrainStage::Actions, villager::process_order_system)
            // Thirsty scorers
            .add_system_to_stage(BigBrainStage::Scorers, villager::thirsty_scorer_system)
            .add_system_to_stage(BigBrainStage::Scorers, villager::find_drink_scorer_system)
            .add_system_to_stage(BigBrainStage::Scorers, villager::drink_distance_scorer_system)
            .add_system_to_stage(BigBrainStage::Scorers, villager::transfer_drink_scorer_system)
            .add_system_to_stage(BigBrainStage::Scorers, villager::has_drink_scorer_system)
            // Hunger scorers
            .add_system_to_stage(BigBrainStage::Scorers, villager::hungry_scorer_system)
            .add_system_to_stage(BigBrainStage::Scorers, villager::find_food_scorer_system)
            .add_system_to_stage(BigBrainStage::Scorers, villager::food_distance_scorer_system)
            .add_system_to_stage(BigBrainStage::Scorers, villager::transfer_food_scorer_system)
            .add_system_to_stage(BigBrainStage::Scorers, villager::has_food_scorer_system)            

            .add_system_to_stage(BigBrainStage::Scorers, villager::morale_scorer_system)
            .add_system_to_stage(BigBrainStage::Actions, npc::attack_target_system)
            .add_system_to_stage(BigBrainStage::Scorers, npc::target_scorer_system);


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
