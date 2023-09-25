use bevy::prelude::*;

use std::collections::HashMap;

use rand::{random, Rng};

use crate::game::{BaseAttrs, Id, Ids, MapEvents, PlayerId, Position, VisibleEvent};
use crate::item::{Item, Items};
use crate::map::TileType;
use crate::skill::{Skill, Skills};
use crate::templates::{ObjTemplate, ObjTemplates, SkillTemplate, SkillTemplates, Templates};

#[derive(Debug, Clone)]
pub struct ObjUtil;

pub const TEMPLATE: &str = "template";
pub const CLASS_STRUCTURE: &str = "structure";
pub const CLASS_UNIT: &str = "unit";
pub const SUBCLASS_HERO: &str = "hero";
pub const SUBCLASS_VILLAGER: &str = "villager";
pub const SUBCLASS_SHELTER: &str = "shelter";

// States
pub const STATE_NONE: &str = "none";
pub const STATE_MOVING: &str = "moving";
pub const STATE_ATTACKING: &str = "attacking";
pub const STATE_DEAD: &str = "dead";
pub const STATE_FOUNDED: &str = "founded";
pub const STATE_PROGRESSING: &str = "progressing";
pub const STATE_BUILDING: &str = "building";
pub const STATE_STALLED: &str = "stalled";
pub const STATE_GATHERING: &str = "gathering";
pub const STATE_REFINING: &str = "refining";
pub const STATE_CRAFTING: &str = "crafting";
pub const STATE_EXPLORING: &str = "exploring";
pub const STATE_DRINKING: &str = "drinking";
pub const STATE_EATING: &str = "eating";
pub const STATE_SLEEPING: &str = "sleeping";

impl ObjUtil {
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
