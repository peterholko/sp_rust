use bevy::prelude::*;
use std::collections::HashMap;
use big_brain::{prelude::*, evaluators::{PowerEvaluator, Evaluator, LinearEvaluator}};
use crate::game::Ids;

pub mod npc;
pub mod villager;

pub struct AIPlugin;

impl Plugin for AIPlugin {
    fn build(&self, app: &mut App) {

        // Initialize indexes
        let ids: Ids = Ids {
            map_event: 0,
            player_event: 0,
            obj: 0,
            item: 0,
            player_hero_map: HashMap::new(),
            obj_entity_map: HashMap::new(),
        };

        app.insert_resource(ids);


        app.add_plugin(BigBrainPlugin)
            .add_system(npc::nearby_target_system)
            .add_system_to_stage(
                BigBrainStage::Actions,
                villager::move_to_water_source_action_system,
            )
            .add_system_to_stage(
                BigBrainStage::Actions,
                villager::move_to_food_action_system,
            )
            // Actions
            .add_system_to_stage(BigBrainStage::Actions, villager::find_drink_system)
            .add_system_to_stage(BigBrainStage::Actions, villager::move_to_shelter_system)
            .add_system_to_stage(BigBrainStage::Actions, villager::transfer_drink_system)
            .add_system_to_stage(BigBrainStage::Actions, villager::drink_action_system)
            .add_system_to_stage(BigBrainStage::Actions, villager::find_food_system)
            .add_system_to_stage(BigBrainStage::Actions, villager::transfer_food_system)
            .add_system_to_stage(BigBrainStage::Actions, villager::eat_action_system)
            .add_system_to_stage(BigBrainStage::Actions, villager::sleep_action_system)
            .add_system_to_stage(BigBrainStage::Actions, villager::find_shelter_system)
            .add_system_to_stage(BigBrainStage::Actions, villager::move_to_shelter_system)
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
            //Tired scorers
            .add_system_to_stage(BigBrainStage::Scorers, villager::drowsy_scorer_system)
            .add_system_to_stage(BigBrainStage::Scorers, villager::find_shelter_scorer_system)
            .add_system_to_stage(BigBrainStage::Scorers, villager::shelter_distance_scorer_system)
            .add_system_to_stage(BigBrainStage::Scorers, villager::near_shelter_scorer_system)

            .add_system_to_stage(BigBrainStage::Scorers, villager::morale_scorer_system)
            .add_system_to_stage(BigBrainStage::Actions, npc::attack_target_system)
            .add_system_to_stage(BigBrainStage::Scorers, npc::target_scorer_system)
            
            // Enemy distance scorer
            .add_system_to_stage(BigBrainStage::Scorers, villager::enemy_distance_scorer_system)
            .add_system_to_stage(BigBrainStage::Actions, villager::flee_system);


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
