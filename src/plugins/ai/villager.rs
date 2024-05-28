use bevy::ecs::query::WorldQuery;
use bevy::prelude::*;

use big_brain::prelude::*;

use crate::components::npc::Idle;
use crate::components::villager::Dehydrated;
use crate::components::villager::DrinkDistanceScorer;
use crate::components::villager::DrowsyScorer;
use crate::components::villager::EnemyDistanceScorer;
use crate::components::villager::Exhausted;
use crate::components::villager::FindDrink;
use crate::components::villager::FindDrinkScorer;
use crate::components::villager::FindFood;
use crate::components::villager::FindFoodScorer;
use crate::components::villager::FindShelter;
use crate::components::villager::FindShelterScorer;
use crate::components::villager::Flee;
use crate::components::villager::FoodDistanceScorer;
use crate::components::villager::GoodMorale;
use crate::components::villager::HasDrinkScorer;
use crate::components::villager::HasFoodScorer;
use crate::components::villager::IdleScorer;
use crate::components::villager::MoveToDrink;
use crate::components::villager::MoveToFood;
use crate::components::villager::MoveToInProgress;
use crate::components::villager::MoveToShelter;
use crate::components::villager::NearShelterScorer;
use crate::components::villager::NoDrinks;
use crate::components::villager::ProcessOrder;

use crate::components::villager::ShelterDistanceScorer;

use crate::components::villager::Starving;
use crate::components::villager::TransferDrink;
use crate::components::villager::TransferDrinkScorer;
use crate::components::villager::TransferFood;
use crate::components::villager::TransferFoodScorer;
use crate::constants::DEHYDRATED;
use crate::constants::EMERGENCY_SCORE;
use crate::constants::EXHAUSTED;
use crate::constants::MAX_ROUTINE_SCORE;
use crate::constants::SLIGHTLY_THIRSTY;
use crate::constants::STARVING;
use crate::constants::URGENT_SCORE;
use crate::event::{GameEvent, GameEventType, GameEvents, MapEvents, VisibleEvent};
use crate::experiment;
use crate::experiment::*;
use crate::game::State;
use crate::game::*;
use crate::ids::Ids;
use crate::item;
use crate::item::*;
use crate::map::Map;
use crate::map::MapPos;
use crate::network::ResponsePacket;
use crate::obj;
use crate::obj::Obj;
use crate::player;
use crate::player::*;
use crate::structure;
use crate::templates::Templates;
use crate::villager;
use crate::villager::*;

use crate::components::villager::{
    Drink, Eat, Hunger, HungryScorer, Morale, MoveToFoodSource, MoveToSleepPos, MoveToWaterSource,
    Sleep, Thirst, ThirstyScorer, Tired,
};

#[derive(WorldQuery)]
#[world_query(mutable, derive(Debug))]
pub struct VillagerQuery {
    id: &'static Id,
    player_id: &'static PlayerId,
    pos: &'static Position,
    class: &'static Class,
    state: &'static mut State,
    attrs: &'static mut VillagerAttrs,
}

#[derive(WorldQuery)]
#[world_query(mutable, derive(Debug))]
pub struct VillagerWithOrderQuery {
    id: &'static Id,
    player_id: &'static PlayerId,
    pos: &'static Position,
    class: &'static Class,
    order: &'static Order,
}

#[derive(WorldQuery)]
#[world_query(mutable, derive(Debug))]
pub struct BaseQuery {
    pub id: &'static Id,
    pub player_id: &'static PlayerId,
    pub pos: &'static Position,
    pub class: &'static Class,
    pub subclass: &'static Subclass,
    pub state: &'static State,
}

#[derive(WorldQuery)]
#[world_query(mutable, derive(Debug))]
pub struct VillagerBaseQuery {
    pub id: &'static Id,
    pub player_id: &'static PlayerId,
    pub pos: &'static Position,
    pub class: &'static Class,
    pub subclass: &'static Subclass,
    pub state: &'static State,
    pub attrs: &'static VillagerAttrs,
}

pub fn enemy_distance_scorer_system(
    ids: ResMut<Ids>,
    hero_query: Query<MapObjQuery, With<SubclassHero>>,
    obj_query: Query<MapObjQuery, Without<SubclassHero>>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<EnemyDistanceScorer>>,
) {
    for (Actor(actor), mut score, _span) in &mut query {
        if let Ok(villager) = obj_query.get(*actor) {
            let Some(hero_id) = ids.get_hero(villager.player_id.0) else {
                error!("Cannot find hero for player {:?}", villager.player_id);
                continue;
            };

            let Some(hero_entity) = ids.get_entity(hero_id) else {
                error!("Cannot find hero entity for hero {:?}", hero_id);
                continue;
            };

            let Ok(_hero) = hero_query.get(hero_entity) else {
                error!("Cannot find hero for {:?}", hero_entity);
                continue;
            };

            let mut nearby_enemies = false;

            for obj in obj_query.iter() {
                if *obj.state == State::Dead {
                    continue;
                }

                if obj.player_id.0 != villager.player_id.0 {
                    let distance =
                        Map::distance((villager.pos.x, villager.pos.y), (obj.pos.x, obj.pos.y));

                    if distance <= 2 {
                        nearby_enemies = true;
                    }
                }
            }

            if nearby_enemies {
                score.set(1.0);
            } else {
                score.set(0.0);
            }
        }
    }
}

pub fn idle_scorer_system(
    templates: Res<Templates>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<IdleScorer>>,
) {
    for (Actor(actor), mut score, _span) in &mut query {
        score.set(0.5);
    }
}

pub fn thirsty_scorer_system(
    thirsts: Query<&Thirst>,
    dehydrated: Query<&Dehydrated>,
    no_drinks: Query<&NoDrinks>,
    villager_attrs: Query<&VillagerAttrs>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<ThirstyScorer>>,
) {
    for (Actor(actor), mut score, _span) in &mut query {
        if let Ok(thirst) = thirsts.get(*actor) {
            let Ok(villager_attrs) = villager_attrs.get(*actor) else {
                error!("No villager attrs component for {:?}", *actor);
                continue;
            };
            let mut thirst_score;

            if villager_attrs.activity == villager::Activity::Fleeing && thirst.thirst >= DEHYDRATED
            {
                thirst_score = EMERGENCY_SCORE;
            } else if villager_attrs.activity == villager::Activity::GettingDrink {
                thirst_score = thirst.thirst * 1.50;

                if thirst_score >= MAX_ROUTINE_SCORE {
                    thirst_score = MAX_ROUTINE_SCORE;
                }
            } else {
                thirst_score = thirst.thirst;

                if thirst_score >= MAX_ROUTINE_SCORE {
                    thirst_score = MAX_ROUTINE_SCORE;
                }
            }
            score.set(thirst_score / 100.0);
            /*debug!(
                "thirst score: {:?} activity: {:?}",
                thirst_score, villager_attrs.activity
            );*/

            // For now just set score to 1.0 if dehydrated
            /*if let Ok(_dehydrated) = dehydrated.get(*actor) {
                score.set(0.99);
            } else {
                //let evaluator = PowerEvaluator::new(2.0);
                //evaluator.evaluate(thrist_percentage)





                let mut thirst_mod = 1.0;

                if villager_attrs.activity == villager::Activity::GettingDrink {
                    // Apply modifier if the villager is drinking
                    thirst_mod = 1.5;
                }

                if let Ok(_no_drinks) = no_drinks.get(*actor) {
                    thirst_mod = 0.0;
                }

                let mut thrist_percentage = thirst.thirst * thirst_mod / 100.0;

                if thrist_percentage < 0.0 {
                    thrist_percentage = 0.0;
                } else if thrist_percentage > 1.0 {
                    thrist_percentage = 0.99;
                }

                score.set(thrist_percentage);
            }*/
        }
    }
}

pub fn hungry_scorer_system(
    hungers: Query<&Hunger>,
    starving: Query<&Starving>,
    villager_attrs: Query<&VillagerAttrs>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<HungryScorer>>,
) {
    for (Actor(actor), mut score, _span) in &mut query {
        if let Ok(hunger) = hungers.get(*actor) {
            let Ok(villager_attrs) = villager_attrs.get(*actor) else {
                error!("No villager attrs component for {:?}", *actor);
                continue;
            };
            let mut hunger_score;

            if villager_attrs.activity == villager::Activity::Fleeing && hunger.hunger >= STARVING {
                hunger_score = EMERGENCY_SCORE;
            } else if villager_attrs.activity == villager::Activity::GettingFood {
                hunger_score = hunger.hunger * 1.50;

                if hunger_score >= MAX_ROUTINE_SCORE {
                    hunger_score = MAX_ROUTINE_SCORE;
                }
            } else {
                hunger_score = hunger.hunger;

                if hunger_score >= MAX_ROUTINE_SCORE {
                    hunger_score = MAX_ROUTINE_SCORE;
                }
            }
            score.set(hunger_score / 100.0);
            /*debug!(
                "hunger score: {:?} activity: {:?}",
                hunger_score, villager_attrs.activity
            );*/

            /*// For now just set score to 1.0 if starving
            if let Ok(_starving) = starving.get(*actor) {
                score.set(0.99);
            } else {
                let Ok(villager_attrs) = villager_attrs.get(*actor) else {
                    error!("No villager attrs {:?}", *actor);
                    continue;
                };

                let mut hunger_mod = 1.0;

                //debug!("Villager Activity: {:?}", villager_attrs.activity);
                if villager_attrs.activity == villager::Activity::Eating {
                    // Apply modifier if the villager is drinking
                    hunger_mod = 1.5;
                }

                let mut hunger_percentage = hunger.hunger * hunger_mod / 100.0;

                if hunger_percentage < 0.0 {
                    hunger_percentage = 0.0;
                } else if hunger_percentage > 1.0 {
                    hunger_percentage = 0.99;
                }

                debug!(
                    "hungry score: {:?} activity: {:?}",
                    hunger_percentage, villager_attrs.activity
                );
                score.set(hunger_percentage);
            }*/
        }
    }
}

pub fn drowsy_scorer_system(
    tired_query: Query<&Tired>,
    exhausted: Query<&Exhausted>,
    villager_attrs: Query<&VillagerAttrs>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<DrowsyScorer>>,
) {
    for (Actor(actor), mut score, _span) in &mut query {
        if let Ok(tired) = tired_query.get(*actor) {
            let Ok(villager_attrs) = villager_attrs.get(*actor) else {
                error!("No villager attrs component for {:?}", *actor);
                continue;
            };
            let mut tired_score;

            if villager_attrs.activity == villager::Activity::Fleeing && tired.tired >= EXHAUSTED {
                tired_score = EMERGENCY_SCORE;
            } else if villager_attrs.activity == villager::Activity::FindingShelter {
                tired_score = tired.tired * 1.50;

                if tired_score >= MAX_ROUTINE_SCORE {
                    tired_score = MAX_ROUTINE_SCORE;
                }
            } else {
                tired_score = tired.tired;

                if tired_score >= MAX_ROUTINE_SCORE {
                    tired_score = MAX_ROUTINE_SCORE;
                }
            }
            score.set(tired_score / 100.0);
            /*debug!(
                "tired score: {:?} activity: {:?}",
                tired_score, villager_attrs.activity
            );*/

            // For now just set score to 1.0 if exhausted
            /*if let Ok(_exhausted) = exhausted.get(*actor) {
                score.set(0.99);
            } else {
                let Ok(villager_attrs) = villager_attrs.get(*actor) else {
                    error!("No villager attrs component for {:?}", *actor);
                    continue;
                };

                let mut tired_mod = 1.0;

                //debug!("Villager Activity: {:?}", villager_attrs.activity);
                if villager_attrs.activity == villager::Activity::FindingShelter {
                    // Apply modifier if the villager is drinking
                    tired_mod = 1.5;
                }

                let mut tired_percentage = tired.tired * tired_mod / 100.0;

                if tired_percentage < 0.0 {
                    tired_percentage = 0.0;
                } else if tired_percentage > 1.0 {
                    tired_percentage = 0.99;
                }

                debug!(
                    "drowsy score: {:?} activity: {:?}",
                    tired_percentage, villager_attrs.activity
                );
                score.set(tired_percentage);
            }*/
        }
    }
}

pub fn morale_scorer_system(
    morale_query: Query<&Morale>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<GoodMorale>>,
) {
    for (Actor(actor), mut score, _span) in &mut query {
        if let Ok(_morale) = morale_query.get(*actor) {
            score.set(0.6);
            /*if tired.tired >= 80.0 {
                span.span()
                    .in_scope(|| debug!("Tired above threshold! Score: {}", tired.tired / 100.0));
            }*/
        }
    }
}

pub fn idle_action_systel(
    mut attrs_query: Query<&mut VillagerAttrs>,
    mut query: Query<(&Actor, &mut ActionState, &Idle, &ActionSpan)>,
) {
    for (Actor(actor), mut state, _idle, _span) in &mut query {
        if let Ok(mut attrs) = attrs_query.get_mut(*actor) {
            attrs.activity = villager::Activity::Idle;
        }

        *state = ActionState::Success;
    }
}

pub fn process_order_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    clients: Res<Clients>,
    mut ids: ResMut<Ids>,
    map: Res<Map>,
    mut map_events: ResMut<MapEvents>,
    mut experiments: ResMut<Experiments>,
    items: ResMut<Items>,
    active_infos: Res<ActiveInfos>,
    templates: Res<Templates>,
    villager_query: Query<VillagerWithOrderQuery, (With<Order>, Without<EventInProgress>)>,
    obj_query: Query<(&Id, &PlayerId, &Position)>,
    template_query: Query<&Template>,
    mut attrs_query: Query<&mut VillagerAttrs>,
    mut state_query: Query<&mut State>,
    mut query: Query<(&Actor, &mut ActionState, &ProcessOrder, &ActionSpan)>,
) {
    for (Actor(actor), mut state, _process_order, _span) in &mut query {
        match *state {
            ActionState::Requested => {
                let Ok(villager) = villager_query.get(*actor) else {
                    continue;
                };

                let Ok(mut villager_attrs) = attrs_query.get_mut(*actor) else {
                    error!("No villager attrs component for {:?}", *actor);
                    continue;
                };

                let Ok(villager_state) = state_query.get_mut(*actor) else {
                    error!("No state component for {:?}", *actor);
                    continue;
                };

                debug!("Process Order Requested: {:?}", villager.order);

                match villager.order {
                    Order::Follow { target } => {
                        debug!("Process Follow Order");
                        if let Ok((_id, _player, target_pos)) = obj_query.get(*target) {
                            if villager.pos.x != target_pos.x || villager.pos.y != target_pos.y {
                                if *villager_state == State::None {
                                    debug!("Executing Following");

                                    if villager_attrs.activity != Activity::Following {
                                        Obj::add_sound_obj_event(
                                            game_tick.0,
                                            Villager::order_to_speech(&Order::Follow {
                                                target: *target,
                                            }),
                                            villager.id,
                                            &mut map_events,
                                        );
                                    }

                                    villager_attrs.activity = Activity::Following;
                                    *state = ActionState::Executing;
                                }
                            }
                        } else {
                            trace!("Invalid target to follow.");
                        }
                    }
                    Order::Gather { res_type } => {
                        debug!("Process Gather Order");
                        if *villager_state == State::None {
                            debug!("Executing Gathering");

                            if villager_attrs.activity != Activity::Gathering {
                                Obj::add_sound_obj_event(
                                    game_tick.0,
                                    Villager::order_to_speech(&Order::Gather {
                                        res_type: res_type.clone(),
                                    }),
                                    villager.id,
                                    &mut map_events,
                                );
                            }

                            villager_attrs.activity = Activity::Gathering;
                            *state = ActionState::Executing;
                        }
                    }
                    Order::Operate => {
                        debug!("Process Operate Order");
                        if *villager_state == State::None {
                            debug!("Executing Operate");

                            if villager_attrs.activity == Activity::Operating {
                                Obj::add_sound_obj_event(
                                    game_tick.0,
                                    templates.get_dialogue("Operate"),
                                    villager.id,
                                    &mut map_events,
                                );
                            }

                            villager_attrs.activity = Activity::Operating;
                            *state = ActionState::Executing;
                        }
                    }
                    Order::Refine => {
                        debug!("Process Refine Order");
                        if *villager_state == State::None {
                            debug!("Executing Refining");

                            if villager_attrs.activity != Activity::Refining {
                                Obj::add_sound_obj_event(
                                    game_tick.0,
                                    Villager::order_to_speech(&Order::Refine),
                                    villager.id,
                                    &mut map_events,
                                );
                            }

                            villager_attrs.activity = Activity::Refining;
                            *state = ActionState::Executing;
                        }
                    }
                    Order::Craft { recipe_name } => {
                        debug!("Process Craft Order {:?}", recipe_name);
                        if *villager_state == State::None {
                            debug!("Executing Crafting");
                            if villager_attrs.activity != Activity::Crafting {
                                Obj::add_sound_obj_event(
                                    game_tick.0,
                                    Villager::order_to_speech(&Order::Craft {
                                        recipe_name: recipe_name.clone(),
                                    }),
                                    villager.id,
                                    &mut map_events,
                                );
                            }
                            villager_attrs.activity = Activity::Crafting;
                            *state = ActionState::Executing;
                        }
                    }
                    Order::Experiment => {
                        debug!("Process Experiment Order");
                        if *villager_state == State::None {
                            debug!("Executing Experiment");
                            if villager_attrs.activity != Activity::Experimenting {
                                Obj::add_sound_obj_event(
                                    game_tick.0,
                                    Villager::order_to_speech(&Order::Experiment),
                                    villager.id,
                                    &mut map_events,
                                );
                            }
                            villager_attrs.activity = Activity::Experimenting;
                            *state = ActionState::Executing;
                        }
                    }
                    Order::Plant => {
                        debug!("Process Plant Order");
                        if *villager_state == State::None {
                            debug!("Executing Plant");
                            if villager_attrs.activity != Activity::Planting {
                                Obj::add_sound_obj_event(
                                    game_tick.0,
                                    Villager::order_to_speech(&Order::Plant),
                                    villager.id,
                                    &mut map_events,
                                );
                            }
                            villager_attrs.activity = Activity::Planting;
                            *state = ActionState::Executing;
                        }
                    }
                    Order::Harvest => {
                        debug!("Process Harvest Order");
                        if *villager_state == State::None {
                            debug!("Executing Harvest");
                            if villager_attrs.activity != Activity::Harvesting {
                                Obj::add_sound_obj_event(
                                    game_tick.0,
                                    Villager::order_to_speech(&Order::Harvest),
                                    villager.id,
                                    &mut map_events,
                                );
                            }
                            villager_attrs.activity = Activity::Harvesting;
                            *state = ActionState::Executing;
                        }
                    }
                    Order::Explore => {
                        debug!("Process Explore Order");
                        if *villager_state == State::None {
                            debug!("Executing Explore");
                            if villager_attrs.activity != Activity::Exploring {
                                Obj::add_sound_obj_event(
                                    game_tick.0,
                                    Villager::order_to_speech(&Order::Explore),
                                    villager.id,
                                    &mut map_events,
                                );
                            }
                            villager_attrs.activity = Activity::Experimenting;
                            *state = ActionState::Executing;
                        }
                    }
                    _ => {}
                }
            }
            ActionState::Executing => {
                trace!("Process Order Executing");

                let Ok(villager) = villager_query.get(*actor) else {
                    debug!("No order to execute or villager is busy");
                    continue;
                };

                let blocking_list =
                    Obj::blocking_list(villager.player_id.0, actor, &obj_query, &state_query);

                let Ok(mut villager_attrs) = attrs_query.get_mut(*actor) else {
                    error!("No villager attrs component for {:?}", *actor);
                    continue;
                };

                let Ok(mut villager_state) = state_query.get_mut(*actor) else {
                    error!("No state component for {:?}", *actor);
                    continue;
                };

                debug!("Processing villager order: {:?}", villager.order);

                match villager.order {
                    Order::Follow { target } => {
                        if let Ok((_id, _player_id, target_pos)) = obj_query.get(*target) {
                            if villager.pos.x != target_pos.x || villager.pos.y != target_pos.y {
                                if *villager_state == State::None {
                                    if let Some(path_result) = Map::find_path(
                                        *villager.pos,
                                        *target_pos,
                                        &map,
                                        blocking_list,
                                        true,
                                        false,
                                        false,
                                        false,
                                    ) {
                                        debug!("Follower path: {:?}", path_result);

                                        let (path, _c) = path_result;
                                        let next_pos = &path[1];

                                        debug!("Next pos: {:?}", next_pos);

                                        // Add State Change Event to Moving
                                        let state_change_event = VisibleEvent::StateChangeEvent {
                                            new_state: "moving".to_string(),
                                        };

                                        *villager_state = State::Moving;

                                        map_events.new(
                                            villager.id.0,
                                            game_tick.0 + 4,
                                            state_change_event,
                                        );

                                        // Add Move Event
                                        let move_event = VisibleEvent::MoveEvent {
                                            src: *villager.pos,
                                            dst: Position {
                                                x: next_pos.0,
                                                y: next_pos.1,
                                            },
                                        };

                                        let move_map_event = map_events.new(
                                            villager.id.0,
                                            game_tick.0 + 36, // in the future
                                            move_event,
                                        );

                                        commands.entity(*actor).insert(EventInProgress {
                                            event_id: move_map_event.event_id,
                                        });
                                    }
                                } else {
                                    debug!("Follower is still moving");
                                    // ActionState is now executing
                                    debug!("Executing action for entity: {:?}", *actor);
                                }
                            } else {
                                debug!("Follower has reached destination");
                                *state = ActionState::Success;
                            }
                        }
                    }
                    Order::Gather { res_type } => {
                        if *villager_state == State::None {
                            let gather_event = VisibleEvent::GatherEvent {
                                res_type: res_type.clone(),
                            };

                            map_events.new(
                                villager.id.0,
                                game_tick.0 + 8, // in the future
                                gather_event,
                            );
                        }
                    }
                    Order::Refine | Order::Operate => {
                        if *villager_state == State::None {
                            let Some(structure_entity) = ids.get_entity(villager_attrs.structure)
                            else {
                                error!(
                                    "Cannot find structure entity for {:?}",
                                    villager_attrs.structure
                                );
                                continue;
                            };

                            let Ok((_id, _player_, structure_pos)) =
                                obj_query.get(structure_entity)
                            else {
                                error!("Query failed to find entity {:?}", structure_entity);
                                continue;
                            };

                            // Check if villager is on structure
                            if villager.pos.x != structure_pos.x
                                || villager.pos.y != structure_pos.y
                            {
                                if let Some(path_result) = Map::find_path(
                                    *villager.pos,
                                    *structure_pos,
                                    &map,
                                    blocking_list,
                                    true,
                                    false,
                                    false,
                                    false,
                                ) {
                                    let (path, _c) = path_result;
                                    let next_pos = &path[1];

                                    // Add State Change Event to Moving
                                    let state_change_event = VisibleEvent::StateChangeEvent {
                                        new_state: "moving".to_string(),
                                    };

                                    *villager_state = State::Moving;

                                    map_events.new(
                                        villager.id.0,
                                        game_tick.0 + 4,
                                        state_change_event,
                                    );

                                    // Add Move Event
                                    let move_event = VisibleEvent::MoveEvent {
                                        src: *villager.pos,
                                        dst: Position {
                                            x: next_pos.0,
                                            y: next_pos.1,
                                        },
                                    };

                                    let move_map_event = map_events.new(
                                        villager.id.0,
                                        game_tick.0 + 36, // in the future
                                        move_event,
                                    );

                                    commands.entity(*actor).insert(EventInProgress {
                                        event_id: move_map_event.event_id,
                                    });
                                }
                            } else {
                                let map_event;

                                match villager.order {
                                    Order::Refine => {
                                        let refine_event = VisibleEvent::RefineEvent {
                                            structure_id: villager_attrs.structure,
                                        };

                                        *villager_state = State::Refining;

                                        map_event = map_events.new(
                                            villager.id.0,
                                            game_tick.0 + 120, // in the future
                                            refine_event,
                                        );
                                    }
                                    Order::Operate => {
                                        let Ok(template) = template_query.get(structure_entity)
                                        else {
                                            error!(
                                                "No template for structure entity {:?}",
                                                structure_entity
                                            );
                                            continue;
                                        };

                                        let operate_event = VisibleEvent::OperateEvent {
                                            structure_id: villager_attrs.structure,
                                        };

                                        *villager_state =
                                            Villager::get_state_from_structure(template.0.clone());

                                        map_event = map_events.new(
                                            villager.id.0,
                                            game_tick.0 + 40, // in the future
                                            operate_event,
                                        );
                                    }
                                    _ => {
                                        error!("Invalid order type: {:?}", villager.order);
                                        continue;
                                    }
                                }
                                commands.entity(*actor).insert(EventInProgress {
                                    event_id: map_event.event_id,
                                });

                                *state = ActionState::Success;
                            }
                        }
                    }
                    Order::Craft { recipe_name } => {
                        if *villager_state == State::None {
                            let Some(structure_entity) = ids.get_entity(villager_attrs.structure)
                            else {
                                error!(
                                    "Cannot find structure entity for {:?}",
                                    villager_attrs.structure
                                );
                                continue;
                            };

                            let Ok((_id, _player_id, structure_pos)) =
                                obj_query.get(structure_entity)
                            else {
                                error!("Query failed to find entity {:?}", structure_entity);
                                continue;
                            };

                            // Check if villager is on structure
                            if villager.pos.x != structure_pos.x
                                || villager.pos.y != structure_pos.y
                            {
                                if let Some(path_result) = Map::find_path(
                                    *villager.pos,
                                    *structure_pos,
                                    &map,
                                    blocking_list,
                                    true,
                                    false,
                                    false,
                                    false,
                                ) {
                                    debug!("Path to structure: {:?}", path_result);

                                    let (path, _c) = path_result;
                                    let next_pos = &path[1];

                                    debug!("Next pos: {:?}", next_pos);

                                    // Add State Change Event to Moving
                                    let state_change_event = VisibleEvent::StateChangeEvent {
                                        new_state: "moving".to_string(),
                                    };

                                    *villager_state = State::Moving;

                                    map_events.new(
                                        villager.id.0,
                                        game_tick.0 + 4,
                                        state_change_event,
                                    );

                                    // Add Move Event
                                    let move_event = VisibleEvent::MoveEvent {
                                        src: *villager.pos,
                                        dst: Position {
                                            x: next_pos.0,
                                            y: next_pos.1,
                                        },
                                    };

                                    let _event_id = ids.new_map_event_id();

                                    let map_event = map_events.new(
                                        villager.id.0,
                                        game_tick.0 + 36, // in the future
                                        move_event,
                                    );

                                    commands.entity(*actor).insert(EventInProgress {
                                        event_id: map_event.event_id,
                                    });
                                }
                            } else {
                                // Create craft event
                                let craft_event = VisibleEvent::CraftEvent {
                                    structure_id: villager_attrs.structure,
                                    recipe_name: recipe_name.clone(),
                                };

                                // Add State Change Event to Moving
                                let state_change_event = VisibleEvent::StateChangeEvent {
                                    new_state: "crafting".to_string(),
                                };

                                *villager_state = State::Crafting;

                                map_events.new(villager.id.0, game_tick.0 + 4, state_change_event);

                                let _event_id = ids.new_map_event_id();

                                let map_event = map_events.new(
                                    villager.id.0,
                                    game_tick.0 + 200, // in the future
                                    craft_event,
                                );

                                commands.entity(*actor).insert(EventInProgress {
                                    event_id: map_event.event_id,
                                });

                                *state = ActionState::Success;
                            }
                        }
                    }
                    Order::Experiment => {
                        if *villager_state == State::None {
                            let Some(structure_entity) = ids.get_entity(villager_attrs.structure)
                            else {
                                error!(
                                    "Cannot find structure entity for {:?}",
                                    villager_attrs.structure
                                );
                                continue;
                            };

                            let Ok((_id, _player, structure_pos)) = obj_query.get(structure_entity)
                            else {
                                error!("Query failed to find entity {:?}", structure_entity);
                                continue;
                            };

                            // Check if villager is on structure
                            if villager.pos.x != structure_pos.x
                                || villager.pos.y != structure_pos.y
                            {
                                if let Some(path_result) = Map::find_path(
                                    *villager.pos,
                                    *structure_pos,
                                    &map,
                                    blocking_list,
                                    true,
                                    false,
                                    false,
                                    false,
                                ) {
                                    debug!("Path to structure: {:?}", path_result);

                                    let (path, _c) = path_result;
                                    let next_pos = &path[1];

                                    debug!("Next pos: {:?}", next_pos);

                                    // Add State Change Event to Moving
                                    let state_change_event = VisibleEvent::StateChangeEvent {
                                        new_state: "moving".to_string(),
                                    };

                                    *villager_state = State::Moving;

                                    map_events.new(
                                        villager.id.0,
                                        game_tick.0 + 4,
                                        state_change_event,
                                    );

                                    // Add Move Event
                                    let move_event = VisibleEvent::MoveEvent {
                                        src: *villager.pos,
                                        dst: Position {
                                            x: next_pos.0,
                                            y: next_pos.1,
                                        },
                                    };

                                    let map_event = map_events.new(
                                        villager.id.0,
                                        game_tick.0 + 36, // in the future
                                        move_event,
                                    );

                                    commands.entity(*actor).insert(EventInProgress {
                                        event_id: map_event.event_id,
                                    });
                                }
                            } else {
                                // Create experiment event
                                let experiment_event = VisibleEvent::ExperimentEvent {
                                    structure_id: villager_attrs.structure,
                                };

                                // Add State Change Event to Moving
                                let state_change_event = VisibleEvent::StateChangeEvent {
                                    new_state: "experimenting".to_string(),
                                };

                                *villager_state = State::Experimenting;

                                map_events.new(villager.id.0, game_tick.0 + 4, state_change_event);

                                let map_event = map_events.new(
                                    villager.id.0,
                                    game_tick.0 + 100, // in the future
                                    experiment_event,
                                );

                                commands.entity(*actor).insert(EventInProgress {
                                    event_id: map_event.event_id,
                                });

                                // Update experiment state to progressing
                                let updated_experiment = Experiment::update_state(
                                    villager_attrs.structure,
                                    experiment::ExperimentState::Progressing,
                                    &mut experiments,
                                );

                                if let Some(updated_experiment) = updated_experiment {
                                    player::active_info_experiment(
                                        villager.player_id.0,
                                        villager_attrs.structure,
                                        updated_experiment,
                                        &items,
                                        &active_infos,
                                        &clients,
                                    );
                                }

                                *state = ActionState::Success;
                            }
                        }
                    }
                    Order::Plant => {
                        if *villager_state == State::None {
                            let Some(structure_entity) = ids.get_entity(villager_attrs.structure)
                            else {
                                error!(
                                    "Cannot find structure entity for {:?}",
                                    villager_attrs.structure
                                );
                                continue;
                            };

                            let Ok((_id, _player, structure_pos)) = obj_query.get(structure_entity)
                            else {
                                error!("Query failed to find entity {:?}", structure_entity);
                                continue;
                            };

                            // Check if villager is on structure
                            if villager.pos.x != structure_pos.x
                                || villager.pos.y != structure_pos.y
                            {
                                if let Some(path_result) = Map::find_path(
                                    *villager.pos,
                                    *structure_pos,
                                    &map,
                                    blocking_list,
                                    true,
                                    false,
                                    false,
                                    false,
                                ) {
                                    debug!("Path to structure: {:?}", path_result);

                                    let (path, _c) = path_result;
                                    let next_pos = &path[1];

                                    debug!("Next pos: {:?}", next_pos);

                                    // Add State Change Event to Moving
                                    let state_change_event = VisibleEvent::StateChangeEvent {
                                        new_state: "moving".to_string(),
                                    };

                                    *villager_state = State::Moving;

                                    map_events.new(
                                        villager.id.0,
                                        game_tick.0 + 4,
                                        state_change_event,
                                    );

                                    // Add Move Event
                                    let move_event = VisibleEvent::MoveEvent {
                                        src: *villager.pos,
                                        dst: Position {
                                            x: next_pos.0,
                                            y: next_pos.1,
                                        },
                                    };

                                    let map_event = map_events.new(
                                        villager.id.0,
                                        game_tick.0 + 36, // in the future
                                        move_event,
                                    );

                                    commands.entity(*actor).insert(EventInProgress {
                                        event_id: map_event.event_id,
                                    });
                                }
                            } else {
                                // Create plant event
                                let plant_event = VisibleEvent::PlantEvent {
                                    structure_id: villager_attrs.structure,
                                };

                                // Add State Change Event to Moving
                                let state_change_event = VisibleEvent::StateChangeEvent {
                                    new_state: "planting".to_string(),
                                };

                                *villager_state = State::Planting;

                                map_events.new(villager.id.0, game_tick.0 + 4, state_change_event);

                                let map_event = map_events.new(
                                    villager.id.0,
                                    game_tick.0 + 50, // in the future
                                    plant_event,
                                );

                                commands.entity(*actor).insert(EventInProgress {
                                    event_id: map_event.event_id,
                                });

                                *state = ActionState::Success;
                            }
                        }
                    }
                    Order::Harvest => {
                        info!("Order::Harvest");
                        if *villager_state == State::None {
                            let Some(structure_entity) = ids.get_entity(villager_attrs.structure)
                            else {
                                error!(
                                    "Cannot find structure entity for {:?}",
                                    villager_attrs.structure
                                );
                                continue;
                            };

                            let Ok((_id, _player, structure_pos)) = obj_query.get(structure_entity)
                            else {
                                error!("Query failed to find entity {:?}", structure_entity);
                                continue;
                            };

                            // Check if villager is on structure
                            if villager.pos.x != structure_pos.x
                                || villager.pos.y != structure_pos.y
                            {
                                if let Some(path_result) = Map::find_path(
                                    *villager.pos,
                                    *structure_pos,
                                    &map,
                                    blocking_list,
                                    true,
                                    false,
                                    false,
                                    false,
                                ) {
                                    debug!("Path to structure: {:?}", path_result);

                                    let (path, _c) = path_result;
                                    let next_pos = &path[1];

                                    debug!("Next pos: {:?}", next_pos);

                                    // Add State Change Event to Moving
                                    let state_change_event = VisibleEvent::StateChangeEvent {
                                        new_state: "moving".to_string(),
                                    };

                                    *villager_state = State::Moving;

                                    map_events.new(
                                        villager.id.0,
                                        game_tick.0 + 4,
                                        state_change_event,
                                    );

                                    // Add Move Event
                                    let move_event = VisibleEvent::MoveEvent {
                                        src: *villager.pos,
                                        dst: Position {
                                            x: next_pos.0,
                                            y: next_pos.1,
                                        },
                                    };

                                    let map_event = map_events.new(
                                        villager.id.0,
                                        game_tick.0 + 36, // in the future
                                        move_event,
                                    );

                                    commands.entity(*actor).insert(EventInProgress {
                                        event_id: map_event.event_id,
                                    });
                                }
                            } else {
                                info!("Creating Harvest Event");
                                // Create harvest event
                                let harvest_event = VisibleEvent::HarvestEvent {
                                    structure_id: villager_attrs.structure,
                                };

                                // Add State Change Event to Moving
                                let state_change_event = VisibleEvent::StateChangeEvent {
                                    new_state: "harvesting".to_string(),
                                };

                                *villager_state = State::Harvesting;

                                map_events.new(villager.id.0, game_tick.0 + 4, state_change_event);

                                let map_event = map_events.new(
                                    villager.id.0,
                                    game_tick.0 + 50, // in the future
                                    harvest_event,
                                );

                                commands.entity(*actor).insert(EventInProgress {
                                    event_id: map_event.event_id,
                                });

                                *state = ActionState::Success;
                            }
                        }
                    }
                    Order::Explore => {
                        if *villager_state == State::None {
                            let explore_event = VisibleEvent::ExploreEvent;

                            map_events.new(
                                villager.id.0,
                                game_tick.0 + 8, // in the future
                                explore_event,
                            );
                        }
                    }
                    _ => {}
                }
            }
            ActionState::Cancelled => {
                debug!("Process Order Cancelled");
                // Cannot cancel an move
                *state = ActionState::Failure;
            }
            _ => {}
        }
    }
}

pub fn flee_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    map: Res<Map>,
    mut ids: ResMut<Ids>,
    mut map_events: ResMut<MapEvents>,
    active_infos: Res<ActiveInfos>,
    clients: Res<Clients>,
    events_in_progress: Query<&EventInProgress>,
    hero_query: Query<BaseQuery, (With<SubclassHero>, Without<SubclassVillager>)>,
    villager_query: Query<BaseQuery, With<SubclassVillager>>,
    blocking_query: Query<BaseQuery>,
    mut attrs_query: Query<&mut VillagerAttrs>,
    mut action_query: Query<(&Actor, &mut ActionState, &Flee, &ActionSpan)>,
) {
    for (Actor(actor), mut state, _find_drink, _span) in &mut action_query {
        match *state {
            ActionState::Requested => {
                debug!("Flee");
                let Ok(villager) = villager_query.get(*actor) else {
                    error!("Cannot get villager {:?}", actor);
                    continue;
                };

                let Ok(mut villager_attrs) = attrs_query.get_mut(*actor) else {
                    error!("Cannot get villager attrs {:?}", actor);
                    continue;
                };

                Obj::add_sound_obj_event(
                    game_tick.0,
                    "Run for your lives!".to_owned(),
                    villager.id,
                    &mut map_events,
                );

                // Set activity to drinking
                villager_attrs.activity = villager::Activity::Fleeing;

                // Check if player has an active info for this mover
                let active_info_key = (villager.player_id.0, villager.id.0, "obj".to_string());
                debug!(
                    "Active Info Key: {:?} Active Infos: {:?}",
                    active_info_key, active_infos
                );

                if let Some(_active_info) = active_infos.get(&active_info_key) {
                    let response_packet = ResponsePacket::InfoActivityUpdate {
                        id: villager.id.0,
                        activity: villager_attrs.activity.to_string(),
                    };

                    info!("Sending info activity update: {:?}", response_packet);
                    for (_client_id, client) in clients.lock().unwrap().iter() {
                        if client.player_id == villager.player_id.0 {
                            client
                                .sender
                                .try_send(serde_json::to_string(&response_packet).unwrap())
                                .expect("Could not send message");
                        }
                    }
                }

                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                if let Ok(_event) = events_in_progress.get(*actor) {
                    debug!("Flee Event In Progress...");
                } else {
                    let Ok(villager) = villager_query.get(*actor) else {
                        error!("Cannot find villager {:?}", *actor);
                        continue;
                    };

                    let Some(hero_id) = ids.get_hero(villager.player_id.0) else {
                        error!("Cannot find hero for player {:?}", villager.player_id);
                        continue;
                    };

                    let Some(hero_entity) = ids.get_entity(hero_id) else {
                        error!("Cannot find hero entity for hero {:?}", hero_id);
                        continue;
                    };

                    let Ok(hero) = hero_query.get(hero_entity) else {
                        error!("Cannot find hero for {:?}", hero_entity);
                        continue;
                    };

                    if hero.pos != villager.pos {
                        let mut blocking_list = Vec::new();

                        for blocking_obj in blocking_query.iter() {
                            if *blocking_obj.state != State::Dead {
                                if blocking_obj.player_id.0 != villager.player_id.0 {
                                    let map_pos = MapPos(blocking_obj.pos.x, blocking_obj.pos.y);
                                    blocking_list.push(map_pos);
                                }
                            }
                        }

                        if let Some(path_result) = Map::find_path(
                            *villager.pos,
                            *hero.pos,
                            &map,
                            blocking_list,
                            true,
                            false,
                            false,
                            false,
                        ) {
                            debug!("Path to structure: {:?}", path_result);

                            let (path, _c) = path_result;
                            let next_pos = &path[1];

                            debug!("Next pos: {:?}", next_pos);

                            // Add State Change Event to Moving
                            let state_change_event = VisibleEvent::StateChangeEvent {
                                new_state: "moving".to_string(),
                            };

                            map_events.new(villager.id.0, game_tick.0 + 1, state_change_event);

                            // Add Move Event
                            let move_event = VisibleEvent::MoveEvent {
                                src: *villager.pos,
                                dst: Position {
                                    x: next_pos.0,
                                    y: next_pos.1,
                                },
                            };

                            let event_id = ids.new_map_event_id();

                            let map_event = map_events.new(
                                villager.id.0,
                                game_tick.0 + 48, // in the future
                                move_event,
                            );

                            debug!("MoveToHero - Adding EventInProgress {:?}", event_id);
                            commands.entity(*actor).insert(EventInProgress {
                                event_id: map_event.event_id,
                            });

                            commands.entity(*actor).insert(MoveToInProgress);
                        } else {
                            //TODO randomly pick a flee location
                        }
                    } else {
                        debug!("Villager has arrived at hero");
                        commands.entity(*actor).remove::<MoveToInProgress>();
                        *state = ActionState::Success;
                    }
                }
            }
            ActionState::Cancelled => {
                debug!("Flee cancelled...");
                *state = ActionState::Failure;
            }
            _ => {}
        }
    }
}

pub fn find_drink_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    mut map_events: ResMut<MapEvents>,
    map: Res<Map>,
    ids: ResMut<Ids>,
    items: ResMut<Items>,
    active_infos: Res<ActiveInfos>,
    clients: Res<Clients>,
    templates: Res<Templates>,
    mut villager_query: Query<VillagerQuery, With<SubclassVillager>>,
    structure_query: Query<ObjQuery, (With<ClassStructure>, Without<SubclassVillager>)>,
    mut action_query: Query<(&Actor, &mut ActionState, &FindDrink, &ActionSpan)>,
) {
    for (Actor(actor), mut state, _find_drink, span) in &mut action_query {
        let _guard = span.span().enter();

        match *state {
            ActionState::Requested => {
                debug!("Find Drink Item");
                let Ok(mut villager) = villager_query.get_mut(*actor) else {
                    error!("Cannot get villager {:?}", actor);
                    continue;
                };

                Obj::add_sound_obj_event(
                    game_tick.0,
                    templates.get_dialogue("GettingDrink"),
                    villager.id,
                    &mut map_events,
                );

                // Set activity to drinking
                villager.attrs.activity = villager::Activity::GettingDrink;

                // Check if player has an active info for this mover
                let active_info_key = (villager.player_id.0, villager.id.0, "obj".to_string());
                debug!(
                    "Active Info Key: {:?} Active Infos: {:?}",
                    active_info_key, active_infos
                );

                if let Some(_active_info) = active_infos.get(&active_info_key) {
                    let response_packet = ResponsePacket::InfoActivityUpdate {
                        id: villager.id.0,
                        activity: villager.attrs.activity.to_string(),
                    };

                    info!("Sending info activity update: {:?}", response_packet);
                    for (_client_id, client) in clients.lock().unwrap().iter() {
                        if client.player_id == villager.player_id.0 {
                            client
                                .sender
                                .try_send(serde_json::to_string(&response_packet).unwrap())
                                .expect("Could not send message");
                        }
                    }
                }

                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                let Ok(villager) = villager_query.get_mut(*actor) else {
                    error!("Cannot get villager {:?}", actor);
                    continue;
                };

                let Some((item_location, item)) = find_item_location_by_class(
                    &villager,
                    &structure_query,
                    item::WATER.to_string(),
                    &items,
                    &map,
                ) else {
                    debug!("Cannot find any drinks. ");
                    commands.entity(*actor).insert(NoDrinks);

                    *state = ActionState::Failure;
                    continue;
                };

                if item_location == ItemLocation::OwnStructure {
                    let Some(entity) = ids.get_entity(item.owner) else {
                        error!("Cannot find entity for {:?}", item.owner);
                        continue;
                    };

                    let Ok(structure) = structure_query.get(entity) else {
                        error!("Cannot get structure from entity {:?}", entity);
                        continue;
                    };

                    commands.entity(*actor).insert(MoveToDrink {
                        dest: *structure.pos,
                    });
                } else if item_location == ItemLocation::Own {
                    commands.entity(*actor).insert(MoveToDrink {
                        dest: *villager.pos,
                    });
                }

                *state = ActionState::Success;
            }
            ActionState::Cancelled => {
                debug!("Cancelling find drink");
                // Reset activity
                if let Ok(mut villager) = villager_query.get_mut(*actor) {
                    villager.attrs.activity = villager::Activity::None;
                }

                remove_components(&mut commands, &*actor);

                *state = ActionState::Failure
            }
            _ => {}
        }
    }
}

pub fn move_to_water_source_action_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    mut ids: ResMut<Ids>,
    map: Res<Map>,
    mut map_events: ResMut<MapEvents>,
    mut game_events: ResMut<GameEvents>,
    move_to_drink: Query<&MoveToDrink>,
    events_in_progress: Query<&EventInProgress>,
    obj_query: Query<(&Id, &PlayerId, &Position)>,
    mut state_query: Query<&mut State>,
    mut attrs_query: Query<&mut VillagerAttrs>,
    mut action_query: Query<(&Actor, &mut ActionState, &MoveToWaterSource, &ActionSpan)>,
) {
    for (Actor(actor), mut state, _move_to, span) in &mut action_query {
        let _guard = span.span().enter();

        match *state {
            ActionState::Requested => {
                debug!("Let's go find some water!");
                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                let Some(villager_player_id) = ids.get_player_by_entity(*actor) else {
                    error!("Cannot find player id for entity {:?}", *actor);
                    *state = ActionState::Failure;
                    continue;
                };

                let blocking_list =
                    Obj::blocking_list(villager_player_id, actor, &obj_query, &state_query);

                if let Ok((id, player_id, pos)) = obj_query.get(*actor) {
                    if let Ok(_event) = events_in_progress.get(*actor) {
                        debug!("Move to water source still executing...");
                    } else {
                        let Ok(move_to_drink) = move_to_drink.get(*actor) else {
                            error!("Entity {:?} does not have MoveToDrink", *actor);
                            *state = ActionState::Failure;
                            continue;
                        };

                        let Ok(mut villager_state) = state_query.get_mut(*actor) else {
                            error!("Cannot get villager {:?}", actor);
                            *state = ActionState::Failure;
                            continue;
                        };

                        // Check if villager is on structure
                        if !Map::is_adjacent(*pos, Position { x: 16, y: 37 }) {
                            if let Some(path_result) = Map::find_path(
                                *pos,
                                move_to_drink.dest,
                                &map,
                                blocking_list,
                                true,
                                false,
                                false,
                                false,
                            ) {
                                debug!("Path to structure: {:?}", path_result);

                                let (path, _c) = path_result;
                                let next_pos = &path[1];

                                debug!("Next pos: {:?}", next_pos);

                                // Add State Change Event to Moving
                                let state_change_event = VisibleEvent::StateChangeEvent {
                                    new_state: "moving".to_string(),
                                };

                                //*villager_state = State::Moving;

                                map_events.new(id.0, game_tick.0 + 1, state_change_event);

                                // Add Move Event
                                let move_event = VisibleEvent::MoveEvent {
                                    src: *pos,
                                    dst: Position {
                                        x: next_pos.0,
                                        y: next_pos.1,
                                    },
                                };

                                let map_event = map_events.new(
                                    id.0,
                                    game_tick.0 + 48, // in the future
                                    move_event,
                                );

                                commands.entity(*actor).insert(EventInProgress {
                                    event_id: map_event.event_id,
                                });

                                commands.entity(*actor).insert(MoveToInProgress);
                            } else {
                                debug!("Cannot find path to drink");
                                *state = ActionState::Failure
                            }
                        } else {
                            debug!("Villager is adjacent to drink source");
                            commands.entity(*actor).remove::<MoveToInProgress>();
                            *state = ActionState::Success;
                        }
                    }
                }
            }
            ActionState::Cancelled => {
                // Reset activity
                if let Ok(mut attrs) = attrs_query.get_mut(*actor) {
                    attrs.activity = villager::Activity::None;
                }

                debug!("Cancelling MoveToWaterSource action");

                if let Ok(event) = events_in_progress.get(*actor) {
                    debug!(
                        "Event still executing, canceling event {:?}",
                        event.event_id
                    );

                    let event_type = GameEventType::CancelEvents {
                        event_ids: vec![event.event_id],
                    };
                    let event_id = ids.new_map_event_id();

                    let event = GameEvent {
                        event_id: event_id,
                        run_tick: game_tick.0 + 1, // Add one game tick
                        game_event_type: event_type,
                    };

                    game_events.insert(event.event_id, event);
                }

                remove_components(&mut commands, &*actor);

                *state = ActionState::Failure;
            }
            _ => {}
        }
    }
}

pub fn transfer_drink_system(
    map: Res<Map>,
    _ids: ResMut<Ids>,
    mut items: ResMut<Items>,
    _templates: Res<Templates>,
    structure_query: Query<ObjQuery, (With<ClassStructure>, Without<SubclassVillager>)>,
    mut villager_query: Query<VillagerQuery, With<SubclassVillager>>,
    mut action_query: Query<(&Actor, &mut ActionState, &TransferDrink, &ActionSpan)>,
) {
    for (Actor(actor), mut state, _transfer_drink, span) in &mut action_query {
        let _guard = span.span().enter();

        match *state {
            ActionState::Requested => {
                debug!("Transfer Drink Item");
                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                let Ok(villager) = villager_query.get_mut(*actor) else {
                    debug!("Cannot get villager {:?}", actor);
                    *state = ActionState::Failure;
                    continue;
                };

                let Some((_item_location, item)) = find_item_location_by_class(
                    &villager,
                    &structure_query,
                    item::WATER.to_string(),
                    &items,
                    &map,
                ) else {
                    error!("Cannot find any drinks. ");
                    *state = ActionState::Failure;
                    continue;
                };

                items.transfer_quantity(item.id, villager.id.0, 1);

                *state = ActionState::Success;
            }
            ActionState::Cancelled => {
                debug!("Cancelling transfer drink");
                // Reset activity
                if let Ok(mut villager) = villager_query.get_mut(*actor) {
                    villager.attrs.activity = villager::Activity::None;
                }

                *state = ActionState::Failure
            }
            _ => {}
        }
    }
}

pub fn drink_action_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    mut ids: ResMut<Ids>,
    mut map_events: ResMut<MapEvents>,
    mut game_events: ResMut<GameEvents>,
    mut items: ResMut<Items>,
    mut thirsts: Query<&mut Thirst>,
    events_in_progress: Query<&EventInProgress>,
    drink_events_completed: Query<&DrinkEventCompleted>,
    mut villager_query: Query<VillagerQuery, With<SubclassVillager>>,
    mut query: Query<(&Actor, &mut ActionState, &Drink, &ActionSpan)>,
) {
    for (Actor(actor), mut state, _drink, span) in &mut query {
        let _guard = span.span().enter();

        // Use the drink_action's actor to look up the corresponding Thirst Component.
        if let Ok(mut thirst) = thirsts.get_mut(*actor) {
            match *state {
                ActionState::Requested => {
                    debug!("drink action entity: {:?}", *actor);

                    let Ok(mut villager) = villager_query.get_mut(*actor) else {
                        debug!("Cannot get villager {:?}", actor);
                        *state = ActionState::Failure;
                        continue;
                    };

                    let Some(drink_item) =
                        items.get_by_class(villager.id.0, item::WATER.to_owned())
                    else {
                        debug!("Cannot find drink item on {:?}", villager.id.0);
                        *state = ActionState::Failure;
                        continue;
                    };
                    // Create drinking event
                    let drink_event = VisibleEvent::DrinkEvent {
                        item_id: drink_item.id,
                        obj_id: villager.id.0,
                    };

                    // Add State Change Event to drinking
                    let state_change_event = VisibleEvent::StateChangeEvent {
                        new_state: obj::STATE_DRINKING.to_string(),
                    };

                    *villager.state = State::Drinking;

                    map_events.new(villager.id.0, game_tick.0 + 1, state_change_event);

                    let map_event = map_events.new(
                        villager.id.0,
                        game_tick.0 + 24, // in the future
                        drink_event,
                    );

                    commands.entity(*actor).insert(EventInProgress {
                        event_id: map_event.event_id,
                    });

                    *state = ActionState::Executing;
                }
                ActionState::Executing => {
                    if let Ok(drink_event_completed) = drink_events_completed.get(*actor) {
                        debug!("Drink Event completed, getting item thirst mod...");

                        if let Ok(mut villager) = villager_query.get_mut(*actor) {
                            // Reset activity
                            villager.attrs.activity = villager::Activity::None;
                        }

                        // Remove Drink Event Complete
                        commands.entity(*actor).remove::<DrinkEventCompleted>();

                        let Some(thirst_mod) =
                            drink_event_completed.item.attrs.get(&item::AttrKey::Thirst)
                        else {
                            debug!(
                                "Missing thirst mod on item: {:?}",
                                drink_event_completed.item
                            );
                            *state = ActionState::Failure;
                            continue;
                        };

                        let thirst_mod_val = match thirst_mod {
                            item::AttrVal::Num(val) => val,
                            _ => panic!("Incorrect attribute value {:?}", thirst_mod),
                        };

                        // Update thirst
                        thirst.add(-1.0 * *thirst_mod_val);

                        if thirst.thirst <= 80.0 {
                            commands.entity(*actor).remove::<Dehydrated>();
                        }

                        // Update item count
                        items.update_quantity_by_class(
                            drink_event_completed.item.owner,
                            item::WATER.to_string(),
                            -1,
                        );

                        *state = ActionState::Success;
                    } else {
                        debug!("Drink Event still executing, waiting for completed component");
                    }
                }
                // All Actions should make sure to handle cancellations!
                ActionState::Cancelled => {
                    // Reset activity
                    if let Ok(mut villager) = villager_query.get_mut(*actor) {
                        villager.attrs.activity = villager::Activity::None;
                    }

                    debug!("Cancelling Drink action");
                    if let Ok(event) = events_in_progress.get(*actor) {
                        debug!(
                            "Event still executing, canceling event {:?}",
                            event.event_id
                        );

                        let event_type = GameEventType::CancelEvents {
                            event_ids: vec![event.event_id],
                        };
                        let event_id = ids.new_map_event_id();

                        let event = GameEvent {
                            event_id: event_id,
                            run_tick: game_tick.0 + 1, // Add one game tick
                            game_event_type: event_type,
                        };

                        game_events.insert(event.event_id, event);
                    }

                    remove_components(&mut commands, &*actor);

                    *state = ActionState::Failure;
                }
                _ => {}
            }
        }
    }
}

pub fn find_food_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    mut map_events: ResMut<MapEvents>,
    map: Res<Map>,
    ids: ResMut<Ids>,
    items: ResMut<Items>,
    active_infos: Res<ActiveInfos>,
    clients: Res<Clients>,
    templates: Res<Templates>,
    mut villager_query: Query<VillagerQuery, With<SubclassVillager>>,
    structure_query: Query<ObjQuery, (With<ClassStructure>, Without<SubclassVillager>)>,
    mut action_query: Query<(&Actor, &mut ActionState, &FindFood, &ActionSpan)>,
) {
    for (Actor(actor), mut state, _find_food, span) in &mut action_query {
        let _guard = span.span().enter();

        match *state {
            ActionState::Requested => {
                debug!("Find Food Item");
                let Ok(mut villager) = villager_query.get_mut(*actor) else {
                    error!("Cannot get villager {:?}", actor);
                    continue;
                };

                Obj::add_sound_obj_event(
                    game_tick.0,
                    templates.get_dialogue("GettingFood"),
                    villager.id,
                    &mut map_events,
                );

                // Set activity to drinking
                villager.attrs.activity = villager::Activity::GettingFood;

                // Check if player has an active info for this mover
                let active_info_key = (villager.player_id.0, villager.id.0, "obj".to_string());
                debug!(
                    "Active Info Key: {:?} Active Infos: {:?}",
                    active_info_key, active_infos
                );

                if let Some(_active_info) = active_infos.get(&active_info_key) {
                    let response_packet = ResponsePacket::InfoActivityUpdate {
                        id: villager.id.0,
                        activity: villager.attrs.activity.to_string(),
                    };

                    info!("Sending info activity update: {:?}", response_packet);
                    for (_client_id, client) in clients.lock().unwrap().iter() {
                        if client.player_id == villager.player_id.0 {
                            client
                                .sender
                                .try_send(serde_json::to_string(&response_packet).unwrap())
                                .expect("Could not send message");
                        }
                    }
                }

                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                let Ok(mut villager) = villager_query.get_mut(*actor) else {
                    debug!("Cannot get villager {:?}", actor);
                    *state = ActionState::Failure;
                    continue;
                };

                let Some((item_location, item)) = find_item_location_by_class(
                    &villager,
                    &structure_query,
                    item::FOOD.to_string(),
                    &items,
                    &map,
                ) else {
                    error!("Cannot find any food. ");
                    *state = ActionState::Failure;
                    continue;
                };

                debug!("item_location: {:?}", item_location);
                if item_location == ItemLocation::OwnStructure {
                    let Some(entity) = ids.get_entity(item.owner) else {
                        error!("Cannot find entity for {:?}", item.owner);
                        *state = ActionState::Failure;
                        continue;
                    };

                    let Ok(structure) = structure_query.get(entity) else {
                        error!("Cannot get structure from entity {:?}", entity);
                        *state = ActionState::Failure;
                        continue;
                    };

                    commands.entity(*actor).insert(MoveToFood {
                        dest: *structure.pos,
                    });
                }

                *state = ActionState::Success;
            }
            ActionState::Cancelled => {
                debug!("Cancelling transfer food");
                // Reset activity
                if let Ok(mut villager) = villager_query.get_mut(*actor) {
                    villager.attrs.activity = villager::Activity::None;
                }

                remove_components(&mut commands, &*actor);

                *state = ActionState::Failure
            }
            _ => {}
        }
    }
}

pub fn move_to_food_action_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    mut ids: ResMut<Ids>,
    map: Res<Map>,
    move_to_food: Query<&MoveToFood>,
    mut map_events: ResMut<MapEvents>,
    mut game_events: ResMut<GameEvents>,
    events_in_progress: Query<&EventInProgress>,
    mut villager_query: Query<VillagerQuery, With<SubclassVillager>>,
    mut action_query: Query<(&Actor, &mut ActionState, &MoveToFoodSource, &ActionSpan)>,
) {
    // Loop through all actions, just like you'd loop over all entities in any other query.
    for (Actor(actor), mut state, _move_to, span) in &mut action_query {
        let _guard = span.span().enter();

        // Different behavior depending on action state.
        match *state {
            // Action was just requested; it hasn't been seen before.
            ActionState::Requested => {
                debug!("Let's go find some food!");
                // We don't really need any initialization code here, since the queries are cheap enough.
                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                if let Ok(mut villager) = villager_query.get_mut(*actor) {
                    if let Ok(_event) = events_in_progress.get(*actor) {
                        debug!("Move to food source still executing...");
                    } else {
                        let Ok(move_to_food) = move_to_food.get(*actor) else {
                            error!("Entity {:?} does not have MoveToFood", *actor);
                            continue;
                        };

                        // Check if villager is on structure
                        if !Map::is_adjacent(*villager.pos, Position { x: 16, y: 37 }) {
                            if let Some(path_result) = Map::find_path(
                                *villager.pos,
                                move_to_food.dest,
                                &map,
                                Vec::new(),
                                true,
                                false,
                                false,
                                false,
                            ) {
                                let (path, _c) = path_result;
                                let next_pos = &path[1];

                                // Add State Change Event to Moving
                                let state_change_event = VisibleEvent::StateChangeEvent {
                                    new_state: "moving".to_string(),
                                };

                                *villager.state = State::Moving;

                                map_events.new(villager.id.0, game_tick.0 + 1, state_change_event);

                                // Add Move Event
                                let move_event = VisibleEvent::MoveEvent {
                                    src: *villager.pos,
                                    dst: Position {
                                        x: next_pos.0,
                                        y: next_pos.1,
                                    },
                                };

                                let _event_id = ids.new_map_event_id();

                                let map_event = map_events.new(
                                    villager.id.0,
                                    game_tick.0 + 48, // in the future
                                    move_event,
                                );

                                commands.entity(*actor).insert(EventInProgress {
                                    event_id: map_event.event_id,
                                });

                                commands.entity(*actor).insert(MoveToInProgress);
                            } else {
                                debug!("Cannot find path to food");
                                *state = ActionState::Failure
                            }
                        } else {
                            debug!("Villager is adjacent to food source");
                            commands.entity(*actor).remove::<MoveToInProgress>();
                            *state = ActionState::Success;
                        }
                    }
                }
            }
            ActionState::Cancelled => {
                debug!("Cancelling MoveToFoodSource action");
                // Reset activity
                if let Ok(mut villager) = villager_query.get_mut(*actor) {
                    villager.attrs.activity = villager::Activity::None;
                }

                if let Ok(event) = events_in_progress.get(*actor) {
                    debug!(
                        "Event still executing, canceling event {:?}",
                        event.event_id
                    );

                    let event_type = GameEventType::CancelEvents {
                        event_ids: vec![event.event_id],
                    };
                    let event_id = ids.new_map_event_id();

                    let event = GameEvent {
                        event_id: event_id,
                        run_tick: game_tick.0 + 1, // Add one game tick
                        game_event_type: event_type,
                    };

                    game_events.insert(event.event_id, event);
                }

                remove_components(&mut commands, &*actor);

                *state = ActionState::Failure;
            }
            _ => {}
        }
    }
}

pub fn transfer_food_system(
    map: Res<Map>,
    _ids: ResMut<Ids>,
    mut items: ResMut<Items>,
    _templates: Res<Templates>,
    structure_query: Query<ObjQuery, (With<ClassStructure>, Without<SubclassVillager>)>,
    mut villager_query: Query<VillagerQuery, With<SubclassVillager>>,
    mut action_query: Query<(&Actor, &mut ActionState, &TransferFood, &ActionSpan)>,
) {
    for (Actor(actor), mut state, _transfer_food, span) in &mut action_query {
        let _guard = span.span().enter();

        match *state {
            ActionState::Requested => {
                debug!("Transfer Food Item");
                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                let Ok(mut villager) = villager_query.get_mut(*actor) else {
                    debug!("Cannot get villager {:?}", actor);
                    *state = ActionState::Failure;
                    continue;
                };

                let Some((_item_location, item)) = find_item_location_by_class(
                    &villager,
                    &structure_query,
                    item::FOOD.to_string(),
                    &items,
                    &map,
                ) else {
                    error!("Cannot find any food. ");

                    villager.attrs.activity = villager::Activity::None;
                    *state = ActionState::Failure;
                    continue;
                };

                items.transfer_quantity(item.id, villager.id.0, 1);

                *state = ActionState::Success;
            }
            ActionState::Cancelled => {
                debug!("Cancelling transfer food");
                // Reset activity
                if let Ok(mut villager) = villager_query.get_mut(*actor) {
                    villager.attrs.activity = villager::Activity::None;
                }

                *state = ActionState::Failure
            }
            _ => {}
        }
    }
}

pub fn eat_action_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    mut ids: ResMut<Ids>,
    mut map_events: ResMut<MapEvents>,
    mut game_events: ResMut<GameEvents>,
    mut items: ResMut<Items>,
    mut hungers: Query<&mut Hunger>,
    events_in_progress: Query<&EventInProgress>,
    eat_events_completed: Query<&EatEventCompleted>,
    mut villager_query: Query<VillagerQuery, With<SubclassVillager>>,
    mut query: Query<(&Actor, &mut ActionState, &Eat, &ActionSpan)>,
) {
    for (Actor(actor), mut state, _eat, span) in &mut query {
        let _guard = span.span().enter();

        // Use the drink_action's actor to look up the corresponding Thirst Component.
        if let Ok(mut hunger) = hungers.get_mut(*actor) {
            match *state {
                ActionState::Requested => {
                    debug!("Hunger action entity: {:?}", *actor);

                    let Ok(mut villager) = villager_query.get_mut(*actor) else {
                        debug!("Cannot find villager {:?}", actor);
                        *state = ActionState::Failure;
                        continue;
                    };

                    let Some(food_item) = items.get_by_class(villager.id.0, item::FOOD.to_owned())
                    else {
                        debug!("Cannot find food item on {:?}", villager.id.0);
                        *state = ActionState::Failure;
                        continue;
                    };
                    // Create drinking event
                    let eat_event = VisibleEvent::EatEvent {
                        item_id: food_item.id,
                        obj_id: villager.id.0,
                    };

                    // Add State Change Event to drinking
                    let state_change_event = VisibleEvent::StateChangeEvent {
                        new_state: obj::STATE_EATING.to_string(),
                    };

                    *villager.state = State::Eating;

                    map_events.new(villager.id.0, game_tick.0 + 1, state_change_event);

                    let map_event = map_events.new(
                        villager.id.0,
                        game_tick.0 + 24, // in the future
                        eat_event,
                    );

                    commands.entity(*actor).insert(EventInProgress {
                        event_id: map_event.event_id,
                    });

                    *state = ActionState::Executing;
                }
                ActionState::Executing => {
                    if let Ok(eat_event_completed) = eat_events_completed.get(*actor) {
                        debug!("Eat Event completed, getting item feed mod...");

                        // Reset activity
                        if let Ok(mut villager) = villager_query.get_mut(*actor) {
                            villager.attrs.activity = villager::Activity::None;
                        }

                        // Remove Eat Event Complete
                        commands.entity(*actor).remove::<EatEventCompleted>();

                        let Some(feed_mod) =
                            eat_event_completed.item.attrs.get(&item::AttrKey::Feed)
                        else {
                            debug!("Missing feed mod on item: {:?}", eat_event_completed.item);
                            *state = ActionState::Failure;
                            continue;
                        };

                        let feed_mod_val = match feed_mod {
                            item::AttrVal::Num(val) => val,
                            _ => panic!("Incorrect attribute value {:?}", feed_mod),
                        };

                        // Update hunger
                        hunger.update(-1.0 * *feed_mod_val);

                        if hunger.hunger <= 80.0 {
                            commands.entity(*actor).remove::<Starving>();
                        }

                        // Update item count
                        items.update_quantity_by_class(
                            eat_event_completed.item.owner,
                            item::FOOD.to_string(),
                            -1,
                        );

                        *state = ActionState::Success;
                    } else {
                        debug!("Still waiting for Eat Event to complete...");
                    }
                }
                // All Actions should make sure to handle cancellations!
                ActionState::Cancelled => {
                    // Reset activity
                    if let Ok(mut villager) = villager_query.get_mut(*actor) {
                        villager.attrs.activity = villager::Activity::None;
                    }

                    debug!("Cancelling Eat action");
                    if let Ok(event) = events_in_progress.get(*actor) {
                        debug!(
                            "Event still executing, canceling event {:?}",
                            event.event_id
                        );

                        let event_type = GameEventType::CancelEvents {
                            event_ids: vec![event.event_id],
                        };
                        let event_id = ids.new_map_event_id();

                        let event = GameEvent {
                            event_id: event_id,
                            run_tick: game_tick.0 + 1, // Add one game tick
                            game_event_type: event_type,
                        };

                        game_events.insert(event.event_id, event);
                    }
                    *state = ActionState::Failure;
                }
                _ => {}
            }
        }
    }
}

pub fn find_shelter_system(
    mut commands: Commands,
    map: Res<Map>,
    game_tick: Res<GameTick>,
    active_infos: Res<ActiveInfos>,
    clients: Res<Clients>,
    mut map_events: ResMut<MapEvents>,
    templates: Res<Templates>,
    mut villager_query: Query<VillagerQuery, With<SubclassVillager>>,
    structure_query: Query<ObjQuery, (With<ClassStructure>, Without<SubclassVillager>)>,
    mut action_query: Query<(&Actor, &mut ActionState, &FindShelter, &ActionSpan)>,
) {
    for (Actor(actor), mut state, _find_shelter, span) in &mut action_query {
        let _guard = span.span().enter();

        match *state {
            ActionState::Requested => {
                debug!("Find Shelter");
                let Ok(mut villager) = villager_query.get_mut(*actor) else {
                    debug!("Cannot get villager {:?}", actor);
                    continue;
                };

                Obj::add_sound_obj_event(
                    game_tick.0,
                    templates.get_dialogue("FindingShelter"),
                    villager.id,
                    &mut map_events,
                );

                // Set activity to drinking
                villager.attrs.activity = villager::Activity::FindingShelter;

                // Check if player has an active info for this mover
                let active_info_key = (villager.player_id.0, villager.id.0, "obj".to_string());
                debug!(
                    "Active Info Key: {:?} Active Infos: {:?}",
                    active_info_key, active_infos
                );

                if let Some(_active_info) = active_infos.get(&active_info_key) {
                    let response_packet = ResponsePacket::InfoActivityUpdate {
                        id: villager.id.0,
                        activity: villager.attrs.activity.to_string(),
                    };

                    info!("Sending info activity update: {:?}", response_packet);
                    for (_client_id, client) in clients.lock().unwrap().iter() {
                        if client.player_id == villager.player_id.0 {
                            client
                                .sender
                                .try_send(serde_json::to_string(&response_packet).unwrap())
                                .expect("Could not send message");
                        }
                    }
                }

                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                let Ok(mut villager) = villager_query.get_mut(*actor) else {
                    debug!("Cannot get villager {:?}", actor);
                    *state = ActionState::Failure;
                    continue;
                };

                if let (Some(structure_pos), Some(_path)) =
                    find_shelter(&villager, &structure_query, &map)
                {
                    commands.entity(*actor).insert(MoveToShelter {
                        dest: structure_pos,
                    });
                    debug!("Found shelter, moving to shelter");

                    *state = ActionState::Success;
                } else {
                    debug!(
                        "{:?} cannot find shelter, setting current location as shelter",
                        *actor
                    );
                    commands.entity(*actor).insert(MoveToShelter {
                        dest: *villager.pos,
                    });

                    *state = ActionState::Success;
                }
            }
            ActionState::Cancelled => {
                debug!("Cancelling transfer drink");

                // Reset activity
                if let Ok(mut villager) = villager_query.get_mut(*actor) {
                    villager.attrs.activity = villager::Activity::None;
                }

                *state = ActionState::Failure
            }
            _ => {}
        }
    }
}

pub fn move_to_shelter_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    mut ids: ResMut<Ids>,
    map: Res<Map>,
    move_to_shelter: Query<&MoveToShelter>,
    mut map_events: ResMut<MapEvents>,
    mut game_events: ResMut<GameEvents>,
    events_in_progress: Query<&EventInProgress>,
    obj_query: Query<(&Id, &PlayerId, &Position)>,
    mut state_query: Query<&mut State>,
    mut attrs_query: Query<&mut VillagerAttrs>,
    mut action_query: Query<(&Actor, &mut ActionState, &ActionSpan), With<MoveToSleepPos>>,
) {
    for (Actor(actor), mut state, span) in &mut action_query {
        let _guard = span.span().enter();

        match *state {
            ActionState::Requested => {
                debug!("Let's go find some shelter!");
                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                let Some(villager_player_id) = ids.get_player_by_entity(*actor) else {
                    error!("Cannot find player id for entity {:?}", *actor);
                    *state = ActionState::Failure;
                    continue;
                };

                let blocking_list =
                    Obj::blocking_list(villager_player_id, actor, &obj_query, &state_query);

                if let Ok((id, player_id, pos)) = obj_query.get(*actor) {
                    if let Ok(_event) = events_in_progress.get(*actor) {
                        debug!("Move to shelter still executing...");
                    } else {
                        let Ok(move_to_shelter) = move_to_shelter.get(*actor) else {
                            error!("Entity {:?} does not have MoveToSleepPos", *actor);
                            *state = ActionState::Failure;
                            continue;
                        };

                        let Ok(mut villager_state) = state_query.get_mut(*actor) else {
                            error!("Cannot get villager {:?}", actor);
                            *state = ActionState::Failure;
                            continue;
                        };

                        // Check if villager is on structure
                        if pos.x != move_to_shelter.dest.x && pos.y != move_to_shelter.dest.y {
                            if let Some(path_result) = Map::find_path(
                                *pos,
                                move_to_shelter.dest,
                                &map,
                                blocking_list,
                                true,
                                false,
                                false,
                                false,
                            ) {
                                debug!("Path to structure: {:?}", path_result);

                                let (path, _c) = path_result;
                                let next_pos = &path[1];

                                debug!("Next pos: {:?}", next_pos);

                                // Add State Change Event to Moving
                                let state_change_event = VisibleEvent::StateChangeEvent {
                                    new_state: "moving".to_string(),
                                };

                                //*villager_state = State::Moving;

                                map_events.new(id.0, game_tick.0 + 1, state_change_event);

                                // Add Move Event
                                let move_event = VisibleEvent::MoveEvent {
                                    src: *pos,
                                    dst: Position {
                                        x: next_pos.0,
                                        y: next_pos.1,
                                    },
                                };

                                let map_event = map_events.new(
                                    id.0,
                                    game_tick.0 + 48, // in the future
                                    move_event,
                                );

                                commands.entity(*actor).insert(EventInProgress {
                                    event_id: map_event.event_id,
                                });

                                commands.entity(*actor).insert(MoveToInProgress);
                            } else {
                                debug!("Cannot find path to shelter");
                                *state = ActionState::Failure
                            }
                        } else {
                            debug!("Villager is adjacent to shelter");
                            commands.entity(*actor).remove::<MoveToInProgress>();
                            *state = ActionState::Success;
                        }
                    }
                }
            }
            ActionState::Cancelled => {
                // Reset activity
                if let Ok(mut attrs) = attrs_query.get_mut(*actor) {
                    attrs.activity = villager::Activity::None;
                }
                debug!("Cancelling MoveToShelter action");

                if let Ok(event) = events_in_progress.get(*actor) {
                    debug!(
                        "Event still executing, canceling event {:?}",
                        event.event_id
                    );

                    let event_type = GameEventType::CancelEvents {
                        event_ids: vec![event.event_id],
                    };
                    let event_id = ids.new_map_event_id();

                    let event = GameEvent {
                        event_id: event_id,
                        run_tick: game_tick.0 + 1, // Add one game tick
                        game_event_type: event_type,
                    };

                    game_events.insert(event.event_id, event);
                }

                remove_components(&mut commands, &*actor);

                *state = ActionState::Failure;
            }
            _ => {}
        }
    }
}

pub fn sleep_action_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    mut ids: ResMut<Ids>,
    mut map_events: ResMut<MapEvents>,
    mut game_events: ResMut<GameEvents>,
    mut tired_query: Query<&mut Tired>,
    events_in_progress: Query<&EventInProgress>,
    sleep_events_completed: Query<&SleepEventCompleted>,
    mut villager_query: Query<VillagerQuery, With<SubclassVillager>>,
    mut query: Query<(&Actor, &mut ActionState, &Sleep, &ActionSpan)>,
) {
    for (Actor(actor), mut state, _sleep, span) in &mut query {
        let _guard = span.span().enter();

        if let Ok(mut tired) = tired_query.get_mut(*actor) {
            match *state {
                ActionState::Requested => {
                    debug!("Tired action entity: {:?}", *actor);

                    let Ok(mut villager) = villager_query.get_mut(*actor) else {
                        debug!("Cannot find villager {:?}", actor);
                        *state = ActionState::Failure;
                        continue;
                    };

                    // Create sleep event
                    let sleep_event = VisibleEvent::SleepEvent {
                        obj_id: villager.id.0,
                    };

                    // Add State Change Event to drinking
                    let state_change_event = VisibleEvent::StateChangeEvent {
                        new_state: obj::STATE_SLEEPING.to_string(),
                    };

                    *villager.state = State::Sleeping;

                    map_events.new(villager.id.0, game_tick.0 + 1, state_change_event);

                    let map_event = map_events.new(
                        villager.id.0,
                        game_tick.0 + 50, // in the future
                        sleep_event,
                    );

                    commands.entity(*actor).insert(EventInProgress {
                        event_id: map_event.event_id,
                    });

                    *state = ActionState::Executing;
                }
                ActionState::Executing => {
                    if let Ok(_sleep_event_completed) = sleep_events_completed.get(*actor) {
                        debug!("Sleep Event completed at game_tick {:?}", game_tick.0);

                        // Reset activity
                        if let Ok(mut villager) = villager_query.get_mut(*actor) {
                            villager.attrs.activity = villager::Activity::None;
                        }

                        // Remove Sleep Event Complete
                        commands.entity(*actor).remove::<SleepEventCompleted>();

                        // Update Tired, remove all tiredness
                        tired.update(-100.0);

                        if tired.tired <= 80.0 {
                            commands.entity(*actor).remove::<Exhausted>();
                        }

                        *state = ActionState::Success;
                    } else {
                        debug!("Still waiting for the sleep event to complete...");
                        /*tired.update(-1.0);

                        if tired.tired <= 80.0 {
                            commands.entity(*actor).remove::<Exhausted>();
                        }*/
                    }
                }
                // All Actions should make sure to handle cancellations!
                ActionState::Cancelled => {
                    // Reset activity
                    if let Ok(mut villager) = villager_query.get_mut(*actor) {
                        villager.attrs.activity = villager::Activity::None;
                    }

                    remove_components(&mut commands, &*actor);

                    debug!("Cancelling Sleep action");
                    if let Ok(event) = events_in_progress.get(*actor) {
                        debug!(
                            "Event still executing, canceling event {:?}",
                            event.event_id
                        );

                        let event_type = GameEventType::CancelEvents {
                            event_ids: vec![event.event_id],
                        };
                        let event_id = ids.new_map_event_id();

                        let event = GameEvent {
                            event_id: event_id,
                            run_tick: game_tick.0 + 1, // Add one game tick
                            game_event_type: event_type,
                        };

                        game_events.insert(event.event_id, event);
                    }
                    *state = ActionState::Failure;
                }
                _ => {}
            }
        }
    }
}

fn find_item_location_by_class(
    villager: &VillagerQueryItem,
    structure_query: &Query<ObjQuery, (With<ClassStructure>, Without<SubclassVillager>)>,
    item_class: String,
    items: &ResMut<Items>,
    map: &Res<Map>,
) -> Option<(item::ItemLocation, Item)> {
    //First check obj if they have any water on hand

    if let Some(item) = items.get_by_class(villager.id.0, item_class.clone()) {
        return Some((item::ItemLocation::Own, item.clone()));
    } else {
        let mut nearest_source_dist = 10000 as u32;
        let mut nearest_item = None;

        for structure in structure_query.iter() {
            // Skip if player_id of villager and structure are not matching
            if villager.player_id.0 != structure.player_id.0 {
                debug!("Villager and structure player_id are not matching");
                continue;
            }

            // Check if the structure has any water items
            let Some(item) = items.get_by_class(structure.id.0, item_class.clone()) else {
                debug!(
                    "Structure does not have any items of class {:?}",
                    item_class
                );
                continue;
            };

            let Some(path_result) = Map::find_path(
                *villager.pos,
                *structure.pos,
                &map,
                Vec::new(),
                true,
                false,
                false,
                false,
            ) else {
                debug!("Not path found to structure...");
                continue;
            };

            debug!("Path to structure: {:?}", path_result);

            let (_path, c) = path_result;
            debug!("Path count: {:?}", c);

            if nearest_source_dist > c {
                nearest_source_dist = c;
                nearest_item = Some(item);
            }
        }

        if let Some(nearest_item) = nearest_item {
            return Some((item::ItemLocation::OwnStructure, nearest_item.clone()));
        } else {
            return None;
        }
    }
}

fn find_shelter(
    villager: &VillagerQueryItem,
    structure_query: &Query<ObjQuery, (With<ClassStructure>, Without<SubclassVillager>)>,
    map: &Res<Map>,
) -> (Option<Position>, Option<Vec<MapPos>>) {
    let mut nearest_shelter_dist = 10000 as u32;
    let mut nearest_structure_pos = None;
    let mut nearest_path = None;

    for structure in structure_query.iter() {
        // Skip if player_id of villager and structure are not matching
        if villager.player_id.0 != structure.player_id.0 {
            debug!("Villager and structure player_id are not matching");
            continue;
        }

        // Check if the structure is a shelter
        if structure.subclass.0 != structure::SHELTER.to_string() {
            debug!("Structure is not a shelter");
            continue;
        }

        if *structure.state != State::None {
            debug!("Structure is not completed");
            continue;
        }

        let Some(path_result) = Map::find_path(
            *villager.pos,
            *structure.pos,
            &map,
            Vec::new(),
            true,
            false,
            false,
            false,
        ) else {
            debug!("No path found to structure...");
            continue;
        };

        debug!("Path to structure: {:?}", path_result);

        let (path, c) = path_result;

        if nearest_shelter_dist > c {
            nearest_shelter_dist = c;
            nearest_structure_pos = Some(*structure.pos);
            nearest_path = Some(path);
        }
    }

    return (nearest_structure_pos, nearest_path);
}

fn remove_components(commands: &mut Commands, entity: &Entity) {
    commands.entity(*entity).remove::<MoveToDrink>();
    commands.entity(*entity).remove::<MoveToFood>();
    commands.entity(*entity).remove::<MoveToShelter>();
    commands.entity(*entity).remove::<MoveToInProgress>();
}
