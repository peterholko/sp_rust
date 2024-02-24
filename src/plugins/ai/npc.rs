use std::f32::consts::E;

use bevy::prelude::*;
use big_brain::prelude::*;
use rand::Rng;

use crate::combat::AttackType;
use crate::combat::Combat;
use crate::combat::CombatQuery;
use crate::components::npc::ChaseAndCast;
use crate::components::npc::FleeScorer;
use crate::components::npc::FleeToHome;
use crate::components::npc::MerchantScorer;
use crate::components::npc::RaiseDead;
use crate::components::npc::SailToPort;
use crate::components::npc::VisibleCorpse;
use crate::components::npc::VisibleCorpseScorer;
use crate::components::npc::{ChaseAndAttack, VisibleTarget, VisibleTargetScorer};
use crate::components::villager::MoveToInProgress;
use crate::effect::Effect;
use crate::game::State;
use crate::game::*;
use crate::item::*;
use crate::map::Map;
use crate::obj;
use crate::obj::Obj;
use crate::templates::Templates;

pub const NO_TARGET: i32 = -1;
pub const BASE_MOVE_TICKS: f32 = 100.0;
pub const BASE_SPEED: f32 = 1.0;

pub const NECROMANCER: &str = "Necromancer";

pub fn target_scorer_system(
    target_query: Query<&VisibleTarget>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<VisibleTargetScorer>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        if let Ok(target) = target_query.get(*actor) {
            println!("Scorer target_id: {:?}", target);
            if target.target != NO_TARGET {
                score.set(0.9);
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
            let mut min_distance = u32::MAX;
            let mut target_id = NO_TARGET;

            for target in target_query.iter() {
                // Skip dead targets
                if Obj::is_dead(target.state) {
                    continue;
                }

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
    mut query: Query<(&Actor, &mut ActionState, &ChaseAndAttack)>,
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

                // NPC is stunned, skip execution
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
                info!("NPC move duration: {:?}", move_duration);

                if target_id == NO_TARGET {
                    debug!("No target to chase, start wandering...");
                    let wander_pos_list = Map::get_neighbour_tiles(
                        npc.pos.x,
                        npc.pos.y,
                        &map,
                        &Vec::new(),
                        true,
                        false,
                        false,
                    );

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

                            let move_map_event = map_events.new(
                                *actor,
                                npc.id,
                                npc.player_id,
                                npc.pos,
                                game_tick.0 + move_duration, // in the future
                                move_event,
                            );

                            commands.entity(*actor).insert(EventInProgress {
                                event_id: move_map_event.event_id,
                            });

                            commands.entity(*actor).insert(MoveToInProgress);
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
                        let (damage, combo, _skill_gain) = Combat::process_attack(
                            AttackType::Quick,
                            &mut npc,
                            &mut target,
                            &mut commands,
                            &mut items,
                            &templates,
                            &map,
                            &mut ids,
                            &game_tick,
                            &mut map_events,
                        );

                        // Add visible damage event to broadcast to everyone nearby
                        Combat::add_damage_event(
                            game_tick.0,
                            "quick".to_string(),
                            damage,
                            combo,
                            &npc,
                            &target,
                            &mut map_events,
                        );

                        // Add Cooldown Event
                        let cooldown_event = VisibleEvent::CooldownEvent { duration: 30 };

                        let cooldown_map_event = map_events.new(
                            *actor,
                            npc.id,
                            npc.player_id,
                            npc.pos,
                            game_tick.0 + 30, // in the future
                            cooldown_event,
                        );

                        commands.entity(*actor).insert(EventInProgress {
                            event_id: cooldown_map_event.event_id,
                        });
                    } else {
                        if *npc.state == State::None {
                            if let Some(path_result) = Map::find_path(
                                *npc.pos,
                                *target.pos,
                                &map,
                                &Vec::new(),
                                true,
                                false,
                                false,
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

                                let move_map_event = map_events.new(
                                    *actor,
                                    npc.id,
                                    npc.player_id,
                                    npc.pos,
                                    game_tick.0 + move_duration,
                                    move_event,
                                );

                                commands.entity(*actor).insert(EventInProgress {
                                    event_id: move_map_event.event_id,
                                });
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

//Necromancer systems

pub fn corpses_scorer_system(
    corpse_query: Query<&VisibleCorpse>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<VisibleCorpseScorer>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        if let Ok(corpse) = corpse_query.get(*actor) {
            println!("Scorer corpse_id: {:?}", corpse);
            if corpse.corpse != NO_TARGET {
                score.set(1.0);
            } else {
                score.set(0.0);
            }
        }
    }
}

pub fn nearby_corpses_system(
    game_tick: Res<GameTick>,
    mut npc_query: Query<(&Position, &Viewshed, &mut VisibleCorpse), With<SubclassNPC>>,
    target_query: Query<ObjQuery>,
) {
    if game_tick.0 % 30 == 0 {
        for (npc_pos, npc_viewshed, mut visible_corpse) in npc_query.iter_mut() {
            let mut min_distance = u32::MAX;
            let mut corpse_id = NO_TARGET;

            for target in target_query.iter() {
                if target.class.0 == obj::CLASS_CORPSE.to_string() {
                    let distance = Map::dist(*npc_pos, *target.pos);

                    if npc_viewshed.range >= distance {
                        if distance < min_distance {
                            min_distance = distance;
                            corpse_id = target.id.0;
                        }
                    }
                }
            }

            debug!("Distance system corpse_id: {:?}", corpse_id);
            visible_corpse.corpse = corpse_id;
        }
    }
}

pub fn flee_scorer_system(
    minions_query: Query<&Minions>,
    state_query: Query<&State>,
    ids: Res<Ids>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<FleeScorer>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        if let Ok(minions) = minions_query.get(*actor) {
            let mut minions_alive = true;

            for minion_id in minions.ids.iter() {
                println!("Minion_id: {:?}", minion_id);

                let Some(minion_entity) = ids.get_entity(*minion_id) else {
                    continue;
                };

                if let Ok(minion_state) = state_query.get(minion_entity) {
                    if *minion_state == State::Dead {
                        minions_alive = false;
                    }
                }
            }

            if !minions_alive {
                println!("All minions dead");
                score.set(0.95);
            }
        }
    }
}

pub fn cast_target_system(
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
    mut query: Query<(&Actor, &mut ActionState, &ChaseAndCast)>,
) {
    for (Actor(actor), mut state, chase_and_cast) in &mut query {
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

                // NPC is stunned, skip execution
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
                info!("NPC move duration: {:?}", move_duration);

                if target_id == NO_TARGET {
                    /*debug!("No target to chase, start wandering...");
                    let wander_pos_list = Map::get_neighbour_tiles(npc.pos.x, npc.pos.y, &map, &Vec::new(), true, false, false);

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

                            let move_map_event = map_events.new(
                                *actor,
                                npc.id,
                                npc.player_id,
                                npc.pos,
                                game_tick.0 + move_duration, // in the future
                                move_event,
                            );

                            commands
                                .entity(*actor)
                                .insert(EventInProgress { event_id: move_map_event.event_id });

                            commands.entity(*actor).insert(MoveToInProgress);
                        }
                    }*/
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

                    if Map::dist(*npc.pos, *target.pos) <= 2 {
                        debug!("Target is in range, time to cast spell");

                        *npc.state = State::Casting;

                        map_events.new(
                            *actor,
                            npc.id,
                            npc.player_id,
                            npc.pos,
                            game_tick.0 + 4,
                            VisibleEvent::StateChangeEvent {
                                new_state: "casting".to_string(),
                            },
                        );

                        let spell_damage_event = VisibleEvent::SpellDamageEvent {
                            spell: Spell::ShadowBolt,
                            target_id: target.id.0,
                        };

                        let map_event = map_events.new(
                            *actor,
                            npc.id,
                            npc.player_id,
                            npc.pos,
                            game_tick.0 + 30,
                            spell_damage_event,
                        );

                        commands.entity(*actor).insert(EventInProgress {
                            event_id: map_event.event_id,
                        });
                    } else {
                        if *npc.state == State::None {
                            if let Some(path_result) = Map::find_path(
                                *npc.pos,
                                *target.pos,
                                &map,
                                &Vec::new(),
                                true,
                                false,
                                false,
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

                                let move_map_event = map_events.new(
                                    *actor,
                                    npc.id,
                                    npc.player_id,
                                    npc.pos,
                                    game_tick.0 + move_duration,
                                    move_event,
                                );

                                commands.entity(*actor).insert(EventInProgress {
                                    event_id: move_map_event.event_id,
                                });
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

pub fn raise_dead_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    mut ids: ResMut<Ids>,
    map: Res<Map>,
    mut game_events: ResMut<GameEvents>,
    mut map_events: ResMut<MapEvents>,
    mut items: ResMut<Items>,
    templates: Res<Templates>,
    visible_corpse_query: Query<&VisibleCorpse>,
    obj_query: Query<(&Id, &PlayerId, &Position), Without<EventInProgress>>,
    mut state_query: Query<&mut State>,
    mut query: Query<(&Actor, &mut ActionState, &RaiseDead)>,
) {
    for (Actor(actor), mut state, raise_dead) in &mut query {
        let Ok(visible_corpse) = visible_corpse_query.get(*actor) else {
            continue;
        };

        println!("Visible corpse: {:?}", visible_corpse);

        match *state {
            ActionState::Requested => {
                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                let corpse_id = visible_corpse.corpse;

                // Get target entity
                let Some(corpse_entity) = ids.get_entity(corpse_id) else {
                    *state = ActionState::Failure;
                    error!("Cannot find target entity for {:?}", corpse_id);
                    continue;
                };

                let Ok((_id, corpse_player, corpse_pos)) = obj_query.get(corpse_entity) else {
                    error!("Query failed to find entity {:?}", corpse_entity);
                    *state = ActionState::Failure;
                    continue;
                };

                let Ok((npc_id, npc_player_id, npc_pos)) = obj_query.get(*actor) else {
                    error!("Query failed to find entity {:?}", *actor);
                    *state = ActionState::Failure;
                    continue;
                };

                let Ok(mut npc_state) = state_query.get_mut(*actor) else {
                    error!("Query failed to find entity {:?}", *actor);
                    *state = ActionState::Failure;
                    continue;
                };

                if Map::is_adjacent(*npc_pos, *corpse_pos) {
                    println!("Corpse is adjacent, time to raise the dead");

                    *npc_state = State::Casting;

                    map_events.new(
                        *actor,
                        npc_id,
                        npc_player_id,
                        npc_pos,
                        game_tick.0 + 4,
                        VisibleEvent::StateChangeEvent {
                            new_state: "casting".to_string(),
                        },
                    );

                    let map_event_id = map_events.add(
                        VisibleEvent::SpellRaiseDeadEvent { corpse_id: corpse_id },
                        *actor,
                        npc_id.0,
                        npc_player_id.0,
                        npc_pos.x,
                        npc_pos.y,
                        game_tick.0 + 30,
                    );

                    commands.entity(*actor).insert(EventInProgress {
                        event_id: map_event_id,
                    });
                } else {
                    if *npc_state == State::None {
                        if let Some(path_result) = Map::find_path(
                            *npc_pos,
                            *corpse_pos,
                            &map,
                            &Vec::new(),
                            true,
                            false,
                            false,
                        ) {
                            debug!("Follower path: {:?}", path_result);

                            let (path, c) = path_result;
                            let next_pos = &path[1];

                            debug!("Next pos: {:?}", next_pos);

                            // Add State Change Event to Moving
                            let state_change_event = VisibleEvent::StateChangeEvent {
                                new_state: "moving".to_string(),
                            };

                            *npc_state = State::Moving;

                            map_events.new(
                                *actor,
                                npc_id,
                                npc_player_id,
                                npc_pos,
                                game_tick.0 + 4,
                                state_change_event,
                            );

                            // Add Move Event
                            let move_event = VisibleEvent::MoveEvent {
                                dst_x: next_pos.0,
                                dst_y: next_pos.1,
                            };

                            let move_map_event = map_events.new(
                                *actor,
                                npc_id,
                                npc_player_id,
                                npc_pos,
                                game_tick.0 + 10,
                                move_event,
                            );

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

pub fn flee_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    map: Res<Map>,
    mut map_events: ResMut<MapEvents>,
    mut obj_query: Query<ObjQuery, Without<EventInProgress>>,
    home_query: Query<&Home>,
    //mut merchant_query: Query<(ObjQuery, &mut Merchant), (With<Merchant>, Without<EventInProgress>)>,
    mut query: Query<(&Actor, &mut ActionState, &FleeToHome)>,
) {
    for (Actor(actor), mut state, _flee_to_home) in &mut query {
        debug!("actor: {:?}", actor);
        match *state {
            ActionState::Requested => {
                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                println!("NPC fleeing...");

                let Ok(mut obj) = obj_query.get_mut(*actor) else {
                    //debug!("Obj query failed to find actor: {:?}", actor);
                    continue;
                };

                let Ok(home) = home_query.get(*actor) else {
                    //debug!("Merchant querny failed to find actor: {:?}", actor);
                    continue;
                };

                if *obj.pos == home.pos {
                    commands.entity(*actor).remove::<MoveToInProgress>();
                    *state = ActionState::Success;
                } else {
                    println!("Finding path from {:?} to {:?}", obj.pos, home.pos);

                    if let Some(path_result) =
                        Map::find_path(*obj.pos, home.pos, &map, &Vec::new(), true, false, false)
                    {
                        println!("Follower path: {:?}", path_result);

                        let (path, c) = path_result;
                        let next_pos = &path[1];

                        debug!("Next pos: {:?}", next_pos);

                        // Add State Change Event to Moving
                        let state_change_event = VisibleEvent::StateChangeEvent {
                            new_state: "moving".to_string(),
                        };

                        *obj.state = State::Moving;

                        map_events.new(
                            *actor,
                            obj.id,
                            obj.player_id,
                            obj.pos,
                            game_tick.0 + 4,
                            state_change_event,
                        );

                        // Add Move Event
                        let move_event = VisibleEvent::MoveEvent {
                            dst_x: next_pos.0,
                            dst_y: next_pos.1,
                        };

                        let move_map_event = map_events.new(
                            *actor,
                            obj.id,
                            obj.player_id,
                            obj.pos,
                            game_tick.0 + 36, // in the future
                            move_event,
                        );

                        commands.entity(*actor).insert(EventInProgress {
                            event_id: move_map_event.event_id,
                        });

                        commands.entity(*actor).insert(MoveToInProgress);
                    } else {
                        println!("Cannot find path");
                        *state = ActionState::Failure;
                    }
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

pub fn merchant_scorer_system(
    game_tick: Res<GameTick>,
    move_in_progress: Query<&MoveToInProgress>,
    mut pos_query: Query<(&Position, &mut Merchant)>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<MerchantScorer>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        if let Ok(_move_in_progress) = move_in_progress.get(*actor) {
            score.set(1.0);
        } else {
            if let Ok((position, mut merchant)) = pos_query.get_mut(*actor) {
                if *position == merchant.home_port {
                    if (merchant.in_port_at + 50) <= game_tick.0 {
                        // destination to target port
                        merchant.dest = merchant.target_port;

                        score.set(1.0);
                    } else {
                        score.set(0.0);
                    }
                } else if *position == merchant.target_port {
                    if (merchant.in_port_at + 500) <= game_tick.0 {
                        // destination to home port
                        merchant.dest = merchant.home_port;

                        score.set(1.0);
                    } else {
                        score.set(0.0);
                    }
                } else {
                    score.set(0.0);
                }

                debug!("merchant score: {:?}", score);
            }
        }
    }
}

pub fn merchant_move_system(
    mut commands: Commands,
    game_tick: Res<GameTick>,
    mut ids: ResMut<Ids>,
    map: Res<Map>,
    mut map_events: ResMut<MapEvents>,
    mut obj_query: Query<ObjQuery, (With<Merchant>, Without<EventInProgress>)>,
    mut merchant_query: Query<&mut Merchant>,
    //mut merchant_query: Query<(ObjQuery, &mut Merchant), (With<Merchant>, Without<EventInProgress>)>,
    mut query: Query<(&Actor, &mut ActionState, &SailToPort)>,
) {
    for (Actor(actor), mut state, sail_to_port) in &mut query {
        debug!("actor: {:?}", actor);
        match *state {
            ActionState::Requested => {
                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                debug!("Sail to port executing...");

                debug!("Getting obj...");
                let Ok(mut obj) = obj_query.get_mut(*actor) else {
                    //debug!("Obj query failed to find actor: {:?}", actor);
                    continue;
                };

                let Ok(mut merchant) = merchant_query.get_mut(*actor) else {
                    //debug!("Merchant querny failed to find actor: {:?}", actor);
                    continue;
                };

                if *obj.pos == merchant.dest {
                    merchant.in_port_at = game_tick.0;
                    commands.entity(*actor).remove::<MoveToInProgress>();
                    *state = ActionState::Success;
                } else {
                    debug!("Finding path from {:?} to {:?}", obj.pos, merchant.dest);

                    if let Some(path_result) = Map::find_path(
                        *obj.pos,
                        merchant.dest,
                        &map,
                        &Vec::new(),
                        false,
                        true,
                        false,
                    ) {
                        debug!("Follower path: {:?}", path_result);

                        let (path, c) = path_result;
                        let next_pos = &path[1];

                        debug!("Next pos: {:?}", next_pos);

                        // Add State Change Event to Moving
                        let state_change_event = VisibleEvent::StateChangeEvent {
                            new_state: "moving".to_string(),
                        };

                        *obj.state = State::Moving;

                        map_events.new(
                            *actor,
                            obj.id,
                            obj.player_id,
                            obj.pos,
                            game_tick.0 + 4,
                            state_change_event,
                        );

                        // Add Move Event
                        let move_event = VisibleEvent::MoveEvent {
                            dst_x: next_pos.0,
                            dst_y: next_pos.1,
                        };

                        let move_map_event = map_events.new(
                            *actor,
                            obj.id,
                            obj.player_id,
                            obj.pos,
                            game_tick.0 + 36, // in the future
                            move_event,
                        );

                        commands.entity(*actor).insert(EventInProgress {
                            event_id: move_map_event.event_id,
                        });

                        commands.entity(*actor).insert(MoveToInProgress);
                    } else {
                        debug!("Cannot find path");
                        *state = ActionState::Failure;
                    }
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
