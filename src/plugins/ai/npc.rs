use bevy::prelude::*;
use big_brain::prelude::*;
use rand::Rng;

use crate::combat::AttackType;
use crate::combat::Combat;
use crate::combat::CombatQuery;
use crate::components::npc::ChaseAndCast;
use crate::components::npc::FleeScorer;
use crate::components::npc::FleeToHome;
use crate::components::npc::Hide;
use crate::components::npc::MerchantScorer;
use crate::components::npc::RaiseDead;
use crate::components::npc::SailToPort;
use crate::components::npc::VisibleCorpse;
use crate::components::npc::VisibleCorpseScorer;
use crate::components::npc::{ChaseAndAttack, VisibleTarget, VisibleTargetScorer};
use crate::components::villager::MoveToInProgress;
use crate::effect::Effect;
use crate::event::Spell;
use crate::event::{GameEvents, MapEvents, VisibleEvent};
use crate::game::State;
use crate::game::*;
use crate::ids::Ids;
use crate::item::*;
use crate::map::Map;
use crate::map::MapPos;
use crate::obj;
use crate::obj::Obj;
use crate::templates::Templates;

pub const INIT_TARGET: i32 = -2;
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
    if game_tick.0 % 30 == 0 {
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
                        false,
                        MapPos(npc.pos.x, npc.pos.y),
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

                            map_events.new(npc.id.0, game_tick.0 + 4, state_change_event);

                            // Add Move Event
                            let move_event = VisibleEvent::MoveEvent {
                                src: *npc.pos,
                                dst: Position {
                                    x: random_pos_x,
                                    y: random_pos_y,
                                },
                            };

                            let move_map_event = map_events.new(
                                npc.id.0,
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
                            npc.id.0,
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
                                Vec::new(),
                                true,
                                false,
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

                                map_events.new(npc.id.0, game_tick.0 + 4, state_change_event);

                                // Add Move Event
                                let move_event = VisibleEvent::MoveEvent {
                                    src: *npc.pos,
                                    dst: Position {
                                        x: next_pos.0,
                                        y: next_pos.1,
                                    },
                                };

                                let move_map_event = map_events.new(
                                    npc.id.0,
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

// Necromancer systems
pub fn corpses_scorer_system(
    corpse_query: Query<&VisibleCorpse>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<VisibleCorpseScorer>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        if let Ok(corpse) = corpse_query.get(*actor) {
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

            visible_corpse.corpse = corpse_id;
        }
    }
}

pub fn flee_scorer_system(
    game_tick: Res<GameTick>,
    minions_query: Query<&Minions>,
    state_query: Query<&State>,
    ids: Res<Ids>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<FleeScorer>>,
) {
    // To prevent flee before raise dead
    if game_tick.0 % 30 == 0 {
        for (Actor(actor), mut score, span) in &mut query {
            if let Ok(minions) = minions_query.get(*actor) {
                let mut minions_dead = true;

                for minion_id in minions.ids.iter() {
                    let Some(minion_entity) = ids.get_entity(*minion_id) else {
                        continue;
                    };

                    if let Ok(minion_state) = state_query.get(minion_entity) {
                        if *minion_state != State::Dead {
                            minions_dead = false;
                        }
                    }
                }

                if minions_dead {
                    score.set(0.95);
                } else {
                    score.set(0.0);
                }
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
    mut query: Query<(&Actor, &mut ActionState, &mut ChaseAndCast)>,
) {
    for (Actor(actor), mut state, mut chase_and_cast) in &mut query {
        let Ok(visible_target) = visible_target_query.get(*actor) else {
            continue;
        };

        match *state {
            ActionState::Requested => {
                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                let target_id = visible_target.target;

                let Ok(mut npc) = npc_query.get_mut(*actor) else {
                    continue;
                };

                if game_tick.0 - chase_and_cast.start_time > 30 {
                    info!("Spell completed");
                    *state = ActionState::Success;
                    continue;
                }

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

                if target_id != NO_TARGET {
                    // Get target entity
                    let Some(target_entity) = ids.get_entity(target_id) else {
                        continue;
                    };

                    let Ok(mut target) = target_query.get_mut(target_entity) else {
                        continue;
                    };

                    let target_dist = Map::dist(*npc.pos, *target.pos);

                    if target_dist == 2 {
                        info!("Target is in range, time to cast spell");

                        // Shout spell
                        let sound_event = VisibleEvent::SoundObjEvent {
                            sound: "Wis An Ben!".to_string(),
                            intensity: 2,
                        };

                        map_events.new(npc.id.0, game_tick.0 + 4, sound_event);

                        *npc.state = State::Casting;

                        map_events.new(
                            npc.id.0,
                            game_tick.0 + 1,
                            VisibleEvent::StateChangeEvent {
                                new_state: "casting".to_string(),
                            },
                        );

                        let spell_damage_event = VisibleEvent::SpellDamageEvent {
                            spell: Spell::ShadowBolt,
                            target_id: target.id.0,
                        };

                        let map_event =
                            map_events.new(npc.id.0, game_tick.0 + 30, spell_damage_event);

                        commands.entity(*actor).insert(EventInProgress {
                            event_id: map_event.event_id,
                        });

                        // Set start time of action
                        chase_and_cast.start_time = game_tick.0;
                    } else if target_dist > 2 {
                        if *npc.state == State::None {
                            if let Some(path_result) = Map::find_path(
                                *npc.pos,
                                *target.pos,
                                &map,
                                Vec::new(),
                                true,
                                false,
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

                                map_events.new(npc.id.0, game_tick.0 + 4, state_change_event);

                                // Add Move Event
                                let move_event = VisibleEvent::MoveEvent {
                                    src: *npc.pos,
                                    dst: Position {
                                        x: next_pos.0,
                                        y: next_pos.1,
                                    },
                                };

                                let move_map_event = map_events.new(
                                    npc.id.0,
                                    game_tick.0 + move_duration,
                                    move_event,
                                );

                                commands.entity(*actor).insert(EventInProgress {
                                    event_id: move_map_event.event_id,
                                });
                            }
                        }
                    } else if target_dist == 1 {
                        let neighbour_tiles = Map::get_neighbour_tiles(
                            npc.pos.x,
                            npc.pos.y,
                            &map,
                            &Vec::new(),
                            true,
                            false,
                            false,
                            false,
                            MapPos(npc.pos.x, npc.pos.y),
                        );

                        println!("neighbour tiles: {:?}", neighbour_tiles);

                        let mut selected_pos_list = Vec::new();

                        for (map_pos, movement_cost) in neighbour_tiles.iter() {
                            let dist = Map::dist(
                                Position {
                                    x: map_pos.0,
                                    y: map_pos.1,
                                },
                                *target.pos,
                            );

                            if dist == 2 {
                                selected_pos_list.push(map_pos.clone());
                            }
                        }

                        println!("selected_pos_list: {:?}", selected_pos_list);

                        if selected_pos_list.len() > 0 {
                            // Randomly select a pos from list
                            let mut rng = rand::thread_rng();
                            let next_pos = selected_pos_list
                                [rng.gen_range(0..selected_pos_list.len())]
                            .clone();

                            // Add State Change Event to Moving
                            let state_change_event = VisibleEvent::StateChangeEvent {
                                new_state: "moving".to_string(),
                            };

                            *npc.state = State::Moving;

                            map_events.new(npc.id.0, game_tick.0 + 4, state_change_event);

                            // Add Move Event
                            let move_event = VisibleEvent::MoveEvent {
                                src: *npc.pos,
                                dst: Position {
                                    x: next_pos.0,
                                    y: next_pos.1,
                                },
                            };

                            let move_map_event =
                                map_events.new(npc.id.0, game_tick.0 + move_duration, move_event);

                            commands.entity(*actor).insert(EventInProgress {
                                event_id: move_map_event.event_id,
                            });
                        } else {
                            // No choice but has to fight

                            // Shout spell
                            let sound_event = VisibleEvent::SoundObjEvent {
                                sound: "Wis An Ben!".to_string(),
                                intensity: 2,
                            };

                            map_events.new(npc.id.0, game_tick.0 + 4, sound_event);

                            *npc.state = State::Casting;

                            map_events.new(
                                npc.id.0,
                                game_tick.0 + 1,
                                VisibleEvent::StateChangeEvent {
                                    new_state: "casting".to_string(),
                                },
                            );

                            let spell_damage_event = VisibleEvent::SpellDamageEvent {
                                spell: Spell::ShadowBolt,
                                target_id: target.id.0,
                            };

                            let map_event =
                                map_events.new(npc.id.0, game_tick.0 + 30, spell_damage_event);

                            commands.entity(*actor).insert(EventInProgress {
                                event_id: map_event.event_id,
                            });

                            // Set start time of action
                            chase_and_cast.start_time = game_tick.0;
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
    mut map_events: ResMut<MapEvents>,
    visible_corpse_query: Query<&VisibleCorpse>,
    obj_query: Query<(&Id, &PlayerId, &Position), Without<EventInProgress>>,
    mut state_query: Query<&mut State>,
    mut query: Query<(&Actor, &mut ActionState, &mut RaiseDead)>,
) {
    for (Actor(actor), mut state, mut raise_dead) in &mut query {
        let Ok(visible_corpse) = visible_corpse_query.get(*actor) else {
            continue;
        };

        match *state {
            ActionState::Requested => {
                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                let corpse_id = visible_corpse.corpse;

                // Get target entity
                let Some(corpse_entity) = ids.get_entity(corpse_id) else {
                    continue;
                };

                let Ok((_id, _corpse_player, corpse_pos)) = obj_query.get(corpse_entity) else {
                    continue;
                };

                let Ok((npc_id, _npc_player_id, npc_pos)) = obj_query.get(*actor) else {
                    continue;
                };

                let Ok(mut npc_state) = state_query.get_mut(*actor) else {
                    continue;
                };

                if game_tick.0 - raise_dead.start_time > 30 {
                    info!("Raise dead spell completed");
                    *state = ActionState::Success;
                    continue;
                }

                if Map::is_adjacent(*npc_pos, *corpse_pos) {
                    info!("Corpse is adjacent, time to raise the dead");

                    // Shout spell
                    let sound_event = VisibleEvent::SoundObjEvent {
                        sound: "Rise from the dead, Uus Corp!".to_string(),
                        intensity: 2,
                    };

                    map_events.new(npc_id.0, game_tick.0 + 4, sound_event);

                    *npc_state = State::Casting;

                    map_events.new(
                        npc_id.0,
                        game_tick.0 + 1,
                        VisibleEvent::StateChangeEvent {
                            new_state: "casting".to_string(),
                        },
                    );

                    let map_event_id = map_events.new(
                        npc_id.0,
                        game_tick.0 + 30,
                        VisibleEvent::SpellRaiseDeadEvent {
                            corpse_id: corpse_id,
                        },
                    );

                    commands.entity(*actor).insert(EventInProgress {
                        event_id: map_event_id.event_id,
                    });

                    // Set start time of action
                    raise_dead.start_time = game_tick.0;
                } else {
                    if *npc_state == State::None {
                        if let Some(path_result) = Map::find_path(
                            *npc_pos,
                            *corpse_pos,
                            &map,
                            Vec::new(),
                            true,
                            false,
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

                            map_events.new(npc_id.0, game_tick.0 + 4, state_change_event);

                            // Add Move Event
                            let move_event = VisibleEvent::MoveEvent {
                                src: *npc_pos,
                                dst: Position {
                                    x: next_pos.0,
                                    y: next_pos.1,
                                },
                            };

                            let move_map_event =
                                map_events.new(npc_id.0, game_tick.0 + 10, move_event);

                            commands.entity(*actor).insert(EventInProgress {
                                event_id: move_map_event.event_id,
                            });
                        }
                    } else {
                        info!("Failed to find path to corpse");
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
        match *state {
            ActionState::Requested => {
                let Ok(obj) = obj_query.get(*actor) else {
                    continue;
                };

                let sound_event = VisibleEvent::SoundObjEvent {
                    sound: "My minions fall, but I will get my revenge!".to_string(),
                    intensity: 2,
                };

                map_events.new(obj.id.0, game_tick.0 + 4, sound_event);

                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                let Ok(mut obj) = obj_query.get_mut(*actor) else {
                    //debug!("Obj query failed to find actor: {:?}", actor);
                    continue;
                };

                let Ok(home) = home_query.get(*actor) else {
                    //debug!("Merchant query failed to find actor: {:?}", actor);
                    continue;
                };

                if *obj.pos == home.pos {
                    commands.entity(*actor).remove::<MoveToInProgress>();
                    *state = ActionState::Success;
                } else {
                    println!("Finding path from {:?} to {:?}", obj.pos, home.pos);

                    if let Some(path_result) = Map::find_path(
                        *obj.pos,
                        home.pos,
                        &map,
                        Vec::new(),
                        true,
                        false,
                        false,
                        false,
                    ) {
                        println!("Follower path: {:?}", path_result);

                        let (path, c) = path_result;
                        let next_pos = &path[1];

                        debug!("Next pos: {:?}", next_pos);

                        // Add State Change Event to Moving
                        let state_change_event = VisibleEvent::StateChangeEvent {
                            new_state: "moving".to_string(),
                        };

                        *obj.state = State::Moving;

                        map_events.new(obj.id.0, game_tick.0 + 4, state_change_event);

                        // Add Move Event
                        let move_event = VisibleEvent::MoveEvent {
                            src: *obj.pos,
                            dst: Position {
                                x: next_pos.0,
                                y: next_pos.1,
                            },
                        };

                        let move_map_event = map_events.new(
                            obj.id.0,
                            game_tick.0 + 36, // in the future
                            move_event,
                        );

                        commands.entity(*actor).insert(EventInProgress {
                            event_id: move_map_event.event_id,
                        });

                        commands.entity(*actor).insert(MoveToInProgress);
                    } else {
                        error!("Cannot find path");
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

pub fn hide_action_system(
    game_tick: Res<GameTick>,
    mut map_events: ResMut<MapEvents>,
    obj_query: Query<&Id>,
    mut query: Query<(&Actor, &mut ActionState, &mut Hide, &ActionSpan)>,
) {
    for (Actor(actor), mut state, mut hide, span) in &mut query {
        match *state {
            ActionState::Requested => {
                *state = ActionState::Executing;
            }
            ActionState::Executing => {

                // Get Id from actor
                let Ok(obj_id) = obj_query.get(*actor) else {
                    continue;
                };

                map_events.new(
                    obj_id.0,
                    game_tick.0 + 1, // in the future
                    VisibleEvent::HideEvent,
                );

                *state = ActionState::Success;
            }
            ActionState::Cancelled => {
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
        score.set(1.0);
    }
}

