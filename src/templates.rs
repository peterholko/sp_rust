use bevy::prelude::*;

use std::{
    collections::HashMap};

use serde::{Deserialize, Serialize};
use std::fs;
use serde_json::Value;

#[derive(Debug, Resource)]
pub struct Templates {
    pub item_templates: ItemTemplates,
    pub res_templates: ResTemplates,
    pub skill_templates: SkillTemplates,
    pub obj_templates: ObjTemplates,
    pub recipe_templates: RecipeTemplates,
}

#[derive(Debug, Resource, Deref, DerefMut)]
pub struct ObjTemplates(Vec<ObjTemplate>);

#[derive(Debug, Clone, Resource, PartialEq, Serialize, Deserialize)]
pub struct ResReq {
    #[serde(rename = "type")]
    pub req_type: String,
    pub quantity: i32,
    pub cquantity: Option<i32> // current quantity
}

#[derive(Debug, Clone, Resource, PartialEq, Serialize, Deserialize)]
// Another way to build the struct... 
/*pub struct ObjTemplate {
    pub name: String,
    pub class: String,
    pub subclass: String,
    pub template: String,
    #[serde(flatten)]
    pub attrs: HashMap<String, Value>
}*/
pub struct ObjTemplate {
    pub name: String,
    pub class: String,
    pub subclass: String,
    pub template: String,
    pub family: Option<String>,
    pub base_hp: Option<i32>,
    pub base_stamina: Option<i32>,
    pub base_dmg: Option<i32>,
    pub dmg_range: Option<i32>,
    pub base_def: Option<i32>,
    pub base_speed: Option<i32>,
    pub base_vision: Option<i32>,
    pub int: Option<String>,
    pub aggression: Option<String>,
    pub kill_xp: Option<i32>,
    pub images: Option<Vec<String>>,
    pub hsl: Option<Vec<i32>>,
    pub waterwalk: Option<i32>,
    pub landwalk: Option<i32>,
    pub capacity: Option<i32>,
    pub build_time: Option<i32>,
    pub level: Option<i32>,
    pub refine: Option<Vec<String>>,
    pub req: Option<Vec<ResReq>>,
    pub profession: Option<String>,
    pub upkeep: Option<Vec<ResReq>>,
}

impl ObjTemplate {
    pub fn get_template(template_name: String, templates: &Res<Templates>) -> ObjTemplate {
        for obj_template in templates.obj_templates.iter() {
            if template_name == obj_template.template {
                return obj_template.clone();
            }
        }

        // Cannot recover from an invalid obj template 
        panic!("Cannot find obj_template: {:?}", template_name);
    }

}



#[derive(Debug, Resource, Deref, DerefMut)]
pub struct ItemTemplates(Vec<ItemTemplate>);

#[derive(Debug, Resource, PartialEq, Serialize, Deserialize)]
pub struct ItemTemplate {
    pub name: String,
    pub class: String,
    pub subclass: String,
    pub image: String,
    pub weight: f32,
    pub produces: Option<Vec<String>>,
    pub slot: Option<String>,
}

#[derive(Debug, Resource, Deref, DerefMut)]
pub struct ResTemplates(HashMap<String, ResTemplate>);

#[derive(Debug, Resource, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResTemplate {
    pub name: String,
    #[serde(rename = "type")]
    pub res_type: String,
    pub terrain: Vec<String>,
    pub quantity_rate: Vec<i32>,
    pub quantity: Vec<i32>,
    pub skill_req: i32,
}

#[derive(Debug, Resource, Deref, DerefMut)]
pub struct SkillTemplates(HashMap<String, SkillTemplate>);

#[derive(Debug, Resource, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillTemplate {
    pub name: String,
    pub class: String,
    pub xp: Vec<i32>,
}

#[derive(Debug, Resource, Deref, DerefMut)]
pub struct RecipeTemplates(Vec<RecipeTemplate>);

#[derive(Debug, Resource, PartialEq, Serialize, Deserialize)]
pub struct RecipeTemplate {
    name: String,
    image: String,
    structure: String,
    class: String,
    subclass: String,
    tier: Option<i32>,
    slot: String,
    damage: Option<i32>,
    speed: Option<f32>,
    stamina_req: Option<i32>,
    skill_req: Option<i32>,
    weight: i32,
    req: Vec<ResReq>,
}

/// The systems that make structures tick.
pub struct TemplatesPlugin;

impl Plugin for TemplatesPlugin {
    fn build(&self, app: &mut App) {
        // Load skill template data
        let obj_template_file = fs::File::open("obj_template.yaml").expect("Could not open file.");
        let obj_templates: Vec<ObjTemplate> =
            serde_yaml::from_reader(obj_template_file).expect("Could not read values.");

        // Load item template data
        let item_template_file =
            fs::File::open("item_template.yaml").expect("Could not open file.");
        let item_templates: Vec<ItemTemplate> =
            serde_yaml::from_reader(item_template_file).expect("Could not read values.");

        // Load res template data
        let res_template_file = fs::File::open("res_template.yaml").expect("Could not open file.");
        let res_templates_vec: Vec<ResTemplate> =
            serde_yaml::from_reader(res_template_file).expect("Could not read values.");

        // Convert vector to hashmap for faster access of individual skill
        let res_templates: HashMap<_, _> =
            res_templates_vec.iter().map(|x| (x.name.clone(), x.clone())).collect();

        // Load skill template data
        let skill_template_file =
            fs::File::open("skill_template.yaml").expect("Could not open file.");
        let skill_templates_vec: Vec<SkillTemplate> =
            serde_yaml::from_reader(skill_template_file).expect("Could not read values.");

        // Convert vector to hashmap for faster access of individual skill
        let skill_templates: HashMap<_, _> =
            skill_templates_vec.iter().map(|x| (x.name.clone(), x.clone())).collect();

        // Load skill template data
        let recipe_template_file =
            fs::File::open("recipe_template.yaml").expect("Could not open file.");
        let recipe_templates: Vec<RecipeTemplate> =
            serde_yaml::from_reader(recipe_template_file).expect("Could not read values.");

        let templates = Templates {
            item_templates: ItemTemplates(item_templates),
            res_templates: ResTemplates(res_templates),
            skill_templates: SkillTemplates(skill_templates),
            obj_templates: ObjTemplates(obj_templates),
            recipe_templates: RecipeTemplates(recipe_templates),
        };

        app.insert_resource(templates);
    }
}
