use bevy::ecs::query::WorldQuery;
use bevy::prelude::*;
use big_brain::thinker::{Actor, ThinkerBuilder};

use rand::Rng;
use std::collections::HashMap;

use crate::effect::{self, Effect, Effects};
use crate::game::{
    BaseAttrs, Class, GameTick, Id, Ids, MapEvent, MapEvents, Misc, PlayerId, Position, State,
    Stats, Subclass, Template, VisibleEvent, VisibleEvents,
};
use crate::item::{self, Item, Items, DAMAGE, AttrKey};
use crate::map::Map;
use crate::obj::ObjUtil;
use crate::skill::{Skill, SkillUpdated, Skills};
use crate::templates::{
    ComboTemplate, EffectTemplate, ObjTemplate, SkillTemplate, SkillTemplates, Templates,
};

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
        map: &Res<Map>,
        mut ids: &mut ResMut<Ids>,
        game_tick: &Res<GameTick>,
        mut map_events: &mut ResMut<MapEvents>,
    ) -> (i32, Option<String>, Option<SkillUpdated>) {
        let mut rng = rand::thread_rng();

        // 1 Get Base Damage, DamageRange, BaseDef and DefHp
        let target_template = ObjTemplate::get_template(target.template.0.clone(), &templates);
        let damage_range = attacker.stats.damage_range.unwrap() as f32;
        let base_damage = attacker.stats.base_damage.unwrap() as f32;
        let base_defense = target.stats.base_def as f32;
    
        // 2 Get attacker & defender items
        let attacker_items = items.get_equipped(attacker.id.0);
        let defender_items = items.get_equipped(target.id.0);

        // #3 Get attacker weapons
        let attacker_weapons = items.get_equipped_weapons(attacker.id.0);
        debug!("Attacker_weapons: {:?}", attacker_weapons);

        // 4 Get damage effects on attacker
        let damage_effects_mod = Self::get_damage_effects(attacker, templates);

        // 5 Get defense effects on defender
        let defense_effects_mod = Self::get_defense_effects(target, templates);

        // 6 Get damage mod from items 
        let damage_from_items =
            Item::get_items_value_by_attr(&item::AttrKey::Damage, attacker_items);

        // 7 Get attack type damage from
        let attack_type_damage_mod = Self::attack_type_damage_mod(attack_type.clone());

        // TODO 8 Get damage reduction from Defensive action

        // 9 Get armor from defender items
        let defense_from_items =
        Item::get_items_value_by_attr(&item::AttrKey::Defense, defender_items);

        // TODO 10 Check if Defender has Defensive Stance

        // 11 & 12 Add attack type to attack list and check if combo is completed
        let combo_template =
            Self::process_combo(commands, templates, attack_type, attacker, target);

        // TODO 13 Check if combo is countered

        // TODO 14 Remove Defense Stanc Effect if combo countered

        // 15 Calculate combo damage and apply combo effects
        let (combo_quick_damage_mod, combo_precise_damage_mod, combo_fierce_damage_mod) =
            Self::get_combo_damage(combo_template.clone());            

        let combo_damage_mod = combo_quick_damage_mod * combo_precise_damage_mod * combo_fierce_damage_mod;
        debug!("combo_damage_mod: {:?}", combo_damage_mod);

        // TODO 16 Check if target is fortified 

        // 17 Roll from base damage
        let roll_damage = rng.gen_range(0.0..damage_range) + base_damage;

        // 18 Calculate total damage
        let total_damage = (roll_damage + damage_from_items) * damage_effects_mod * attack_type_damage_mod * combo_damage_mod;

        // 19 Calculate total defense
        let total_defense = (base_defense * defense_from_items) * defense_effects_mod;

        // 20 & 21 Calculate damage defense reduction
        let defense_reduction = total_defense / (total_defense + 50.0);
        let damage_reduction = total_damage * (1.0 - defense_reduction);

        // TODO 22 Get defense stance mod
        let defend_stance_mod = 1.0;

        // 23 Get terrain defense mod
        let terrain_defense_mod = Self::get_terrain_defense(*target.pos, map);
        
        // TODO 24 Get monolith distance defense mod
        let monolith_distance_defense_mod = 1.0;

        // 25 Calculate final damage
        let final_damage = damage_reduction * defend_stance_mod * terrain_defense_mod * monolith_distance_defense_mod;

        // 26 Update Hp and check if target is dead
        target.stats.hp -= final_damage as i32;

        // 27 Update stamina TODO remove static 100 value
        let attacker_stamina = attacker.stats.stamina.expect("Missing stamina stat");
        attacker.stats.stamina = Some(attacker_stamina - 100);
        
        // 28 Apply new effects from this attack
        Self::apply_combo_effects(
            combo_template.clone(),
            templates,
            attacker,
            target,
            ids,
            game_tick,
            map_events,
        );

        // 29 Check if any weapons procced
        Self::process_weapon_procs(templates, &attacker_weapons, target);


        // 30 & 31 Check if target is dead and update skills
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

        // Return combo name
        let mut combo_name = None;

        if let Some(combo) = combo_template {
            combo_name = Some(combo.name);
        }

        return (total_damage as i32, combo_name, skill_updated);
    }

    fn process_weapon_procs(
        templates: &Res<Templates>,
        attacker_weapons: &Vec<Item>,
        target: &mut CombatQueryItem,
    ) {
        let mut rng = rand::thread_rng();

        for weapon in attacker_weapons.iter() {
            debug!("weapon: {:?}", weapon);

            for proc_attr_key in AttrKey::proc_iter() {
                if let Some(attr_val) = weapon.attrs.get(&proc_attr_key) {
                    debug!("attr_val: {:?}", attr_val);
                    let chance = match attr_val {
                        item::AttrVal::Num(chance) => *chance,
                        _ => panic!("Invalid attr value"),
                    };

                    let roll = rng.gen_range(0.0..1.0);

                    debug!("roll: {:?} chance: {:?}", roll, chance);

                    if roll <= chance {
                        let effect = proc_attr_key.clone().proc_to_effect();
                        debug!("proc effect: {:?}", effect);

                        let effect_string = effect.clone().to_str();

                        let effect_template = templates
                            .effect_templates
                            .get(&effect_string)
                            .expect("Cannot find template for effect");

                        let effects = &mut target.effects.0;
                        effects.insert(effect, (effect_template.duration, 1.0, 1));
    
                        debug!("effects: {:?}", effects);
                    }
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
    ) -> Option<ComboTemplate> {
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
                            combo = Some(combo_template.clone());
                            combo_tracker.attacks.clear();
                            break;
                        }
                    }
                } else {
                    combo_tracker.target_id = target.id.0;
                    combo_tracker.attacks = vec![attack_type];
                }
            } else {

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

    fn apply_combo_effects(
        combo: Option<ComboTemplate>,
        templates: &Res<Templates>,
        attacker: &mut CombatQueryItem,
        target: &mut CombatQueryItem,
        mut ids: &mut ResMut<Ids>,
        game_tick: &Res<GameTick>,
        mut map_events: &mut ResMut<MapEvents>,
    ) {
        if let Some(combo_template) = combo {
            for effect_name in combo_template.effects.iter() {
                debug!("combo_template.effect: {:?}", combo_template.effects);

                let effect_template = templates
                    .effect_templates
                    .get(&effect_name.clone())
                    .expect("Effect missing from templates");
                debug!("effect_template: {:?}", effect_template);
                let effect = Effect::from_string(&effect_template.name);

                debug!("Effect applied: {:?}", effect);
                //
                match effect {
                    Effect::Hamstrung => {
                        let hamstrung_event = VisibleEvent::EffectExpiredEvent {
                            effect: effect.clone(),
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
                            effect: effect.clone(),
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
            }
        }
    }

    fn get_combo_damage(combo_template: Option<ComboTemplate>) -> (f32, f32, f32) {
        if let Some(combo_template) = combo_template {
            return (
                combo_template.quick_damage,
                combo_template.precise_damage,
                combo_template.fierce_damage,
            );
        } else {
            return (1.0, 1.0, 1.0);
        }
    }

    // Value returned is between 0.0 and 1.0
    fn get_damage_effects(attacker: &mut CombatQueryItem, templates: &Res<Templates>) -> f32 {
        for (effect, (_duration, _amplifier, _stacks)) in attacker.effects.0.iter() {
            let effect_template = templates
                .effect_templates
                .get(&effect.clone().to_str())
                .expect("Effect missing from templates");

            if let Some(effect_damage) = effect_template.damage {
                let modifier = 1.0 + effect_damage; // atk is negative in the template file
                return modifier;
            }
        }

        // No modifier if 1.0 is returned
        return 1.0;
    }

    fn get_defense_effects(target: &mut CombatQueryItem, templates: &Res<Templates>) -> f32 {
        for (effect, (_duration, _amplifier, _stacks)) in target.effects.0.iter() {
            let effect_template = templates
                .effect_templates
                .get(&effect.clone().to_str())
                .expect("Effect missing from templates");

            if let Some(effect_defense) = effect_template.defense {
                let modifier = 1.0 + effect_defense; // defense is negative in the template file
                return modifier;
            }
        }

        // No modifier if 1.0 is returned
        return 1.0;
    }

    fn get_terrain_defense(position: Position, map: &Res<Map>) -> f32 {
        return 1.0 + Map::def_bonus(Map::tile_type(position.x, position.y, &map));
    }

    pub fn add_damage_event(
        event_id: i32,
        game_tick: i32,
        attack_type: String,
        damage: i32,
        combo: Option<String>,
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

    fn attack_type_damage_mod(attack_type: AttackType) -> f32 {
        match attack_type {
            AttackType::Quick => 0.5,
            AttackType::Precise => 1.0,
            AttackType::Fierce => 1.5,
            _ => 0.0
        }
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
