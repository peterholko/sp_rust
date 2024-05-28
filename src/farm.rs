use std::collections::HashMap;

use bevy::prelude::*;

use crate::game::GameTick;


#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CropStages {
    Seed,
    Sprout,
    Sapling,
    Mature,
    Dead,
}


#[derive(Debug, Clone)]
pub struct Crop {
    pub structure: i32,
    pub crop_type: String,
    pub crop_quantity: i32,
    pub stage: CropStages,
    pub stage_start: i32,
    pub stage_end: i32,
}

impl Crop {

}

#[derive(Resource, Deref, DerefMut, Debug)]
pub struct Crops(HashMap<i32, Crop>);


impl Crops {
    pub fn plant(&mut self, game_tick: i32, structure: i32, seed: String, quantity: i32) {
        if let Some(crop) = self.get_mut(&structure) {  
            // Check crop type
            if crop.stage == CropStages::Seed {
                if crop.crop_type == seed {
                    crop.crop_quantity += quantity;
                    info!("Update Crop: {:?}", self);
                }
            } else if crop.stage == CropStages::Dead {
                // Replan crop
                crop.crop_type = seed;
                crop.crop_quantity = quantity;
                crop.stage = CropStages::Seed;
                crop.stage_start = game_tick;
                crop.stage_end = game_tick + 60;                
            }
        } else {
            self.insert(structure, Crop {
                structure,
                crop_type: "Wheat".to_string(),
                crop_quantity: quantity,
                stage: CropStages::Seed,
                stage_start: game_tick,
                stage_end: game_tick + 60, // TODO Determine this based on crop type / player skill
            });
            info!("New Crop: {:?}", self);
        }
    }

    pub fn harvest(&mut self, structure: i32, quantity: i32) -> Option<Crop> {
        if let Some(crop) = self.get_mut(&structure) {
            if crop.stage == CropStages::Mature {
                info!("Harvested Crop");
                if crop.crop_quantity > quantity {
                    crop.crop_quantity -= quantity;
                    return Some(crop.clone());
                } else {
                    let cloned_crop = crop.clone();
                    self.remove(&structure);
                    return Some(cloned_crop);
                }

            }
        }

        return None;
    }
}

fn crop_system(
    game_tick: ResMut<GameTick>,
    mut crops: ResMut<Crops>,
) {
    // Iterate through crops and check if start end is greater or equal to game tick
    for (_structure, crop) in crops.iter_mut() {        
        if crop.stage_end <= game_tick.0 {
            info!("Crop {:?} has reached stage end.", crop);
            match crop.stage {
                CropStages::Seed => {
                    crop.stage = CropStages::Sprout;
                    crop.stage_start = game_tick.0;
                    crop.stage_end = game_tick.0 + 50;
                }
                CropStages::Sprout => {
                    crop.stage = CropStages::Sapling;
                    crop.stage_start = game_tick.0;
                    crop.stage_end = game_tick.0 + 50;
                }
                CropStages::Sapling => {
                    crop.stage = CropStages::Mature;
                    crop.stage_start = game_tick.0;
                    crop.stage_end = game_tick.0 + 50;
                }
                CropStages::Mature => {
                }
                CropStages::Dead => {
                    // Remove crop
                }
            }
        }
    }

}

pub struct FarmPlugin;

impl Plugin for FarmPlugin {
    fn build(&self, app: &mut App) {
        let crops = Crops(HashMap::new());

        app.insert_resource(crops);

        app.add_systems(Update, crop_system);
    }
}
