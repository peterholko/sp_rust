use bevy::ecs::query::WorldQuery;
use bevy::prelude::*;

use big_brain::prelude::*;
use pathfinding::prelude::directions::E;
use rand::Rng;

use crate::combat::Combat;
use crate::combat::CombatQuery;
use crate::game::is_none_state;
use crate::game::Class;
use crate::game::GameTick;
use crate::game::NPCAttrs;
use crate::game::Subclass;
use crate::game::SubclassHero;
use crate::game::SubclassNPC;
use crate::game::SubclassVillager;
use crate::game::Viewshed;
use crate::game::VillagerAttrs;
use crate::game::{
    EventInProgress, Id, Ids, MapEvents, Order, PlayerId, Position, State, VisibleEvent,
};
use crate::item::{Item, Items, THIRST, WATER};
use crate::map::Map;
use crate::templates::Templates;

pub const NO_TARGET: i32 = -1;

#[derive(Clone, Component, Debug)]
pub struct HighMorale;

#[derive(Clone, Component, Debug)]
pub struct ProcessOrder;

#[derive(Clone, Component, Debug)]
pub struct ChaseAttack;

#[derive(Clone, Component, Debug)]
pub struct Chase;

#[derive(Clone, Component, Debug)]
pub struct Thirsty;

#[derive(Clone, Component, Debug)]
pub struct VisibleTargetScorerBuilder;

#[derive(Component, Debug)]
pub struct DrinkingState {
    pub thirst_mod: f32,
    pub end_tick: i32,
}

#[derive(Clone, Component, Debug)]
pub struct Drink {
    pub until: f32,
    pub per_tick: f32,
}

#[derive(Component, Debug)]
pub struct Thirst {
    pub per_tick: f32,
    pub thirst: f32,
}

impl Thirst {
    pub fn new(thirst: f32, per_tick: f32) -> Self {
        Self { thirst, per_tick }
    }
}

#[derive(Component, Debug)]
pub struct Morale {
    pub per_tick: f32,
    pub morale: f32,
}

impl Morale {
    pub fn new(morale: f32, per_tick: f32) -> Self {
        Self { morale, per_tick }
    }
}

#[derive(Component, Debug)]
pub struct VisibleTarget {
    pub target: i32,
}

impl VisibleTarget {
    pub fn new(target: i32) -> Self {
        Self { target }
    }
}

#[derive(WorldQuery)]
#[world_query(mutable, derive(Debug))]
pub struct ObjQuery {
    id: &'static Id,
    player_id: &'static PlayerId,
    pos: &'static Position,
    class: &'static Class,
    subclass: &'static Subclass,
    state: &'static State,
}

#[derive(WorldQuery)]
#[world_query(mutable, derive(Debug))]
pub struct VillagerQuery {
    id: &'static Id,
    player_id: &'static PlayerId,
    pos: &'static Position,
    state: &'static mut State,
    attrs: &'static mut VillagerAttrs,
    order: &'static Order,
}

#[derive(WorldQuery)]
#[world_query(mutable, derive(Debug))]
pub struct NPCQuery {
    id: &'static Id,
    player_id: &'static PlayerId,
    pos: &'static Position,
    state: &'static mut State,
    visible_target: &'static mut VisibleTarget,
}

/// The systems that make structures tick.
pub struct AIPlugin;

impl Plugin for AIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(BigBrainPlugin)
            .add_system(nearby_target_system)
            //.add_system_to_stage(BigBrainStage::Actions, drink_action_system)
            .add_system_to_stage(BigBrainStage::Actions, process_order_system)
            .add_system_to_stage(BigBrainStage::Actions, attack_target_system)
            .add_system_to_stage(BigBrainStage::Scorers, target_scorer_system)
            //.add_system_to_stage(BigBrainStage::Scorers, thirsty_scorer_system)
            .add_system_to_stage(BigBrainStage::Scorers, morale_scorer_system);
    }
}

pub fn process_order_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    mut ids: ResMut<Ids>,
    map: Res<Map>,
    mut map_events: ResMut<MapEvents>,
    morales: Query<&Morale>,
    all_pos: Query<&Position>,
    mut villager_query: Query<VillagerQuery, (With<Order>, Without<EventInProgress>)>,
    mut query: Query<(&Actor, &mut ActionState, &ProcessOrder, &ActionSpan)>,
) {
    for (Actor(actor), mut state, _process_order, span) in &mut query {
        if let Ok(morale) = morales.get(*actor) {
            match *state {
                ActionState::Requested => {
                    trace!("Process Order Requested");

                    let Ok(villager) = villager_query.get(*actor) else {
                        trace!("No order to execute");
                        continue;
                    };

                    match villager.order {
                        Order::Follow { target } => {
                            debug!("Process Follow Order");
                            if let Ok(target_pos) = all_pos.get(*target) {
                                if villager.pos.x != target_pos.x || villager.pos.y != target_pos.y
                                {
                                    if is_none_state(&villager.state.0) {
                                        debug!("Executing Following");
                                        *state = ActionState::Executing;
                                    }
                                }
                            } else {
                                trace!("Invalid target to follow.");
                            }
                        }
                        Order::Gather { res_type } => {
                            debug!("Process Gather Order");
                            if is_none_state(&villager.state.0) {
                                debug!("Executing Gathering");
                                *state = ActionState::Executing;
                            }
                        }
                        Order::Operate => {
                            debug!("Process Operate Order");
                            if is_none_state(&villager.state.0) {
                                debug!("Executing Operate");
                                *state = ActionState::Executing;
                            }
                        }                        
                        Order::Refine => {
                            debug!("Process Refine Order");
                            if is_none_state(&villager.state.0) {
                                debug!("Executing Refining");
                                *state = ActionState::Executing;
                            }
                        }
                        Order::Craft { recipe_name } => {
                            debug!("Process Craft Order {:?}", recipe_name);
                            if is_none_state(&villager.state.0) {
                                debug!("Executing Crafting");
                                *state = ActionState::Executing;
                            }
                        }
                        Order::Experiment => {
                            debug!("Process Experiment Order");
                            if is_none_state(&villager.state.0) {
                                debug!("Executing Experiment");
                                *state = ActionState::Executing;
                            }
                        }
                        Order::Explore => {
                            debug!("Process Explore Order");
                            if is_none_state(&villager.state.0) {
                                debug!("Executing Explore");
                                *state = ActionState::Executing;
                            }
                        }                        
                        _ => {}
                    }
                }
                ActionState::Executing => {
                    trace!("Process Order Executing");

                    let Ok(mut villager) = villager_query.get_mut(*actor) else {
                        debug!("No villager order to process");
                        continue;
                    };

                    debug!("Processing villager order: {:?}", villager.order);

                    match villager.order {
                        Order::Follow { target } => {
                            if let Ok(target_pos) = all_pos.get(*target) {
                                if villager.pos.x != target_pos.x || villager.pos.y != target_pos.y
                                {
                                    if is_none_state(&villager.state.0) {
                                        if let Some(path_result) = Map::find_path(
                                            villager.pos.x,
                                            villager.pos.y,
                                            target_pos.x,
                                            target_pos.y,
                                            &map,
                                        ) {
                                            debug!("Follower path: {:?}", path_result);

                                            let (path, c) = path_result;
                                            let next_pos = &path[1];

                                            debug!("Next pos: {:?}", next_pos);

                                            // Add State Change Event to Moving
                                            let state_change_event =
                                                VisibleEvent::StateChangeEvent {
                                                    new_state: "moving".to_string()
                                                };

                                            villager.state.0 = "moving".to_string();

                                            map_events.new(
                                                ids.new_map_event_id(),
                                                *actor,
                                                villager.id,
                                                villager.player_id,
                                                villager.pos,
                                                game_tick.0 + 4,
                                                state_change_event,
                                            );

                                            // Add Move Event
                                            let move_event = VisibleEvent::MoveEvent {
                                                dst_x: next_pos.0,
                                                dst_y: next_pos.1,
                                            };

                                            map_events.new(
                                                ids.new_map_event_id(),
                                                *actor,
                                                villager.id,
                                                villager.player_id,
                                                villager.pos,
                                                game_tick.0 + 36, // in the future
                                                move_event,
                                            );

                                            commands.entity(*actor).insert(EventInProgress);
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
                            if is_none_state(&villager.state.0) {
                                let gather_event = VisibleEvent::GatherEvent {
                                    res_type: res_type.clone(),
                                };

                                map_events.new(
                                    ids.new_map_event_id(),
                                    *actor,
                                    villager.id,
                                    villager.player_id,
                                    villager.pos,
                                    game_tick.0 + 8, // in the future
                                    gather_event,
                                );
                            }
                        }
                        Order::Refine | Order::Operate => {
                            if is_none_state(&villager.state.0) {
                                let Some(structure_entity) = ids.get_entity(villager.attrs.structure) else {
                                    error!("Cannot find structure entity for {:?}", villager.attrs.structure);
                                    continue;
                                };

                                let Ok(structure_pos) = all_pos.get(structure_entity) else {
                                    error!("Query failed to find entity {:?}", structure_entity);
                                    continue;
                                };

                                // Check if villager is on structure
                                if villager.pos.x != structure_pos.x
                                    || villager.pos.y != structure_pos.y
                                {
                                    if let Some(path_result) = Map::find_path(
                                        villager.pos.x,
                                        villager.pos.y,
                                        structure_pos.x,
                                        structure_pos.y,
                                        &map,
                                    ) {
                                        debug!("Path to structure: {:?}", path_result);

                                        let (path, c) = path_result;
                                        let next_pos = &path[1];

                                        debug!("Next pos: {:?}", next_pos);

                                        // Add State Change Event to Moving
                                        let state_change_event = VisibleEvent::StateChangeEvent {
                                            new_state: "moving".to_string(),
                                        };

                                        villager.state.0 = "moving".to_string();

                                        map_events.new(
                                            ids.new_map_event_id(),
                                            *actor,
                                            villager.id,
                                            villager.player_id,
                                            villager.pos,
                                            game_tick.0 + 4,
                                            state_change_event,
                                        );

                                        // Add Move Event
                                        let move_event = VisibleEvent::MoveEvent {
                                            dst_x: next_pos.0,
                                            dst_y: next_pos.1,
                                        };

                                        map_events.new(
                                            ids.new_map_event_id(),
                                            *actor,
                                            villager.id,
                                            villager.player_id,
                                            villager.pos,
                                            game_tick.0 + 36, // in the future
                                            move_event,
                                        );

                                        commands.entity(*actor).insert(EventInProgress);
                                    }
                                } else {
                                    // Create operate or refine event

                                    match villager.order {
                                        Order::Refine => {
                                            let refine_event = VisibleEvent::RefineEvent {
                                                structure_id: villager.attrs.structure,
                                            };
    
                                            villager.state.0 = "refining".to_string();
    
                                            map_events.new(
                                                ids.new_map_event_id(),
                                                *actor,
                                                villager.id,
                                                villager.player_id,
                                                villager.pos,
                                                game_tick.0 + 40, // in the future
                                                refine_event,
                                            );
                                        }     
                                        Order::Operate => {
                                            let operate_event = VisibleEvent::OperateEvent {
                                                structure_id: villager.attrs.structure,
                                            };
    
                                            //TODO look up subclass of structure and replace operating with mining, lumberjacking, etc...
                                            villager.state.0 = "operating".to_string();
    
                                            map_events.new(
                                                ids.new_map_event_id(),
                                                *actor,
                                                villager.id,
                                                villager.player_id,
                                                villager.pos,
                                                game_tick.0 + 40, // in the future
                                                operate_event,
                                            );                                            
                                        }                                   
                                        _ => {
                                            error!("Invalid order type: {:?}", villager.order);
                                            continue;
                                        }
                                    }

                                    commands.entity(*actor).insert(EventInProgress);

                                    *state = ActionState::Success;
                                }
                            }
                        }
                        Order::Craft { recipe_name } => {
                            if is_none_state(&villager.state.0) {
                                let Some(structure_entity) = ids.get_entity(villager.attrs.structure) else {
                                    error!("Cannot find structure entity for {:?}", villager.attrs.structure);
                                    continue;
                                };

                                let Ok(structure_pos) = all_pos.get(structure_entity) else {
                                    error!("Query failed to find entity {:?}", structure_entity);
                                    continue;
                                };

                                // Check if villager is on structure
                                if villager.pos.x != structure_pos.x
                                    || villager.pos.y != structure_pos.y
                                {
                                    if let Some(path_result) = Map::find_path(
                                        villager.pos.x,
                                        villager.pos.y,
                                        structure_pos.x,
                                        structure_pos.y,
                                        &map,
                                    ) {
                                        debug!("Path to structure: {:?}", path_result);

                                        let (path, c) = path_result;
                                        let next_pos = &path[1];

                                        debug!("Next pos: {:?}", next_pos);

                                        // Add State Change Event to Moving
                                        let state_change_event = VisibleEvent::StateChangeEvent {
                                            new_state: "moving".to_string(),
                                        };

                                        villager.state.0 = "moving".to_string();

                                        map_events.new(
                                            ids.new_map_event_id(),
                                            *actor,
                                            villager.id,
                                            villager.player_id,
                                            villager.pos,
                                            game_tick.0 + 4,
                                            state_change_event,
                                        );

                                        // Add Move Event
                                        let move_event = VisibleEvent::MoveEvent {
                                            dst_x: next_pos.0,
                                            dst_y: next_pos.1,
                                        };

                                        map_events.new(
                                            ids.new_map_event_id(),
                                            *actor,
                                            villager.id,
                                            villager.player_id,
                                            villager.pos,
                                            game_tick.0 + 36, // in the future
                                            move_event,
                                        );

                                        commands.entity(*actor).insert(EventInProgress);
                                    }
                                } else {
                                    // Create craft event
                                    let craft_event = VisibleEvent::CraftEvent {
                                        structure_id: villager.attrs.structure,
                                        recipe_name: recipe_name.clone(),
                                    };

                                    // Add State Change Event to Moving
                                    let state_change_event = VisibleEvent::StateChangeEvent {
                                        new_state: "crafting".to_string(),
                                    };

                                    villager.state.0 = "crafting".to_string();

                                    map_events.new(
                                        ids.new_map_event_id(),
                                        *actor,
                                        villager.id,
                                        villager.player_id,
                                        villager.pos,
                                        game_tick.0 + 4,
                                        state_change_event,
                                    );

                                    map_events.new(
                                        ids.new_map_event_id(),
                                        *actor,
                                        villager.id,
                                        villager.player_id,
                                        villager.pos,
                                        game_tick.0 + 200, // in the future
                                        craft_event,
                                    );

                                    commands.entity(*actor).insert(EventInProgress);

                                    *state = ActionState::Success;
                                }
                            }
                        }
                        Order::Experiment => {
                            if is_none_state(&villager.state.0) {
                                let Some(structure_entity) = ids.get_entity(villager.attrs.structure) else {
                                    error!("Cannot find structure entity for {:?}", villager.attrs.structure);
                                    continue;
                                };

                                let Ok(structure_pos) = all_pos.get(structure_entity) else {
                                    error!("Query failed to find entity {:?}", structure_entity);
                                    continue;
                                };

                                // Check if villager is on structure
                                if villager.pos.x != structure_pos.x
                                    || villager.pos.y != structure_pos.y
                                {
                                    if let Some(path_result) = Map::find_path(
                                        villager.pos.x,
                                        villager.pos.y,
                                        structure_pos.x,
                                        structure_pos.y,
                                        &map,
                                    ) {
                                        debug!("Path to structure: {:?}", path_result);

                                        let (path, c) = path_result;
                                        let next_pos = &path[1];

                                        debug!("Next pos: {:?}", next_pos);

                                        // Add State Change Event to Moving
                                        let state_change_event = VisibleEvent::StateChangeEvent {
                                            new_state: "moving".to_string(),
                                        };

                                        villager.state.0 = "moving".to_string();

                                        map_events.new(
                                            ids.new_map_event_id(),
                                            *actor,
                                            villager.id,
                                            villager.player_id,
                                            villager.pos,
                                            game_tick.0 + 4,
                                            state_change_event,
                                        );

                                        // Add Move Event
                                        let move_event = VisibleEvent::MoveEvent {
                                            dst_x: next_pos.0,
                                            dst_y: next_pos.1,
                                        };

                                        map_events.new(
                                            ids.new_map_event_id(),
                                            *actor,
                                            villager.id,
                                            villager.player_id,
                                            villager.pos,
                                            game_tick.0 + 36, // in the future
                                            move_event,
                                        );

                                        commands.entity(*actor).insert(EventInProgress);
                                    }
                                } else {
                                    // Create craft event
                                    let craft_event = VisibleEvent::ExperimentEvent {
                                       structure_id: villager.attrs.structure
                                    };

                                    // Add State Change Event to Moving
                                    let state_change_event = VisibleEvent::StateChangeEvent {
                                        new_state: "experimenting".to_string(),
                                    };

                                    villager.state.0 = "experimenting".to_string();

                                    map_events.new(
                                        ids.new_map_event_id(),
                                        *actor,
                                        villager.id,
                                        villager.player_id,
                                        villager.pos,
                                        game_tick.0 + 4,
                                        state_change_event,
                                    );

                                    map_events.new(
                                        ids.new_map_event_id(),
                                        *actor,
                                        villager.id,
                                        villager.player_id,
                                        villager.pos,
                                        game_tick.0 + 200, // in the future
                                        craft_event,
                                    );

                                    commands.entity(*actor).insert(EventInProgress);

                                    *state = ActionState::Success;
                                }
                            }
                        }
                        Order::Explore => {
                            if is_none_state(&villager.state.0) {
                                let explore_event = VisibleEvent::ExploreEvent;

                                map_events.new(
                                    ids.new_map_event_id(),
                                    *actor,
                                    villager.id,
                                    villager.player_id,
                                    villager.pos,
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
}

/*pub fn npc_chase_attack_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    mut ids: ResMut<Ids>,
    map: Res<Map>,
    mut map_events: ResMut<MapEvents>,
    mut npc_query: Query<NPCQuery, With<SubclassNPC>>,
    mut target_query: Query<ObjQuery, Or<(With<SubclassHero>, With<SubclassVillager>)>>,
    mut query: Query<(&Actor, &mut ActionState, &ChaseAttack, &ActionSpan)>,
) {
    for (Actor(actor), mut state, _chase_attack, span) in &mut query {
        match *state {
            ActionState::Requested => {
                let Ok(npc) = npc_query.get(*actor) else {
                    trace!("No npc entity found");
                    continue;
                };

                if npc.attrs.target == NO_TARGET {
                    if is_none_state(&npc.state.0) {
                        debug!("Executing Chase");

                        let mut closest_distance = 100000;
                        let mut closest_target = None;

                        for target in target_query.iter() {
                            let distance = Map::dist(*npc.pos, *target.pos);

                            if distance < closest_distance {
                                closest_distance = distance;
                                closest_target = Some(target);
                            }
                        }

                        if closest_target.is_some() {
                            debug!("Target selected: {:?}", closest_target);
                            *state = ActionState::Executing;
                        }
                    }
                } else {
                    // Get target entity
                    let Some(target_entity) = ids.get_entity(npc.attrs.target) else {
                        error!("Cannot find target entity for {:?}", npc.attrs.target);
                        continue;
                    };

                    let Ok(target) = target_query.get(target_entity) else {
                        error!("Query failed to find entity {:?}", target_entity);
                        continue;
                    };

                    if Map::is_adjacent(*npc.pos, *target.pos) {
                        debug!("Target is adjacent, time to attack");
                    }
                }
            }
            ActionState::Executing => {}
            ActionState::Cancelled => {
                debug!("Action was cancelled. Considering this a failure.");
                *state = ActionState::Failure;
            }
            _ => {}
        }
    }
}*/

pub fn drink_action_system(
    mut commands: Commands,
    tick: Res<GameTick>,
    mut items: ResMut<Items>,
    mut thirsts: Query<&mut Thirst>,
    all: Query<(&Id, &State)>, // TODO combine into 1 query using options
    drinking_states: Query<&DrinkingState>, // TODO combine into 1 query using options
    mut query: Query<(&Actor, &mut ActionState, &Drink, &ActionSpan)>,
) {
    for (Actor(actor), mut state, _drink, span) in &mut query {
        let _guard = span.span().enter();

        // Use the drink_action's actor to look up the corresponding Thirst Component.
        if let Ok(mut thirst) = thirsts.get_mut(*actor) {
            match *state {
                ActionState::Requested => {
                    if let Ok((obj_id, obj_state)) = all.get(*actor) {
                        if is_none_state(&obj_state.0) {
                            if let Some(item) = Item::update_quantity_by_class(
                                obj_id.0,
                                WATER.to_string(),
                                -1,
                                &mut items,
                            ) {
                                if let Some(thirst_mod) = item.attrs.get(THIRST) {
                                    let drinking_state = DrinkingState {
                                        thirst_mod: *thirst_mod,
                                        end_tick: tick.0 + 20,
                                    };

                                    debug!("Adding Drinking State");
                                    commands.entity(*actor).insert(drinking_state);

                                    debug!("Time to drink some water!");
                                    *state = ActionState::Executing;
                                } else {
                                    debug!("Missing thirst mod on item, action failure.");
                                    *state = ActionState::Failure;
                                }
                            } else {
                                debug!("No water items found, action failure.");
                                *state = ActionState::Failure;
                            }
                        }
                    } else {
                        debug!("Id component look up failed, action failure.");
                        *state = ActionState::Failure;
                    }
                }
                ActionState::Executing => {
                    debug!("Drinking...");

                    if let Ok(drinking_state) = drinking_states.get(*actor) {
                        if tick.0 >= drinking_state.end_tick {
                            thirst.thirst -= drinking_state.thirst_mod;

                            commands.entity(*actor).remove::<DrinkingState>();

                            debug!("Done drinking water");
                            *state = ActionState::Success;
                        } else {
                            debug!("Still drinking...")
                        }
                    } else {
                        debug!("Something went wrong with drinking.");
                        *state = ActionState::Failure;
                    }
                }
                // All Actions should make sure to handle cancellations!
                ActionState::Cancelled => {
                    debug!("Action was cancelled. Considering this a failure.");
                    *state = ActionState::Failure;
                }
                _ => {}
            }
        }
    }
}

fn attack_target_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    mut ids: ResMut<Ids>,
    map: Res<Map>,
    mut map_events: ResMut<MapEvents>,
    mut items: ResMut<Items>,
    templates: Res<Templates>,
    visible_target_query: Query<&VisibleTarget, Without<EventInProgress>>,
    mut npc_query: Query<CombatQuery, (With<SubclassNPC>, Without<EventInProgress>)>,
    mut target_query: Query<CombatQuery, Without<SubclassNPC>>,
    mut query: Query<(&Actor, &mut ActionState, &ChaseAttack)>,
) {
    for (Actor(actor), mut state, chase_attack) in &mut query {
        let Ok(visible_target) = visible_target_query.get(*actor) else {
            continue;
        };

        match *state {
            ActionState::Requested => {
                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                debug!("Attacking executing...");
                let target_id = visible_target.target;

                let Ok(mut npc) = npc_query.get_mut(*actor) else {
                    error!("Query failed to find entity {:?}", *actor);
                    *state = ActionState::Failure;
                    continue;
                };

                if target_id == NO_TARGET {
                    debug!("No target to chase, start wandering...");
                    let wander_pos_list = Map::get_neighbour_tiles(npc.pos.x, npc.pos.y, &map);

                    if is_none_state(&npc.state.0) {
                        let mut rng = rand::thread_rng();

                        let random_index = rng.gen_range(0..wander_pos_list.len());

                        if let Some((random_pos, _movement_cost)) =
                            wander_pos_list.get(random_index)
                        {
                            let random_pos_x = random_pos.0;
                            let random_pos_y = random_pos.1;

                            // Add State Change Event to Moving
                            let state_change_event = VisibleEvent::StateChangeEvent {
                                new_state: "moving".to_string(),
                            };

                            npc.state.0 = "moving".to_string();

                            map_events.new(
                                ids.new_map_event_id(),
                                *actor,
                                npc.id,
                                npc.player_id,
                                npc.pos,
                                game_tick.0 + 4,
                                state_change_event,
                            );

                            // Add Move Event
                            let move_event = VisibleEvent::MoveEvent {
                                dst_x: random_pos_x,
                                dst_y: random_pos_y,
                            };

                            map_events.new(
                                ids.new_map_event_id(),
                                *actor,
                                npc.id,
                                npc.player_id,
                                npc.pos,
                                game_tick.0 + 36, // in the future
                                move_event,
                            );

                            commands.entity(*actor).insert(EventInProgress);
                        }
                    }
                } else {
                    debug!("Time to chase and attack target {:?}!", target_id);

                    // Get target entity
                    let Some(target_entity) = ids.get_entity(target_id) else {
                        *state = ActionState::Failure;
                        error!("Cannot find target entity for {:?}", target_id);
                        continue;
                    };

                    let Ok(mut target) = target_query.get_mut(target_entity) else {
                        error!("Query failed to find entity {:?}", target_entity);
                        *state = ActionState::Failure;
                        continue;
                    };

                    if Map::is_adjacent(*npc.pos, *target.pos) {
                        debug!("Target is adjacent, time to attack");

                        // Calculate and process damage
                        let (damage, _skill_gain) = Combat::process_damage(
                            "quick".to_string(),
                            &npc,
                            &mut target,
                            &mut commands,
                            &mut items,
                            &templates
                        );

                        // Add visible damage event to broadcast to everyone nearby
                        Combat::add_damage_event(
                            ids.new_map_event_id(),
                            game_tick.0,
                            "quick".to_string(),
                            damage,
                            &npc,
                            &target,
                            &mut map_events,
                        );

                        // Add Cooldown Event
                        let cooldown_event = VisibleEvent::CooldownEvent { duration: 30 };

                        map_events.new(
                            ids.new_map_event_id(),
                            *actor,
                            npc.id,
                            npc.player_id,
                            npc.pos,
                            game_tick.0 + 30, // in the future
                            cooldown_event,
                        );

                        commands.entity(*actor).insert(EventInProgress);
                    } else {
                        if is_none_state(&npc.state.0) {
                            if let Some(path_result) = Map::find_path(
                                npc.pos.x,
                                npc.pos.y,
                                target.pos.x,
                                target.pos.y,
                                &map,
                            ) {
                                debug!("Follower path: {:?}", path_result);

                                let (path, c) = path_result;
                                let next_pos = &path[1];

                                debug!("Next pos: {:?}", next_pos);

                                // Add State Change Event to Moving
                                let state_change_event = VisibleEvent::StateChangeEvent {
                                    new_state: "moving".to_string(),
                                };

                                npc.state.0 = "moving".to_string();

                                map_events.new(
                                    ids.new_map_event_id(),
                                    *actor,
                                    npc.id,
                                    npc.player_id,
                                    npc.pos,
                                    game_tick.0 + 4,
                                    state_change_event,
                                );

                                // Add Move Event
                                let move_event = VisibleEvent::MoveEvent {
                                    dst_x: next_pos.0,
                                    dst_y: next_pos.1,
                                };

                                map_events.new(
                                    ids.new_map_event_id(),
                                    *actor,
                                    npc.id,
                                    npc.player_id,
                                    npc.pos,
                                    game_tick.0 + 36, // in the future
                                    move_event,
                                );

                                commands.entity(*actor).insert(EventInProgress);
                            }
                        }
                    }

                    *state = ActionState::Success;
                }
            }
            // All Actions should make sure to handle cancellations!
            ActionState::Cancelled => {
                debug!("Action was cancelled. Considering this a failure.");
                *state = ActionState::Failure;
            }
            _ => {}
        }
    }
}

pub fn nearby_target_system(
    game_tick: Res<GameTick>,
    mut npc_query: Query<(&Position, &Viewshed, &mut VisibleTarget), With<SubclassNPC>>,
    target_query: Query<ObjQuery, Or<(With<SubclassHero>, With<SubclassVillager>)>>,
) {
    if game_tick.0 % 10 == 0 {
        for (npc_pos, npc_viewshed, mut npc_visible_target) in npc_query.iter_mut() {
            debug!("NPC_POS: {:?} NPC_VIEWSHED: {:?}", npc_pos, npc_viewshed);

            let mut min_distance = u32::MAX;
            let mut target_id = NO_TARGET;

            for target in target_query.iter() {
                let distance = Map::dist(*npc_pos, *target.pos);

                if npc_viewshed.range >= distance {
                    if distance < min_distance {
                        min_distance = distance;
                        target_id = target.id.0;
                    }
                }
            }

            debug!("Distance system target_id: {:?}", target_id);
            npc_visible_target.target = target_id;
        }
    }
}

// Looks familiar? It's a lot like Actions!
pub fn target_scorer_system(
    target_query: Query<&VisibleTarget>,
    // Same dance with the Actor here, but now we use look up Score instead of ActionState.
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<VisibleTargetScorerBuilder>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        if let Ok(target) = target_query.get(*actor) {
            debug!("Scorer target_id: {:?}", target);
            if target.target != NO_TARGET {
                score.set(1.0);
            } else {
                score.set(0.0);
            }
        }
    }
}

// Looks familiar? It's a lot like Actions!
pub fn thirsty_scorer_system(
    thirsts: Query<&Thirst>,
    // Same dance with the Actor here, but now we use look up Score instead of ActionState.
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<Thirsty>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        if let Ok(thirst) = thirsts.get(*actor) {
            // This is really what the job of a Scorer is. To calculate a
            // generic "Utility" score that the Big Brain engine will compare
            // against others, over time, and use to make decisions. This is
            // generally "the higher the better", and "first across the finish
            // line", but that's all configurable using Pickers!
            //
            // The score here must be between 0.0 and 1.0.
            score.set(thirst.thirst / 100.0);
            if thirst.thirst >= 80.0 {
                span.span().in_scope(|| {
                    debug!("Thirst above threshold! Score: {}", thirst.thirst / 100.0)
                });
            }
        }
    }
}

// Looks familiar? It's a lot like Actions!
pub fn morale_scorer_system(
    morales: Query<&Morale>,
    // Same dance with the Actor here, but now we use look up Score instead of ActionState.
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<HighMorale>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        if let Ok(morale) = morales.get(*actor) {
            score.set(morale.morale / 100.0);

            if morale.morale >= 80.0 {
                span.span().in_scope(|| {
                    trace!("Morale above threshold! Score: {}", morale.morale / 100.0)
                });
            }
        }
    }
}

// if let Some(struct_name.field) =
