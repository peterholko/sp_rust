use bevy::prelude::*;

use std::collections::HashMap;

use crate::network;
use crate::templates::{RecipeTemplates, ResReq, Templates, RecipeTemplate};

#[derive(Debug, Clone)]
pub struct Recipe {
    pub name: String,
    pub image: String,
    pub owner: i32,
    pub structure: String,
    pub class: String,
    pub subclass: String,
    pub tier: Option<i32>,
    pub slot: String,
    pub damage: Option<i32>,
    pub speed: Option<f32>,
    pub armor: Option<i32>,
    pub stamina_req: Option<i32>,
    pub skill_req: Option<i32>,
    pub weight: i32,
    pub req: Vec<ResReq>,
}

#[derive(Resource, Deref, DerefMut, Debug)]
pub struct Recipes(Vec<Recipe>);

impl Recipe {
    pub fn create(
        player: i32,
        name: String,
        recipe_templates: &RecipeTemplates,
        recipes: &mut ResMut<Recipes>,
    ) {
        for recipe_template in recipe_templates.iter() {
            if name == recipe_template.name {
                let new_recipe = Recipe {
                    name: recipe_template.name.clone(),
                    image: recipe_template.image.clone(),
                    owner: player,
                    structure: recipe_template.structure.clone(),
                    class: recipe_template.class.clone(),
                    subclass: recipe_template.subclass.clone(),
                    tier: recipe_template.tier,
                    slot: recipe_template.slot.clone(),
                    damage: recipe_template.damage,
                    speed: recipe_template.speed,
                    armor: recipe_template.armor,
                    stamina_req: recipe_template.stamina_req,
                    skill_req: recipe_template.skill_req,
                    weight: recipe_template.weight,
                    req: recipe_template.req.clone(),
                };

                recipes.push(new_recipe);
            }
        }

        println!("Recipes: {:?}", recipes);
    }

    pub fn get_by_name(name: String, recipes: &Res<Recipes>) -> Option<Recipe> {
        for recipe in recipes.iter() {
            if recipe.name == *name {
                return Some(recipe.clone());
            }
        }

        return None;
    }

    pub fn get_by_subclass_tier(structure: String, subclass: String, tier: i32, templates: Res<Templates>) -> Vec<RecipeTemplate> {
        let all_recipes = RecipeTemplate::get_by_structure(structure, templates);

        let mut recipes_by_subclass_tier = Vec::new();

        for recipe in all_recipes.iter() {
            if let Some(recipe_tier) = recipe.tier {
                if recipe.subclass == subclass && recipe_tier == tier {
                    recipes_by_subclass_tier.push(recipe.clone());
                }
            }
        }

        return recipes_by_subclass_tier;
    }

    pub fn get_by_structure(structure_id: i32, recipes: &Res<Recipes>) -> Vec<Recipe> {
        
        let mut owner_recipes: Vec<Recipe> = Vec::new();

        for recipe in recipes.iter() {
            if recipe.owner == structure_id {
                owner_recipes.push(recipe.clone());
            }
        }

        return owner_recipes
    }


    pub fn get_by_structure_packet(
        owner: i32,
        structure: String,
        recipes: &Res<Recipes>,
    ) -> Vec<network::Recipe> {
        let mut owner_recipes: Vec<network::Recipe> = Vec::new();

        println!("Owner: {:?} Structure: {:?} Recipes: {:?}", owner, structure, recipes);

        for recipe in recipes.iter() {
            // Remove all whitespaces
            let mut recipe_structure: String = recipe.structure.clone();
            recipe_structure.retain(|c| !c.is_whitespace());

            if recipe.owner == owner && recipe_structure == structure {
                let recipe_packet = network::Recipe {
                    name: recipe.name.clone(),
                    image: recipe.image.clone(),
                    structure: recipe.structure.clone(),
                    class: recipe.class.clone(),
                    subclass: recipe.subclass.clone(),
                    tier: recipe.tier.clone(),
                    slot: recipe.slot.clone(),
                    damage: recipe.damage,
                    speed: recipe.speed,
                    armor: recipe.armor,
                    stamina_req: recipe.stamina_req,
                    skill_req: recipe.skill_req,
                    weight: recipe.weight,
                    req: recipe.req.clone(),
                };

                owner_recipes.push(recipe_packet);
            }
        }

        return owner_recipes;
    }
}

pub struct RecipePlugin;

impl Plugin for RecipePlugin {
    fn build(&self, app: &mut App) {
        let recipes = Recipes(Vec::new());

        app.insert_resource(recipes);
    }
}
