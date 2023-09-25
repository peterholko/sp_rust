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
            .add_system(
                villager::move_to_water_source_action_system.in_set(BigBrainSet::Actions)
            )
            .add_system(
                villager::move_to_food_action_system.in_set(BigBrainSet::Actions)
            )
            // Actions
            .add_system(villager::find_drink_system.in_set(BigBrainSet::Actions))
            .add_system(villager::move_to_shelter_system.in_set(BigBrainSet::Actions))
            .add_system(villager::transfer_drink_system.in_set(BigBrainSet::Actions))
            .add_system(villager::drink_action_system.in_set(BigBrainSet::Actions))
            .add_system(villager::find_food_system.in_set(BigBrainSet::Actions))
            .add_system(villager::transfer_food_system.in_set(BigBrainSet::Actions))
            .add_system(villager::eat_action_system.in_set(BigBrainSet::Actions))
            .add_system(villager::sleep_action_system.in_set(BigBrainSet::Actions))
            .add_system(villager::find_shelter_system.in_set(BigBrainSet::Actions))
            .add_system(villager::move_to_shelter_system.in_set(BigBrainSet::Actions))
            .add_system(villager::sleep_action_system.in_set(BigBrainSet::Actions))
            .add_system(villager::process_order_system.in_set(BigBrainSet::Actions))


            // Thirsty scorers
            .add_system(villager::thirsty_scorer_system.in_set(BigBrainSet::Scorers))
            .add_system(villager::find_drink_scorer_system.in_set(BigBrainSet::Scorers))
            .add_system(villager::drink_distance_scorer_system.in_set(BigBrainSet::Scorers))
            .add_system(villager::transfer_drink_scorer_system.in_set(BigBrainSet::Scorers))
            .add_system(villager::has_drink_scorer_system.in_set(BigBrainSet::Scorers))
            // Hunger scorers
            .add_system(villager::hungry_scorer_system.in_set(BigBrainSet::Scorers))
            .add_system(villager::find_food_scorer_system.in_set(BigBrainSet::Scorers))
            .add_system(villager::food_distance_scorer_system.in_set(BigBrainSet::Scorers))
            .add_system(villager::transfer_food_scorer_system.in_set(BigBrainSet::Scorers))
            .add_system(villager::has_food_scorer_system.in_set(BigBrainSet::Scorers))   
            //Tired scorers
            .add_system(villager::drowsy_scorer_system.in_set(BigBrainSet::Scorers))
            .add_system(villager::find_shelter_scorer_system.in_set(BigBrainSet::Scorers))
            .add_system(villager::shelter_distance_scorer_system.in_set(BigBrainSet::Scorers))
            .add_system(villager::near_shelter_scorer_system.in_set(BigBrainSet::Scorers))

            .add_system(villager::morale_scorer_system.in_set(BigBrainSet::Scorers))
            .add_system(npc::attack_target_system.in_set(BigBrainSet::Actions))
            .add_system(npc::target_scorer_system.in_set(BigBrainSet::Scorers))
            
            // Enemy distance scorer
            .add_system(villager::enemy_distance_scorer_system.in_set(BigBrainSet::Scorers))
            .add_system(villager::flee_system.in_set(BigBrainSet::Actions));


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
