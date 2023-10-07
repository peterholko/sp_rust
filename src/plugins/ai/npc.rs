use bevy::prelude::*;
use big_brain::prelude::*;
use rand::Rng;

use crate::combat::Combat;
use crate::combat::CombatQuery;
use crate::components::npc::{ChaseAttack, VisibleTarget, VisibleTargetScorer};
use crate::game::*;
use crate::game::State;    
use crate::item::*;
use crate::map::Map;
use crate::templates::Templates;

pub const NO_TARGET: i32 = -1;


pub fn target_scorer_system(
    target_query: Query<&VisibleTarget>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<VisibleTargetScorer>>,
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

pub fn attack_target_system(
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
                    let wander_pos_list = Map::get_neighbour_tiles(npc.pos.x, npc.pos.y, &map, &Vec::new());

                    if *npc.state == State::None {
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

                            *npc.state = State::Moving;

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
                            let event_id = ids.new_map_event_id();

                            map_events.new(
                                event_id,
                                *actor,
                                npc.id,
                                npc.player_id,
                                npc.pos,
                                game_tick.0 + 36, // in the future
                                move_event,
                            );

                            commands
                                .entity(*actor)
                                .insert(EventInProgress { event_id: event_id });
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
                            &templates,
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

                        let event_id = ids.new_map_event_id();

                        map_events.new(
                            event_id,
                            *actor,
                            npc.id,
                            npc.player_id,
                            npc.pos,
                            game_tick.0 + 30, // in the future
                            cooldown_event,
                        );

                        commands
                            .entity(*actor)
                            .insert(EventInProgress { event_id: event_id });
                    } else {
                        if *npc.state == State::None {
                            if let Some(path_result) = Map::find_path(
                                *npc.pos,
                                *target.pos,
                                &map,
                                &Vec::new()
                            ) {
                                debug!("Follower path: {:?}", path_result);

                                let (path, c) = path_result;
                                let next_pos = &path[1];

                                debug!("Next pos: {:?}", next_pos);

                                // Add State Change Event to Moving
                                let state_change_event = VisibleEvent::StateChangeEvent {
                                    new_state: "moving".to_string(),
                                };

                                *npc.state = State::Moving;

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

                                let event_id = ids.new_map_event_id();

                                map_events.new(
                                    event_id,
                                    *actor,
                                    npc.id,
                                    npc.player_id,
                                    npc.pos,
                                    game_tick.0 + 36, // in the future
                                    move_event,
                                );

                                commands
                                    .entity(*actor)
                                    .insert(EventInProgress { event_id: event_id });
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
