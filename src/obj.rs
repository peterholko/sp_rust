use bevy::prelude::*;

use std::collections::HashMap;

use rand::{random, Rng};

use crate::game::{BaseAttrs, Ids, Position};
use crate::item::{Item, Items};
use crate::map::TileType;
use crate::skill::{Skill, Skills};
use crate::templates::{SkillTemplate, SkillTemplates, Templates, ObjTemplate, ObjTemplates};

#[derive(Debug, Clone)]
pub struct ObjUtil;

pub const TEMPLATE: &str = "template";
pub const CLASS_STRUCTURE: &str = "structure";
pub const CLASS_UNIT: &str = "unit";
pub const SUBCLASS_HERO: &str = "hero";
pub const SUBCLASS_VILLAGER: &str = "villager";

// States
pub const STATE_NONE: &str = "none";
pub const STATE_MOVING: &str = "moving";
pub const STATE_ATTACKING: &str = "attacking";
pub const STATE_DEAD: &str = "dead";
pub const STATE_FOUNDED: &str = "founded";
pub const STATE_PROGRESSING: &str = "progressing";
pub const STATE_BUILDING: &str = "building";
pub const STATE_STALLED: &str = "stalled";

impl ObjUtil {
    pub fn get_capacity(template: &String, obj_templates: &ObjTemplates) -> i32 {

        for obj_template in obj_templates.iter() {
            if obj_template.template == *template {
                return obj_template.capacity.unwrap();
            }
        }

        panic!("Not capacity found for obj template: {:?}", template);
    }
}
