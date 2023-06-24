use bevy::prelude::*;

use crate::{templates::{RecipeTemplates, ResReq, Templates}, item::{Item, Items}, recipe::{Recipe, Recipes}};



pub const EXP_STATE_NONE: &str = "Not started";
pub const EXP_RECIPE_NONE: &str = "No recipe";

#[derive(Debug, Clone)]
pub struct Experiment {
    pub structure: i32,
    pub recipe: String,
    pub state: String,
    pub exp_item: String,
    pub req: Vec<ResReq>
}

#[derive(Resource, Deref, DerefMut, Debug)]
pub struct Experiments(Vec<Experiment>);

impl Experiment {

    pub fn find_recipe(player_id: i32, structure_id: i32, structure_name: String, items: &ResMut<Items>, recipes: &Res<Recipes>, templates: Res<Templates>) {

        let (experiment_source, experiment_reagents) = Item::get_experiment_source_reagents(structure_id, items);

        let Some(experiment_source) = experiment_source else {
            debug!("Experiment source is not set, structure id: {:?}", structure_id);
            return;
        };

        let source_recipe = Recipe::get_by_name(experiment_source.name.clone(), recipes);

        let Some(source_recipe) = source_recipe else {
            debug!("Source item recipe cannot be found, experiment source name: {:?}", experiment_source);
            return;
        }; 

        let Some(source_recipe_tier) = source_recipe.tier else {
            debug!("Source recipe does not have a tier attribute {:?}", source_recipe);
            return;
        };

        let player_recipes = Recipe::get_by_structure(structure_id, recipes);

        let matching_recipe_templates = Recipe::get_by_subclass_tier(structure_name, source_recipe.subclass, source_recipe_tier, templates);

        let mut undiscovered_recipes = Vec::new();

        for recipe_template in matching_recipe_templates.iter() {

            for player_recipe in player_recipes.iter() {

                if recipe_template.name != player_recipe.name {
                    undiscovered_recipes.push(recipe_template.clone());
                }       
            }
        }

        let mut valid_undiscovered_recipes = Vec::new();

        for undiscovered_recipe in undiscovered_recipes.iter() { 

            let mut all_matched = true;

            for req in undiscovered_recipe.req.iter() {

                let mut req_matched = false;

                for reagent in experiment_reagents.iter() {

                    if reagent.subclass == req.req_type {
                        req_matched = true;
                    }
                }

                if !req_matched {
                    all_matched = false;
                }
            }

            if all_matched {
                valid_undiscovered_recipes.push(undiscovered_recipe);
            }
        }

        debug!("Valid undiscovered recipes: {:?}", valid_undiscovered_recipes);
    }
}

pub struct ExperimentPlugin;

impl Plugin for ExperimentPlugin {
    fn build(&self, app: &mut App) {
        let experiments = Experiments(Vec::new());

        app.insert_resource(experiments);
    }
}
