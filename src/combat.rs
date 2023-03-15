use bevy::ecs::query::WorldQuery;
use bevy::prelude::*;

use std::collections::HashMap;

use rand::Rng;

use crate::ai::VisibleTarget;
use crate::game::{
    BaseAttrs, Class, Id, MapEvent, Misc, PlayerId, Position, State, Stats, Subclass, Template,
    VisibleEvent, VisibleEvents, DEAD, MapEvents,
};
use crate::network;
use crate::skill::{Skill, Skills};
use crate::templates::{SkillTemplate, SkillTemplates};

#[derive(WorldQuery)]
#[world_query(mutable, derive(Debug))]
pub struct CombatQuery {
    pub entity: Entity,
    pub id: &'static Id,
    pub player_id: &'static PlayerId,
    pub pos: &'static Position,
    pub class: &'static Class,
    pub subclass: &'static Subclass,
    pub template: &'static Template,
    pub state: &'static mut State,
    pub misc: &'static Misc,
    pub stats: &'static mut Stats,
}

#[derive(Debug, Clone)]
pub struct Combat;

impl Combat {
    pub fn process_damage(
        attack_type: String,
        attacker: &CombatQueryItem,
        target: &mut CombatQueryItem,
        commands: &mut Commands
    ) -> i32 {
        let damage_range = attacker.stats.damage_range.unwrap();
        let base_damage = attacker.stats.base_damage.unwrap();

        //TODO get item and weapons

        //TODO get equiped weapons

        //TODO get effect modifications

        let mut rng = rand::thread_rng();

        let roll_damage = rng.gen_range(0..damage_range) + base_damage;

        target.stats.hp -= roll_damage;

        if target.stats.hp <= 0 {
            target.state.0 = DEAD.to_string();
            commands.entity(target.entity).despawn();
        }

        return roll_damage;
    }

    pub fn add_damage_event(
        event_id: i32,
        game_tick: i32,
        attack_type: String,
        damage: i32,
        attacker: &CombatQueryItem,
        target: &CombatQueryItem,
        map_events: &mut ResMut<MapEvents>,
    ) {
        let damage_event = VisibleEvent::DamageEvent {
            target_id: target.id.0,
            target_pos: target.pos.clone(),
            attack_type: attack_type.clone(),
            damage: damage,
            state: target.state.0.clone(),
        };

        map_events.new(
            event_id,
            attacker.entity,
            attacker.id,
            attacker.player_id,
            attacker.pos,
            game_tick,
            damage_event,
        );

    }
}
