use bevy::prelude::*;

use std::collections::HashMap;
use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;

use crate::obj;

#[derive(Debug, Resource)]
pub struct Templates {
    pub item_templates: Vec<ItemTemplate>,
    pub res_templates: ResTemplates,
    pub skill_templates: SkillTemplates,
    pub obj_templates: ObjTemplates,
    pub recipe_templates: RecipeTemplates,
    pub effect_templates: EffectTemplates,
    pub combo_templates: ComboTemplates,
    pub res_property_templates: ResPropertyTemplates,
    pub terrain_feature_templates: TerrainFeatureTemplates
}

#[derive(Debug, Resource, Deref, DerefMut)]
pub struct ObjTemplates(Vec<ObjTemplate>);

#[derive(Debug, Clone, Resource, PartialEq, Serialize, Deserialize)]
pub struct ResReq {
    #[serde(rename = "type")]
    pub req_type: String,
    pub quantity: i32,
    pub cquantity: Option<i32>, // current quantity
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
    pub upgrade_req: Option<Vec<ResReq>>,
    pub upgrade_to: Option<Vec<String>>,
    pub profession: Option<String>,
    pub upkeep: Option<Vec<ResReq>>,
}

impl ObjTemplate {

    pub fn get_template_by_name(name: String, templates: &Res<Templates>) -> ObjTemplate {
        for obj_template in templates.obj_templates.iter() {
            if name == obj_template.name {
                return obj_template.clone();
            }
        }

        // Cannot recover from an invalid obj template
        panic!("Cannot find obj_template: {:?}", name);
    }

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

/*#[derive(Debug, Resource, Deref, DerefMut)]
pub struct ItemTemplates(Vec<ItemTemplate>);*/

#[derive(Debug, Reflect, Clone, PartialEq, Serialize, Deserialize)]

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
    pub yield_rate: Vec<i32>,
    pub yield_mod: Vec<f32>,
    pub quantity_rate: Vec<i32>,
    pub quantity: Vec<i32>,
    pub skill_req: i32,
    pub level: i32,
    pub quality_rate: Option<Vec<i32>>,
    pub properties: Option<Vec<String>>,
    pub num_properties: Option<i32>
}

#[derive(Debug, Resource, Deref, DerefMut)]
pub struct ResPropertyTemplates(HashMap<String, ResPropertyTemplate>);

#[derive(Debug, Resource, Clone, PartialEq, Hash, Eq, Serialize, Deserialize)]
pub struct ResPropertyTemplate {
    pub name: String,
    pub ranges: Vec<Vec<i32>>,
    pub tag: Vec<String>,
}

impl ResPropertyTemplates {
    pub fn load(&mut self, res_property_templates: Vec<ResPropertyTemplate>) {                 

        for res_property_template in res_property_templates.iter() {
            debug!("{:?}", res_property_template);
            self.insert(res_property_template.name.clone(), res_property_template.clone() );
        }
    }

    pub fn get(&self, name: String) -> Vec<ResPropertyTemplate> {        
        debug!("Finding name: {:?}", name);

        let mut res_properties = HashSet::new();

        // First try to find by the name value
        for (template_name, res_property_template) in self.iter() {
            if name == *template_name {
                res_properties.insert(res_property_template.clone());
            }

            for tag in res_property_template.tag.iter() {
                if name == *tag {
                    res_properties.insert(res_property_template.clone());
                }
            }
        }

        return res_properties.into_iter().collect();
    }
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

#[derive(Debug, Clone, Resource, PartialEq, Serialize, Deserialize)]
pub struct RecipeTemplate {
    pub name: String,
    pub image: String,
    pub structure: String,
    pub class: String,
    pub subclass: String,
    pub tier: Option<i32>,
    pub slot: Option<String>,
    pub damage: Option<i32>,
    pub speed: Option<f32>,
    pub armor: Option<i32>,
    pub stamina_req: Option<i32>,
    pub skill_req: Option<i32>,
    pub weight: i32,
    pub req: Vec<ResReq>,
}

impl RecipeTemplate {
    pub fn get_by_structure(structure: String, templates: &Res<Templates>) -> Vec<RecipeTemplate> {
        let mut recipe_templates = Vec::new();

        for recipe_template in templates.recipe_templates.iter() {
            if structure == recipe_template.structure {
                recipe_templates.push(recipe_template.clone());
            }
        }

        return recipe_templates;
    }

    pub fn get_by_name(name: String, templates: &Res<Templates>) -> Option<RecipeTemplate> {
        for recipe_template in templates.recipe_templates.iter() {
            if name == recipe_template.name {
                return Some(recipe_template.clone());
            }
        }

        return None;
    }
}

#[derive(Debug, Clone, Resource, PartialEq, Serialize, Deserialize)]
pub struct EffectTemplate {
    pub name: String,
    pub duration: i32,
    pub max_hp: Option<f32>,
    pub healing: Option<f32>,
    pub damage: Option<f32>,
    pub damage_over_time: Option<f32>,
    pub speed: Option<f32>,
    pub attack_speed: Option<f32>,
    pub defense: Option<f32>,
    pub stackable: Option<bool>,
    pub armor: Option<f32>,
    pub lifeleech: Option<f32>,
    pub viewshed: Option<i32>,
    pub ignore_all_armor: Option<bool>,
    pub instant_kill_chance: Option<f32>,
    pub next_attack: Option<bool>
}

type EffectName = String;

#[derive(Debug, Resource, Deref, DerefMut)]
pub struct EffectTemplates(HashMap<EffectName, EffectTemplate>);

impl EffectTemplates {
    pub fn load(&mut self, effect_templates: Vec<EffectTemplate>) {                 

        for effect_template in effect_templates.iter() {
            self.insert(effect_template.name.clone(), effect_template.clone() );
        }
    }
}

#[derive(Debug, Clone, Resource, PartialEq, Serialize, Deserialize)]
pub struct ComboTemplate {
    pub name: String,
    pub attacks: Vec<String>,
    pub effects: Vec<String>,
    pub quick_damage: f32,
    pub precise_damage: f32,
    pub fierce_damage: f32
}

type ComboName = String;

#[derive(Debug, Resource, Deref, DerefMut)]
pub struct ComboTemplates(HashMap<ComboName, ComboTemplate>);

impl ComboTemplates {
    pub fn load(&mut self, combo_templates: Vec<ComboTemplate>) {                 

        for combo_template in combo_templates.iter() {
            self.insert(combo_template.name.clone(), combo_template.clone() );
        }
    }
}

#[derive(Debug, Clone, Resource, PartialEq, Serialize, Deserialize)]
pub struct TerrainFeatureTemplate {
    pub name: String,
    pub image: String,
    pub description: String,
    pub bonus: String,
    pub terrain: Vec<String>
}

#[derive(Debug, Resource, Deref, DerefMut)]
pub struct TerrainFeatureTemplates(HashMap<String, TerrainFeatureTemplate>);

impl TerrainFeatureTemplates {
    pub fn load(&mut self, terrain_feature_templates: Vec<TerrainFeatureTemplate>) {                 

        for terrain_feature_templates in terrain_feature_templates.iter() {
            self.insert(terrain_feature_templates.name.clone(), terrain_feature_templates.clone() );
        }
    }
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
        let res_templates: HashMap<_, _> = res_templates_vec
            .iter()
            .map(|x| (x.name.clone(), x.clone()))
            .collect();

        // Load skill template data
        let skill_template_file =
            fs::File::open("skill_template.yaml").expect("Could not open file.");
        let skill_templates_vec: Vec<SkillTemplate> =
            serde_yaml::from_reader(skill_template_file).expect("Could not read values.");

        // Convert vector to hashmap for faster access of individual skill
        let skill_templates: HashMap<_, _> = skill_templates_vec
            .iter()
            .map(|x| (x.name.clone(), x.clone()))
            .collect();

        // Load skill template data
        let recipe_template_file =
            fs::File::open("recipe_template.yaml").expect("Could not open file.");
        let recipe_templates: Vec<RecipeTemplate> =
            serde_yaml::from_reader(recipe_template_file).expect("Could not read values.");

        // Load effect template data
        let effect_template_file =
            fs::File::open("effect_template.yaml").expect("Could not open file.");

        let effect_template_list: Vec<EffectTemplate> =
            serde_yaml::from_reader(effect_template_file).expect("Could not read values.");

        let mut effect_templates = EffectTemplates(HashMap::new());
        effect_templates.load(effect_template_list);

        // Load combo template data
        let combo_template_file =
            fs::File::open("combo_template.yaml").expect("Could not open file.");

        let combo_template_list: Vec<ComboTemplate> =
            serde_yaml::from_reader(combo_template_file).expect("Could not read values.");

        let mut comobo_templates = ComboTemplates(HashMap::new());
        comobo_templates.load(combo_template_list);

        // Load properties template data
        let res_property_template_file =
            fs::File::open("res_property_template.yaml").expect("Could not open file.");

        let res_property_template_list: Vec<ResPropertyTemplate> =
            serde_yaml::from_reader(res_property_template_file).expect("Could not read values.");
        
        let mut res_property_templates = ResPropertyTemplates(HashMap::new());
        res_property_templates.load(res_property_template_list);

        // Load terrain features template data
        let terrain_feature_template_file = fs::File::open("terrain_feature_template.yaml").expect("Could not open file.");

        let terrain_feature_template_list: Vec<TerrainFeatureTemplate> = serde_yaml::from_reader(terrain_feature_template_file).expect("Could not read values.");

        let mut terrain_feature_templates = TerrainFeatureTemplates(HashMap::new());
        terrain_feature_templates.load(terrain_feature_template_list);

        let templates = Templates {
            item_templates: item_templates,
            res_templates: ResTemplates(res_templates),
            skill_templates: SkillTemplates(skill_templates),
            obj_templates: ObjTemplates(obj_templates),
            recipe_templates: RecipeTemplates(recipe_templates),
            effect_templates: effect_templates,
            combo_templates: comobo_templates,
            res_property_templates: res_property_templates,
            terrain_feature_templates: terrain_feature_templates
        };

        app.insert_resource(templates);
    }
}
