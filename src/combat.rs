use bevy::ecs::query::WorldQuery;
use bevy::prelude::*;
use big_brain::thinker::{Actor, ThinkerBuilder};

use rand::Rng;

use crate::game::{
    BaseAttrs, Class, Id, MapEvent, MapEvents, Misc, PlayerId, Position, State, Stats, Subclass,
    Template, VisibleEvent, VisibleEvents,
};
use crate::item::{Item, Items, DAMAGE};
use crate::obj::ObjUtil;
use crate::skill::{Skill, SkillUpdated, Skills};
use crate::templates::{ObjTemplate, SkillTemplate, SkillTemplates, Templates};
use crate::{network, obj};


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
        commands: &mut Commands,
        items: &mut ResMut<Items>,
        templates: &Res<Templates>,
    ) -> (i32, Option<SkillUpdated>) {
        let target_template = ObjTemplate::get_template(target.template.0.clone(), &templates);

        let damage_range = attacker.stats.damage_range.unwrap();
        let base_damage = attacker.stats.base_damage.unwrap();

        //TODO get item and weapons
        let attacker_items = Item::get_equipped(attacker.id.0, &items);
        debug!("Attacker Items: {:?}", attacker_items);
        let damage_from_items = Item::get_items_value_by_attr(DAMAGE, attacker_items);
        debug!("Damage From Items: {:?}", damage_from_items);

        //TODO get equiped weapons
        let attacker_weapons = Item::get_equipped_weapons(attacker.id.0, &items);

        //TODO get effect modifications

        let mut rng = rand::thread_rng();

        let roll_damage = (rng.gen_range(0..damage_range) + base_damage) as f32;
        let total_damage = (roll_damage + damage_from_items) as i32;

        target.stats.hp -= total_damage;

        let mut skill_updated = None;

        debug!("Target HP: {:?}", target.stats.hp);
        if target.stats.hp <= 0 {
            *target.state = State::Dead;
            commands.entity(target.entity).remove::<ThinkerBuilder>();
            //commands.entity(target.entity).despawn();

            for item in attacker_weapons.iter() {
                skill_updated = Some(SkillUpdated {
                    id: attacker.id.0,
                    xp_type: item.subclass.to_string(),
                    xp: target_template.kill_xp.unwrap_or(0),
                });
            }
        }

        debug!("Total Damage: {:?}", total_damage);

        return (total_damage, skill_updated);
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
        debug!("target state: {:?}", target.state);
        let target_state_str = ObjUtil::state_to_str(target.state.clone());
        debug!("target state str: {:?}", target_state_str);

        let damage_event = VisibleEvent::DamageEvent {
            target_id: target.id.0,
            target_pos: target.pos.clone(),
            attack_type: attack_type.clone(),
            damage: damage,
            state: target_state_str,
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
