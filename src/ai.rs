use bevy::prelude::*;

use big_brain::prelude::*;
use pathfinding::prelude::directions::E;

use crate::game::is_none_state;
use crate::game::GameTick;
use crate::game::{
    EventInProgress, Id, Ids, VisibleEvent, MapEvents, OrderFollow, PlayerId, Position, State,
};
use crate::item::{Item, Items, THIRST, WATER};
use crate::map::Map;

#[derive(Clone, Component, Debug)]
pub struct HighMorale;

#[derive(Clone, Component, Debug)]
pub struct ProcessOrder;

#[derive(Clone, Component, Debug)]
pub struct Thirsty;

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

/// The systems that make structures tick.
pub struct AIPlugin;

impl Plugin for AIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(BigBrainPlugin)
            .add_system_to_stage(BigBrainStage::Actions, drink_action_system)
            .add_system_to_stage(BigBrainStage::Actions, process_order_system)
            .add_system_to_stage(BigBrainStage::Scorers, thirsty_scorer_system)
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
    all_pos: Query<&Position, Without<OrderFollow>>,
    mut tasks: Query<
        (&Id, &PlayerId, &Position, &mut State, &OrderFollow),
        (With<OrderFollow>, Without<EventInProgress>),
    >,
    mut query: Query<(&Actor, &mut ActionState, &ProcessOrder, &ActionSpan)>,
) {
    for (Actor(actor), mut state, _process_order, span) in &mut query {
        if let Ok(morale) = morales.get(*actor) {
            match *state {
                ActionState::Requested => {
                    debug!("Process Order Requested");

                    if let Ok((obj_id, player_id, follower_pos, follower_state, order_follow)) =
                        tasks.get(*actor)
                    {
                        if let Ok(target_pos) = all_pos.get(order_follow.target) {
                            if follower_pos.x != target_pos.x || follower_pos.y != target_pos.y {
                                if is_none_state(&follower_state.0) {
                                    debug!("Executing following");
                                    *state = ActionState::Executing;
                                }
                            }
                        }
                    } else {
                        trace!("No order to execute");
                    }
                }
                ActionState::Executing => {
                    debug!("Process Order Executing");
                    if let Ok((obj_id, player_id, follower_pos, mut follower_state, order_follow)) =
                        tasks.get_mut(*actor)
                    {
                        if let Ok(target_pos) = all_pos.get(order_follow.target) {
                            if follower_pos.x != target_pos.x || follower_pos.y != target_pos.y {
                                if is_none_state(&follower_state.0) {
                                    if let Some(path_result) = Map::find_path(
                                        follower_pos.x,
                                        follower_pos.y,
                                        target_pos.x,
                                        target_pos.y,
                                        &map,
                                    ) {
                                        println!("Follower path: {:?}", path_result);

                                        let (path, c) = path_result;
                                        let next_pos = &path[1];

                                        println!("Next pos: {:?}", next_pos);

                                        // Add State Change Event to Moving
                                        let state_change_event = VisibleEvent::StateChangeEvent {
                                            new_state: "moving".to_string(),
                                        };

                                        follower_state.0 = "moving".to_string();

                                        map_events.new(
                                            ids.new_map_event_id(),
                                            *actor,
                                            obj_id,
                                            player_id,
                                            follower_pos,
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
                                            obj_id,
                                            player_id,
                                            follower_pos,
                                            game_tick.0 + 36, // in the future
                                            move_event,
                                        );

                                        commands.entity(*actor).insert(EventInProgress);

                                    }
                                } {
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

pub fn drink_action_system(
    mut commands: Commands,
    tick: Res<GameTick>,
    mut items: ResMut<Items>,
    mut thirsts: Query<&mut Thirst>,
    all: Query<(&Id, &State)>,                        // TODO combine into 1 query using options
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
                            if let Some(item) =
                                Item::update_quantity_by_class(obj_id.0, WATER.to_string(), -1, &mut items)
                            {
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
