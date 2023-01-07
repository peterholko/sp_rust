use bevy::prelude::*;

use std::collections::HashMap;

use crate::network;
use crate::templates::{ObjTemplate, ObjTemplates, ResReq};

pub struct Structure;

impl Structure {
    pub fn available_to_build(obj_templates: &ObjTemplates) -> Vec<network::Structure> {
        let mut available_list: Vec<network::Structure> = Vec::new();

        for obj_template in obj_templates.iter() {
            if let Some(level) = obj_template.level {
                if level == 0 {
                    let structure = network::Structure {
                        name: obj_template.name.clone(),
                        image: str::replace(obj_template.name.as_str(), " ", "").to_lowercase(),
                        class: obj_template.class.clone(),
                        subclass: obj_template.subclass.clone(),
                        template: obj_template.template.clone(),
                        base_hp: obj_template.base_hp.unwrap_or_default(),
                        base_def: obj_template.base_def.unwrap_or_default(),
                        build_time: obj_template.build_time.unwrap_or_default(),
                        req: obj_template.req.clone().unwrap_or_default(),
                    };

                    available_list.push(structure);
                }
            }
        }

        return available_list;
    }

    pub fn get(name: String, obj_templates: &ObjTemplates) -> Option<ObjTemplate> {
        for obj_template in obj_templates.iter() {
            if obj_template.name == *name {
                return Some(obj_template.clone());
            }
        }

        return None;
    }

    pub fn get_reqs() {

    }
}

/*pub struct StructurePlugin;

impl Plugin for StructurePlugin {
    fn build(&self, app: &mut App) {
    }
}*/
