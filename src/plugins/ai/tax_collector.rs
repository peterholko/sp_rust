use bevy::prelude::*;
use big_brain::prelude::*;
use rand::Rng;

use crate::components::npc::{
    AtLanding, Disembark, Embark, Idle, InEmpire, IsAboardScorer, IsHeroNearby, IsTaxCollected, TaxCollector
};
use crate::event::{GameEvents, MapEvents, VisibleEvent};
use crate::game::State;
use crate::game::*;
use crate::ids::Ids;
use crate::item::*;
use crate::map::Map;
use crate::obj;
use crate::obj::Obj;
use crate::templates::Templates;

pub fn is_aboard_scorer_system(
    game_tick: Res<GameTick>,
    state_query: Query<&State, With<TaxCollector>>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<IsAboardScorer>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        if let Ok(state) = state_query.get(*actor) {
            if *state == State::Aboard {
                score.set(1.0);
            } else {
                score.set(0.0);
            }
        }
    }
}

pub fn is_tax_collected_scorer_system(
    game_tick: Res<GameTick>,
    state_query: Query<&State, With<TaxCollector>>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<IsTaxCollected>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        if let Ok(state) = state_query.get(*actor) {
            if *state == State::Aboard {
                score.set(1.0);
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
                score.set(1.0);
            } else {
                score.set(0.0);
            }
        }
    }
}

pub fn is_hero_nearby_scorer_system(
    game_tick: Res<GameTick>,
    ids: Res<Ids>,
    pos_query: Query<&Position>,
    collector_query: Query<&TaxCollector>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<IsHeroNearby>>,
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
 
            if Map::dist(*hero_pos, *tax_collector_pos) <= 2 {
                score.set(1.0);
            } else {
                score.set(0.0);
            }
        }
    }
}

pub fn idle_action_system(
    mut query: Query<(&Actor, &mut ActionState, &Idle, &ActionSpan)>,
) {
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

pub fn embark_action_system(
    game_tick: Res<GameTick>,
    mut map_events: ResMut<MapEvents>,    
    mut obj_query: Query<(&Id, &mut Position, &mut State)>,
    collector_query: Query<&TaxCollector>,
    mut query: Query<(&Actor, &mut ActionState, &Embark, &ActionSpan)>,
) {
    for (Actor(actor), mut state, _idle, span) in &mut query {

        match *state {
            ActionState::Requested => {
                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                let Ok((id, mut pos, mut obj_state)) = obj_query.get_mut(*actor) else {
                    error!("No position found for {:?}", actor);
                    continue;
                };

                let Ok(collector) = collector_query.get(*actor) else {
                    error!("No tax collector found for {:?}", actor);
                    continue;
                };

                map_events.new(
                    id.0,
                    game_tick.0 + 20, 
                    VisibleEvent::EmbarkEvent {transport_id: collector.transport_id}
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

pub fn disembark_action_system(
    mut map_events: ResMut<MapEvents>,
    mut obj_query: Query<(&mut Position, &mut State)>,
    mut query: Query<(&Actor, &mut ActionState, &Disembark, &ActionSpan)>,
) {
    for (Actor(actor), mut state, _idle, span) in &mut query {

        match *state {
            ActionState::Requested => {
                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                let Ok((mut pos, mut obj_state)) = obj_query.get_mut(*actor) else {
                    error!("No position found for {:?}", actor);
                    continue;
                };

                let map_event = VisibleEvent::EmbarkEvent { action: Disembark, transport_id: (), pos: () }

                map_events.new(obj_id, game_tick, map_event_type)

                *state = ActionState::Success;
            }            
            ActionState::Cancelled => {
                *state = ActionState::Failure;
            }
            _ => {}
        }
    }
}