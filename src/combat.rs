use bevy::ecs::query::WorldQuery;
use bevy::prelude::*;
use big_brain::thinker::{Actor, ThinkerBuilder};

use rand::Rng;
use std::collections::HashMap;

use crate::effect::{Effect, Effects, self};
use crate::game::{
    BaseAttrs, Class, Id, MapEvent, MapEvents, Misc, PlayerId, Position, State, Stats, Subclass,
    Template, VisibleEvent, VisibleEvents, Ids, GameTick
};
use crate::item::{self, Item, Items, DAMAGE};
use crate::obj::ObjUtil;
use crate::skill::{Skill, SkillUpdated, Skills};
use crate::templates::{EffectTemplate, ObjTemplate, SkillTemplate, SkillTemplates, Templates};

pub const QUICK: &str = "quick";
pub const PRECISE: &str = "precise";
pub const FIERCE: &str = "fierce";

pub const HAMSTRING: &str = "Hamstring";
pub const GOUGE: &str = "Gouge";

pub const TICKS_PER_SEC: i32 = 10;

#[derive(Debug, Clone, PartialEq)]
pub enum AttackType {
    Quick,
    Precise,
    Fierce,
}

impl AttackType {
    pub fn to_str(self) -> String {
        match self {
            AttackType::Quick => QUICK.to_string(),
            AttackType::Precise => PRECISE.to_string(),
            AttackType::Fierce => FIERCE.to_string(),
        }
    }
}

#[derive(Debug, Clone, Reflect)]
pub enum Combo {
    Hamstring,
    Gouge,
    ImtimidatingShout,
    ShroudedSlash,
    ShatterCleave,
    MassivePummel,
    NightmareStrike,
}

impl Combo {
    pub fn from_string(combo_string: &String) -> Self {
        match combo_string.as_str() {
            HAMSTRING => Combo::Hamstring,
            GOUGE => Combo::Gouge,
            //TODO finish the other combos
            _ => Combo::Hamstring,
        }
    }
}

#[derive(Debug, Component, Clone)]
pub struct ComboTracker {
    pub target_id: i32,
    pub attacks: Vec<AttackType>,
}

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
    pub misc: &'static mut Misc,
    pub stats: &'static mut Stats,
    pub effects: &'static mut Effects,
    pub combo_tracker: Option<&'static mut ComboTracker>,
}

#[derive(Debug, Clone)]
pub struct Combat;

impl Combat {
    pub fn process_damage(
        attack_type: AttackType,
        attacker: &mut CombatQueryItem,
        target: &mut CombatQueryItem,
        commands: &mut Commands,
        items: &mut ResMut<Items>,
        templates: &Res<Templates>,
        mut ids: &mut ResMut<Ids>,
        game_tick: &Res<GameTick>,
        mut map_events: &mut ResMut<MapEvents>
    ) -> (i32, Option<Combo>, Option<SkillUpdated>) {
        let mut rng = rand::thread_rng(); 

        let target_template = ObjTemplate::get_template(target.template.0.clone(), &templates);

        let damage_range = attacker.stats.damage_range.unwrap() as f32;
        let base_damage = attacker.stats.base_damage.unwrap() as f32;

        //TODO get item and weapons
        let attacker_items = items.get_equipped(attacker.id.0);
        debug!("Attacker Items: {:?}", attacker_items);
        let damage_from_items =
            Item::get_items_value_by_attr(&item::AttrKey::Damage, attacker_items);
        debug!("Damage From Items: {:?}", damage_from_items);

        //TODO get equiped weapons
        let attacker_weapons = items.get_equipped_weapons(attacker.id.0);
        debug!("Attacker_weapons: {:?}", attacker_weapons);

        // Get damage mod from effects
        let attack_effects_mod = Self::get_attack_effects(attacker, templates);

        let roll_damage = (rng.gen_range(0.0..damage_range) + base_damage);

        let total_damage = (roll_damage + damage_from_items) * attack_effects_mod;

        target.stats.hp -= total_damage as i32;

        let combo = Self::process_combo(commands, templates, attack_type, attacker, target, ids, game_tick, map_events);

        // Check if any weapons procced
        //Self::process_weapon_procs(commands, templates, &attacker_weapons, target);

        let mut skill_updated = None;

        debug!("Target HP: {:?}", target.stats.hp);
        // Check if target is dead and update skills
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

        return (total_damage as i32, combo, skill_updated);
    }

    fn process_weapon_procs(
        commands: &mut Commands,
        templates: &Res<Templates>,
        attacker_weapons: &Vec<Item>,
        target: &mut CombatQueryItem,
    ) {
        let mut rng = rand::thread_rng();

        for weapon in attacker_weapons.iter() {
            debug!("weapon: {:?}", weapon);
            if let Some(deep_wound_chance_attr) = weapon.attrs.get(&item::AttrKey::DeepWoundChance)
            {
                debug!("deep_wound_chance_attr: {:?}", deep_wound_chance_attr);
                let deep_wound_chance = match deep_wound_chance_attr {
                    item::AttrVal::Num(chance) => *chance,
                    _ => panic!("Invalid deep wound chance value"),
                };

                let roll = rng.gen_range(0.0..1.0);
                debug!(
                    "roll: {:?} deep_wound_chance: {:?}",
                    roll, deep_wound_chance
                );

                if roll <= deep_wound_chance {
                    let effects = &mut target.effects.0;

                    let effect_template = templates
                        .effect_templates
                        .get(&Effect::DeepWound.to_str())
                        .expect("Deep Wound is missing from configuration");

                    effects.insert(Effect::DeepWound, (10, 1.0, 1));

                    debug!("effects: {:?}", effects);
                }
            }
        }
    }

    fn process_combo(
        commands: &mut Commands,
        templates: &Res<Templates>,
        attack_type: AttackType,
        attacker: &mut CombatQueryItem,
        target: &mut CombatQueryItem,
        mut ids: &mut ResMut<Ids>,
        game_tick: &Res<GameTick>,
        mut map_events: &mut ResMut<MapEvents>
    ) -> Option<Combo> {
        let mut combo = None;
        // Only allow combos for players
        if attacker.player_id.0 < 1000 {
            debug!("check combo_tracker: {:?}", attacker.combo_tracker);

            if let Some(combo_tracker) = &mut attacker.combo_tracker {
                // Add to existing combo tracker only if same target id
                if combo_tracker.target_id == target.id.0 {
                    combo_tracker.attacks.push(attack_type);

                    let mut attacks_str = Vec::new();

                    for attack in combo_tracker.attacks.iter() {
                        attacks_str.push(attack.clone().to_str());
                    }

                    debug!("attack_str: {:?}", attacks_str);

                    for (_combo_name, combo_template) in templates.combo_templates.iter() {
                        debug!("combo_template.attacks: {:?}", combo_template.attacks);
                        if combo_template.attacks == attacks_str {
                            debug!("combo_template.effect: {:?}", combo_template.effect);
                            let effect_template = templates
                                .effect_templates
                                .get(&combo_template.effect)
                                .expect("Effect missing from templates");
                            debug!("effect_template: {:?}", effect_template);
                            let effect = Effect::from_string(&effect_template.name);

                            debug!("Effect applied: {:?}", effect);
                            //
                            match effect {
                                Effect::Hamstrung => {
                                    let hamstrung_event = VisibleEvent::EffectExpiredEvent {
                                        effect: effect.clone()
                                    };
                            
                                    map_events.new(
                                        ids.new_map_event_id(),
                                        target.entity,
                                        target.id,
                                        target.player_id,
                                        target.pos,
                                        game_tick.0 + effect_template.duration * TICKS_PER_SEC,
                                        hamstrung_event,
                                    );
                                }
                                Effect::Stunned => {
                                    let stun_event = VisibleEvent::EffectExpiredEvent {
                                        effect: effect.clone()
                                    };
                            
                                    map_events.new(
                                        ids.new_map_event_id(),
                                        target.entity,
                                        target.id,
                                        target.player_id,
                                        target.pos,
                                        game_tick.0 + effect_template.duration * TICKS_PER_SEC,
                                        stun_event,
                                    );
                                }
                                _ => {}
                            }

                            target
                                .effects
                                .0
                                .insert(effect, (effect_template.duration, 1.0, 1));

                            combo = Some(Combo::from_string(&combo_template.name));
                            break;
                        }
                    }
                } else {
                    combo_tracker.target_id = target.id.0;
                    combo_tracker.attacks = vec![attack_type];
                }
            } else {
                // TODO consider adding combo tracker component to every unit entity
                let combo_tracker = ComboTracker {
                    target_id: target.id.0,
                    attacks: vec![attack_type],
                };

                commands.entity(attacker.entity).insert(combo_tracker);
            }

            debug!("post check combo_tracker {:?}", attacker.combo_tracker);
        }

        return combo;
    }

    // Value returned is between 0.0 and 1.0
    fn get_attack_effects(attacker: &mut CombatQueryItem, templates: &Res<Templates>) -> f32 {
        for (effect, (_duration, _amplifier, _stacks)) in attacker.effects.0.iter() {
            let effect_template = templates
                .effect_templates
                .get(&effect.clone().to_str())
                .expect("Effect missing from templates");

            if let Some(effect_atk) = effect_template.atk {
                let modifier = 1.0 + effect_atk; // atk is negative in the template file
                return modifier;
            }
        }

        // No modifier if 1.0 is returned
        return 1.0;
    }
    

    pub fn add_damage_event(
        event_id: i32,
        game_tick: i32,
        attack_type: String,
        damage: i32,
        combo: Option<Combo>,
        attacker: &CombatQueryItem,
        target: &CombatQueryItem,
        map_events: &mut ResMut<MapEvents>,
    ) {
        let target_state_str = ObjUtil::state_to_str(target.state.clone());

        let damage_event = VisibleEvent::DamageEvent {
            target_id: target.id.0,
            target_pos: target.pos.clone(),
            attack_type: attack_type.clone(),
            damage: damage,
            combo: combo,
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

    pub fn attack_type_to_enum(attack_type: String) -> AttackType {
        match attack_type.as_str() {
            QUICK => AttackType::Quick,
            PRECISE => AttackType::Precise,
            FIERCE => AttackType::Fierce,
            _ => AttackType::Quick,
        }
    }

    pub fn combo_to_string(combo: Option<Combo>) -> Option<String> {
        match combo {
            Some(Combo::Hamstring) => Some(HAMSTRING.to_string()),
            Some(Combo::Gouge) => Some(GOUGE.to_string()),
            None => None,
            _ => Some("Unknown Combo".to_string()),
        }
    }
}
