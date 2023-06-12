use bevy::prelude::*;

use crate::templates::{RecipeTemplates, ResReq};

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
}

pub struct ExperimentPlugin;

impl Plugin for ExperimentPlugin {
    fn build(&self, app: &mut App) {
        let experiments = Experiments(Vec::new());

        app.insert_resource(experiments);
    }
}
