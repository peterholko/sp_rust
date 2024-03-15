use bevy::prelude::*;
use big_brain::prelude::*;
use rand::Rng;

use crate::combat::CombatQuery;
use crate::components::npc::{
    AtDestinationScorer, AtLanding, Forfeiture, Idle, InEmpire, IsAboard, IsPassengerAboard, IsTargetAdjacent, IsTaxCollected, IsWaitingForPassenger, MoveToEmpire, MoveToPos, MoveToTarget, NoTaxesToCollect, OverdueTaxScorer, ReadyToSailScorer, SetDestination, TaxCollector, TaxCollectorTransport, TaxesToCollect, Transport, VisibleTarget, WaitForPassenger
};
use crate::effect::Effect;
use crate::event::{GameEvents, MapEvents, VisibleEvent};
use crate::game::{self, State};
use crate::ids::Ids;
use crate::item::*;
use crate::map::Map;
use crate::obj::Obj;
use crate::obj::{self, ObjStatQuery};
use crate::plugins::ai::npc::{BASE_MOVE_TICKS, BASE_SPEED, NO_TARGET};
use crate::templates::Templates;
use crate::{game::*, item};

pub fn update_tax_collection_system(
    game_tick: ResMut<GameTick>,
    ids: ResMut<Ids>,
    mut collector_query: Query<&mut TaxCollector>
) {
    for mut collector in collector_query.iter_mut() {
        let next_tax_collection = collector.last_collection_time + 1000;
        if next_tax_collection <= game_tick.0 {
            info!("Tax collection time for {:?}", collector.target_player);
            collector.collection_amount = 50;
            collector.last_collection_time = game_tick.0;
        }
    }
}

pub fn is_aboard_scorer_system(
    game_tick: Res<GameTick>,
    state_query: Query<&State, With<TaxCollector>>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<IsAboard>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        if let Ok(state) = state_query.get(*actor) {
            if *state == State::Aboard {
                score.set(0.8);
            } else {
                score.set(0.0);
            }
        }
    }
}

pub fn is_tax_collected_scorer_system(
    game_tick: Res<GameTick>,
    items: ResMut<Items>,
    tax_collector_query: Query<(&Id, &TaxCollector)>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<IsTaxCollected>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        if let Ok((id, tax_collector)) = tax_collector_query.get(*actor) {
            if let Some(gold) = items.get_by_class(id.0, item::GOLD.to_string()) {
                if gold.quantity >= tax_collector.collection_amount {
                    score.set(1.0);
                } else {
                    score.set(0.0);
                }
            } else {
                score.set(0.0);
            }
        }
    }
}

pub fn in_empire_scorer_system(
    game_tick: Res<GameTick>,
    pos_query: Query<&Position, With<TaxCollector>>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<InEmpire>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        if let Ok(pos) = pos_query.get(*actor) {
            if Map::in_empire(*pos) {
                score.set(1.0);
            } else {
                score.set(0.0);
            }
        }
    }
}

pub fn at_landing_scorer_system(
    game_tick: Res<GameTick>,
    collector_query: Query<(&Position, &TaxCollector)>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<AtLanding>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        if let Ok((pos, tax_collector)) = collector_query.get(*actor) {
            if Map::is_adjacent(*pos, tax_collector.landing_pos) {
                info!("Tax Collector at landing!");
                score.set(0.9);
            } else {
                score.set(0.0);
            }
        }
    }
}

pub fn at_destination_scorer_system(
    mut transport_query: Query<(&Position, &mut Transport)>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<AtDestinationScorer>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        if let Ok((pos, mut transport)) = transport_query.get_mut(*actor) {
            if pos == &transport.route[transport.next_stop as usize] {
                info!("Reached destination, now waiting...");
                score.set(0.7);
            } else {
                info!("Not at destination");
                score.set(0.0);
            }
        }
    }
}

/*pub fn ready_to_sail_scorer_system(
    game_tick: Res<GameTick>,
    mut transport_query: Query<(&Position, &mut Transport)>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<ReadyToSailScorer>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        if let Ok((pos, mut transport)) = transport_query.get_mut(*actor) {
            if pos == &transport.route[transport.next_stop as usize] {
                if transport.ready_to_sail {
                    info!("Ready to sail");
                    score.set(0.8);
                } else {
                    score.set(0.0);
                }
            } else {
                score.set(0.0);
            }
        }
    }*/

pub fn is_passenger_aboard_scorer_system(
    game_tick: Res<GameTick>,
    transport_query: Query<(&Position, &mut Transport, &WaitForPassenger)>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<IsPassengerAboard>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        if let Ok((pos, transport, wait_for_passenger)) = transport_query.get(*actor) {
            info!("IsPassengerAboard: {:?}", transport.hauling);
            if pos != &transport.route[transport.next_stop as usize] {
                if transport.hauling.contains(&wait_for_passenger.id) {
                    info!("Passenger is aboard");
                    score.set(0.9);
                } else {
                    info!("Passenger is not aboard");
                    score.set(0.0);
                }
            } else {
                score.set(0.0);
            }
        }
    }
}

pub fn is_waiting_for_passenger_scorer_system(
    game_tick: Res<GameTick>,
    transport_query: Query<(&mut Transport, &WaitForPassenger)>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<IsWaitingForPassenger>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        if let Ok((transport, wait_for_passenger)) = transport_query.get(*actor) {
            if transport.hauling.contains(&wait_for_passenger.id) {
                info!("Passenger is aboard");
                score.set(0.9);
            } else {
                info!("Passenger is not aboard");
                score.set(0.0);
            }
        }
    }
}

pub fn is_target_adjacent_scorer_system(
    game_tick: Res<GameTick>,
    ids: Res<Ids>,
    pos_query: Query<&Position>,
    collector_query: Query<&TaxCollector>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<IsTargetAdjacent>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        if let Ok(tax_collector) = collector_query.get(*actor) {
            let Some(hero_id) = ids.get_hero(tax_collector.target_player) else {
                error!(
                    "Cannot find hero for player {:?}",
                    tax_collector.target_player
                );
                continue;
            };

            let Some(hero_entity) = ids.get_entity(hero_id) else {
                error!("Cannot find entity for {:?}", hero_id);
                continue;
            };

            let Ok(hero_pos) = pos_query.get(hero_entity) else {
                error!("No hero position found");
                continue;
            };

            let Ok(tax_collector_pos) = pos_query.get(*actor) else {
                error!("No tax_collector position found");
                continue;
            };

            if Map::is_adjacent(*hero_pos, *tax_collector_pos) {
                score.set(1.0);
            } else {
                score.set(0.0);
            }
        }
    }
}

pub fn no_taxes_to_collect_scorer_system(
    ids: ResMut<Ids>,
    transport_query: Query<(&Transport, &TaxCollectorTransport)>,
    collector_query: Query<&TaxCollector>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<NoTaxesToCollect>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        let Ok((transport, tc_transport)) = transport_query.get(*actor) else {
            continue;
        };

        // Get entity from passenger id
        let Some(collector_entity) = ids.get_entity(tc_transport.tax_collector_id) else {
            error!("Cannot find entity for {:?}", tc_transport.tax_collector_id);
            continue;
        };

        if let Ok(collector) = collector_query.get(collector_entity) {
            if collector.collection_amount == 0 && transport.hauling.contains(&tc_transport.tax_collector_id) {
                score.set(1.0);
            } else {
                score.set(0.0);
            }
        }
    }
}

pub fn taxes_to_collect_scorer_system(
    ids: ResMut<Ids>,
    transport_query: Query<(&Transport, &TaxCollectorTransport)>,
    collector_query: Query<&TaxCollector>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<TaxesToCollect>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        let Ok((transport, tc_transport)) = transport_query.get(*actor) else {
            continue;
        };

        // Get entity from passenger id
        let Some(collector_entity) = ids.get_entity(tc_transport.tax_collector_id) else {
            error!("Cannot find entity for {:?}", tc_transport.tax_collector_id);
            continue;
        };

        if let Ok(collector) = collector_query.get(collector_entity) {
            if collector.collection_amount > 0 {
                score.set(1.0);
            } else {
                score.set(0.0);
            }
        }
    }    
}

pub fn overdue_tax_scorer_system(
    game_tick: Res<GameTick>,
    items: ResMut<Items>,
    collector_query: Query<(&Id, &TaxCollector)>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<OverdueTaxScorer>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        if let Ok((id, collector)) = collector_query.get(*actor) {
            if collector.last_collection_time + 500 < game_tick.0 {
                if let Some(gold) = items.get_by_class(id.0, item::GOLD.to_string()) {
                    if gold.quantity < collector.collection_amount {
                        score.set(1.0);
                    } else {
                        score.set(0.0);
                    }
                }
            }
        }
    }
}

pub fn idle_action_system(mut query: Query<(&Actor, &mut ActionState, &Idle, &ActionSpan)>) {
    for (Actor(actor), mut state, _idle, span) in &mut query {
        match *state {
            ActionState::Requested => {
                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                *state = ActionState::Success;
            }
            ActionState::Cancelled => {
                *state = ActionState::Failure;
            }
            _ => {}
        }
    }
}

/*pub fn set_destination_action_system(
    mut transport_query: Query<(&mut Transport, &WaitForPassenger)>,
    mut query: Query<(&Actor, &mut ActionState, &SetDestination, &ActionSpan)>,
) {
    for (Actor(actor), mut state, _idle, span) in &mut query {
        match *state {
            ActionState::Requested => {
                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                // Increment the transport next stop
                if let Ok((mut transport, wait_for_passenger)) = transport_query.get_mut(*actor) {
                    transport.ready_to_sail = false;

                    // Decrement if next stop is last in list
                    if transport.next_stop == transport.route.len() as i32 - 1 {
                        transport.next_stop = transport.next_stop - 1;
                    } else {
                        transport.next_stop = transport.next_stop + 1;
                    }
                }

                *state = ActionState::Success;
            }
            ActionState::Cancelled => {
                *state = ActionState::Failure;
            }
            _ => {}
        }
    }
}*/

pub fn move_to_target_action_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    ids: ResMut<Ids>,
    map: Res<Map>,
    mut map_events: ResMut<MapEvents>,
    templates: Res<Templates>,
    mut tax_collector_query: Query<(&PlayerId, &mut TaxCollector), Without<EventInProgress>>,
    mut obj_query: Query<ObjStatQuery>,
    mut query: Query<(&Actor, &mut ActionState, &MoveToTarget)>,
) {
    for (Actor(actor), mut state, move_to_target) in &mut query {
        let Ok((tax_collector_player_id, mut tax_collector)) = tax_collector_query.get_mut(*actor)
        else {
            continue;
        };

        match *state {
            ActionState::Requested => {
                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                debug!("Executing MoveToTarget Action");

                let Some(target_entity) = ids.get_entity(move_to_target.target) else {
                    error!("Cannot find entity for {:?}", move_to_target.target);
                    *state = ActionState::Failure;
                    continue;
                };

                // Have to get the list of collision positions before querying the npc and target
                let collision_list = Obj::get_collision_list(tax_collector_player_id.0, &obj_query);

                let entities = [*actor, target_entity];

                let Ok([mut npc, target]) = obj_query.get_many_mut(entities) else {
                    error!("Query failed to find entities {:?}", entities);
                    *state = ActionState::Failure;
                    continue;
                };

                // TODO is it possible to stun a transported object?
                /*if obj.effects.0.contains_key(&Effect::Stunned) {
                    debug!("NPC is stunned");
                    continue;
                }*/

                // Get NPC speed
                let mut npc_speed = 1;

                if let Some(npc_base_speed) = npc.stats.base_speed {
                    npc_speed = npc_base_speed;
                }

                let effect_speed_mod = npc.effects.get_speed_effects(&templates);

                let move_duration = (BASE_MOVE_TICKS
                    * (BASE_SPEED / npc_speed as f32)
                    * (1.0 / effect_speed_mod)) as i32;

                let reached_destination;

                if npc.player_id.0 == target.player_id.0 {
                    reached_destination = npc.pos == target.pos;
                } else {
                    reached_destination = Map::is_adjacent(*npc.pos, *target.pos);
                }

                if reached_destination {
                    if npc.player_id.0 != target.player_id.0 {
                        if tax_collector.last_demand_time + 100 < game_tick.0 {
                            tax_collector.last_demand_time = game_tick.0;

                            let sound_event = VisibleEvent::SoundObjEvent {
                                sound: "Tax Time! Pay now or asset forfeiture!".to_string(),
                                intensity: 2,
                            };

                            map_events.new(npc.id.0, game_tick.0 + 4, sound_event);
                        }
                    }
                } else {
                    info!("Moving to target... {:?}", collision_list);
                    if *npc.state == State::None || *npc.state == State::Aboard {
                        // Get colliding objects

                        if let Some(path_result) = Map::find_path(
                            *npc.pos,
                            *target.pos,
                            &map,
                            collision_list,
                            true,
                            false,
                            false,
                            true // Allow move onto position with transport
                        ) {
                            info!("Follower path: {:?}", path_result);

                            let (path, c) = path_result;
                            let next_pos = &path[1];

                            info!("Next pos: {:?}", next_pos);

                            // Add State Change Event to Moving
                            let state_change_event = VisibleEvent::StateChangeEvent {
                                new_state: "moving".to_string(),
                            };

                            *npc.state = State::Moving;

                            map_events.new(npc.id.0, game_tick.0 + 4, state_change_event);

                            // Add Move Event
                            let move_event = VisibleEvent::MoveEvent {
                                dst_x: next_pos.0,
                                dst_y: next_pos.1,
                            };

                            let move_map_event =
                                map_events.new(npc.id.0, game_tick.0 + move_duration, move_event);

                            commands.entity(*actor).insert(EventInProgress {
                                event_id: move_map_event.event_id,
                            });
                        } else {
                            info!("No path found");
                        }
                    }
                }

                *state = ActionState::Success;
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

pub fn move_to_pos_action_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    map: Res<Map>,
    mut map_events: ResMut<MapEvents>,
    templates: Res<Templates>,
    mut transport_query: Query<&mut Transport, Without<EventInProgress>>,
    mut npc_query: Query<CombatQuery>,
    mut query: Query<(&Actor, &mut ActionState, &MoveToPos)>,
) {
    for (Actor(actor), mut state, move_to_pos) in &mut query {
        info!("Processing MoveToPos Action...");
        let Ok(mut transport) = transport_query.get_mut(*actor) else {
            info!("Cannot find transport without EventInProgress");
            continue;
        };

        match *state {
            ActionState::Requested => {
                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                info!("Executing MoveToTarget Action");

                let Ok(mut npc) = npc_query.get_mut(*actor) else {
                    error!("Query failed to find entity {:?}", *actor);
                    *state = ActionState::Failure;
                    continue;
                };

                if npc.effects.0.contains_key(&Effect::Stunned) {
                    debug!("NPC is stunned");
                    continue;
                }

                // Get NPC speed
                let mut npc_speed = 1;

                if let Some(npc_base_speed) = npc.stats.base_speed {
                    npc_speed = npc_base_speed;
                }

                let effect_speed_mod = npc.effects.get_speed_effects(&templates);

                let move_duration = (BASE_MOVE_TICKS
                    * (BASE_SPEED / npc_speed as f32)
                    * (1.0 / effect_speed_mod)) as i32;

                if *npc.pos == move_to_pos.pos {
                } else {
                    if *npc.state == State::None {
                        if let Some(path_result) = Map::find_path(
                            *npc.pos,
                            move_to_pos.pos,
                            &map,
                            Vec::new(),
                            false,
                            true,
                            false,
                            false
                        ) {
                            info!("Follower path: {:?}", path_result);

                            let (path, c) = path_result;
                            let next_pos = &path[1];

                            info!("Next pos: {:?}", next_pos);

                            // Add State Change Event to Moving
                            let state_change_event = VisibleEvent::StateChangeEvent {
                                new_state: "moving".to_string(),
                            };

                            *npc.state = State::Moving;

                            map_events.new(npc.id.0, game_tick.0 + 4, state_change_event);

                            // Add Move Event
                            let move_event = VisibleEvent::MoveEvent {
                                dst_x: next_pos.0,
                                dst_y: next_pos.1,
                            };

                            let move_map_event =
                                map_events.new(npc.id.0, game_tick.0 + move_duration, move_event);

                            commands.entity(*actor).insert(EventInProgress {
                                event_id: move_map_event.event_id,
                            });
                        }
                    }
                }

                *state = ActionState::Success;
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

pub fn move_to_empire_action_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    map: Res<Map>,
    mut map_events: ResMut<MapEvents>,
    templates: Res<Templates>,
    mut transport_query: Query<&mut Transport, Without<EventInProgress>>,
    mut npc_query: Query<CombatQuery>,
    mut query: Query<(&Actor, &mut ActionState, &MoveToEmpire)>,
) {
    for (Actor(actor), mut state, move_to_pos) in &mut query {
        info!("Processing MoveToEmpire Action...");
        let Ok(mut transport) = transport_query.get_mut(*actor) else {
            info!("Cannot find transport without EventInProgress");
            continue;
        };

        match *state {
            ActionState::Requested => {
                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                info!("Executing MoveToTarget Action");

                let Ok(mut npc) = npc_query.get_mut(*actor) else {
                    error!("Query failed to find entity {:?}", *actor);
                    *state = ActionState::Failure;
                    continue;
                };

                if npc.effects.0.contains_key(&Effect::Stunned) {
                    debug!("NPC is stunned");
                    continue;
                }

                let empire_pos = Position { x: 16, y: 40 };

                // Get NPC speed
                let mut npc_speed = 1;

                if let Some(npc_base_speed) = npc.stats.base_speed {
                    npc_speed = npc_base_speed;
                }

                let effect_speed_mod = npc.effects.get_speed_effects(&templates);

                let move_duration = (BASE_MOVE_TICKS
                    * (BASE_SPEED / npc_speed as f32)
                    * (1.0 / effect_speed_mod)) as i32;

                if *npc.pos == empire_pos{
                } else {
                    if *npc.state == State::None {
                        if let Some(path_result) = Map::find_path(
                            *npc.pos,
                            empire_pos,
                            &map,
                            Vec::new(),
                            false,
                            true,
                            false,
                            false
                        ) {
                            info!("Follower path: {:?}", path_result);

                            let (path, c) = path_result;
                            let next_pos = &path[1];

                            info!("Next pos: {:?}", next_pos);

                            // Add State Change Event to Moving
                            let state_change_event = VisibleEvent::StateChangeEvent {
                                new_state: "moving".to_string(),
                            };

                            *npc.state = State::Moving;

                            map_events.new(npc.id.0, game_tick.0 + 4, state_change_event);

                            // Add Move Event
                            let move_event = VisibleEvent::MoveEvent {
                                dst_x: next_pos.0,
                                dst_y: next_pos.1,
                            };

                            let move_map_event =
                                map_events.new(npc.id.0, game_tick.0 + move_duration, move_event);

                            commands.entity(*actor).insert(EventInProgress {
                                event_id: move_map_event.event_id,
                            });
                        }
                    }
                }

                *state = ActionState::Success;
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

pub fn forfeiture_action_system(
    game_tick: Res<GameTick>,
    mut items: ResMut<Items>,
    ids: ResMut<Ids>,
    mut map_events: ResMut<MapEvents>,
    mut collector_query: Query<(&Id, &mut TaxCollector)>,
    mut query: Query<(&Actor, &mut ActionState, &Forfeiture, &ActionSpan)>,
) {
    for (Actor(actor), mut state, _forfeiture, span) in &mut query {
        match *state {
            ActionState::Requested => {
                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                let Ok((collector_id, mut collector)) = collector_query.get_mut(*actor) else {
                    continue;
                };

                // Get hero id from player
                let Some(hero_id) = ids.get_hero(collector.target_player) else {
                    error!("Cannot find hero for player {:?}", collector.target_player);
                    *state = ActionState::Failure;
                    continue;
                };

                // Get hero items
                let total_gold = items.get_total_gold(hero_id);
                info!(
                    "Total gold: {:?} collection_amount {:?}",
                    total_gold, collector.collection_amount
                );

                let overdue_amount = (collector.collection_amount as f32 * 1.2) as i32;

                if total_gold >= overdue_amount {
                    items.transfer_gold(hero_id, collector_id.0, overdue_amount);

                    collector.collection_amount = 0;
                    collector.last_collection_time = game_tick.0;

                    let sound_event = VisibleEvent::SoundObjEvent {
                        sound: "Times up! I will take what you owe and 20% extra.".to_string(),
                        intensity: 2,
                    };

                    map_events.new(collector_id.0, game_tick.0 + 4, sound_event);
                } else {
                    let remainder_gold = collector.collection_amount - total_gold;

                    collector.debt_amount = (remainder_gold as f32 * 1.5) as i32;
                    collector.collection_amount = 0;
                    collector.last_collection_time = game_tick.0;

                    let sound_event = VisibleEvent::SoundObjEvent {
                        sound: format!(
                            "Typical broke serf, add {} to his debt for next season!",
                            remainder_gold
                        ),
                        intensity: 2,
                    };

                    map_events.new(collector_id.0, game_tick.0 + 4, sound_event);
                }

                *state = ActionState::Success;
            }
            ActionState::Cancelled => {
                *state = ActionState::Failure;
            }
            _ => {}
        }
    }
}
