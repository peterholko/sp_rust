use bevy::prelude::*;

use std::collections::HashMap;

use rand::{random, Rng};

use crate::game::{BaseAttrs, Ids, Position};
use crate::item::{Item, Items};
use crate::map::TileType;
use crate::skill::{Skill, Skills};
use crate::templates::{SkillTemplate, SkillTemplates, Templates};

#[derive(Debug, Clone)]
pub struct Obj;

pub const TEMPLATE: &str = "template";

impl Obj {
    pub fn info() {

    }
}
