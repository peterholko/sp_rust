use bevy::prelude::*;

use std::collections::HashMap;

use rand::{random, Rng};

use crate::game::{BaseAttrs, Id, MapEvents, PlayerId, Position, State, VisibleEvent};
use crate::item::{Item, Items};
use crate::map::TileType;
use crate::skill::{Skill, Skills};
use crate::templates::{ObjTemplate, ObjTemplates, SkillTemplate, SkillTemplates, Templates};

#[derive(Debug, Clone)]
pub struct ObjUtil;

pub const TEMPLATE: &str = "template";
pub const POSITION: &str = "position";
pub const CLASS_STRUCTURE: &str = "structure";
pub const CLASS_UNIT: &str = "unit";
pub const SUBCLASS_HERO: &str = "hero";
pub const SUBCLASS_VILLAGER: &str = "villager";
pub const SUBCLASS_SHELTER: &str = "shelter";
pub const SUBCLASS_MERCHANT: &str = "merchant";

// States
pub const STATE_NONE: &str = "none";
pub const STATE_MOVING: &str = "moving";
pub const STATE_ATTACKING: &str = "attacking";
pub const STATE_DEAD: &str = "dead";
pub const STATE_FOUNDED: &str = "founded";
pub const STATE_PROGRESSING: &str = "progressing";
pub const STATE_BUILDING: &str = "building";
pub const STATE_UPGRADING: &str = "upgrading";
pub const STATE_STALLED: &str = "stalled";
pub const STATE_GATHERING: &str = "gathering";
pub const STATE_REFINING: &str = "refining";
pub const STATE_CRAFTING: &str = "crafting";
pub const STATE_EXPLORING: &str = "exploring";
pub const STATE_DRINKING: &str = "drinking";
pub const STATE_EATING: &str = "eating";
pub const STATE_SLEEPING: &str = "sleeping";

impl ObjUtil {
    pub fn state_to_enum(state: String) -> State {
        match state.as_str() {
            STATE_NONE => State::None,
            STATE_MOVING => State::Moving,
            STATE_DEAD => State::Dead,
            STATE_FOUNDED => State::Founded,
            STATE_PROGRESSING => State::Progressing,
            STATE_BUILDING => State::Building,
            STATE_UPGRADING => State::Upgrading,
            STATE_STALLED => State::Stalled,
            STATE_GATHERING => State::Gathering,
            STATE_REFINING => State::Refining,
            STATE_CRAFTING => State::Crafting,
            STATE_EXPLORING => State::Exploring,
            STATE_DRINKING => State::Drinking,
            STATE_EATING => State::Eating,
            STATE_SLEEPING => State::Sleeping,
            _ => State::None,
        }
    }

    pub fn state_to_str(state: State) -> String {
        let state_string = match state {
            State::None => STATE_NONE,
            State::Moving => STATE_MOVING,
            State::Dead => STATE_DEAD,
            State::Founded => STATE_FOUNDED,
            State::Progressing => STATE_PROGRESSING,
            State::Building => STATE_BUILDING,
            State::Upgrading => STATE_UPGRADING,
            State::Stalled => STATE_STALLED,
            State::Gathering => STATE_GATHERING,
            State::Refining => STATE_REFINING,
            State::Crafting => STATE_CRAFTING,
            State::Exploring => STATE_EXPLORING,
            State::Drinking => STATE_DRINKING,
            State::Eating => STATE_EATING,
            State::Sleeping => STATE_SLEEPING,
            _ => STATE_NONE,
        };

        return state_string.to_string();
    }

    pub fn get_capacity(template: &String, obj_templates: &ObjTemplates) -> i32 {
        for obj_template in obj_templates.iter() {
            if obj_template.template == *template {
                if let Some(capacity) = obj_template.capacity {
                    return capacity;
                } else {
                    info!(
                        "No capacity found for obj template: {:?} defaulting to 0",
                        template
                    );
                    return 0;
                }
            }
        }

        info!("No template found for {:?}", template);

        return 0;
    }

    pub fn add_sound_obj_event(
        event_id: i32,
        game_tick: i32,
        sound: String,
        entity: Entity,
        obj_id: &Id,
        player_id: &PlayerId,
        pos: &Position,
        map_events: &mut ResMut<MapEvents>,
    ) {
        let damage_event = VisibleEvent::SoundObjEvent {
            sound: sound,
            intensity: 2,
        };

        map_events.new(
            event_id,
            entity,
            &obj_id,
            &player_id,
            &pos,
            game_tick,
            damage_event,
        );
    }
}
