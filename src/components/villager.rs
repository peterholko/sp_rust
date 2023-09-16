use bevy::prelude::*;

use crate::game::Position;



#[derive(Clone, Component, Debug)]
pub struct ThirstyScorer;

#[derive(Clone, Component, Debug)]
pub struct FindDrinkScorer;

#[derive(Clone, Component, Debug)]
pub struct DrinkDistanceScorer;

#[derive(Clone, Component, Debug)]
pub struct TransferDrinkScorer;

#[derive(Clone, Component, Debug)]
pub struct HasDrinkScorer;

#[derive(Clone, Component, Debug)]
pub struct MoveToDrink {
    pub dest: Position
}

#[derive(Clone, Component, Debug)]
pub struct FindFoodScorer;

#[derive(Clone, Component, Debug)]
pub struct FoodDistanceScorer;

#[derive(Clone, Component, Debug)]
pub struct TransferFoodScorer;

#[derive(Clone, Component, Debug)]
pub struct HasFoodScorer;

#[derive(Clone, Component, Debug)]
pub struct MoveToFood {
    pub dest: Position
}

#[derive(Clone, Component, Debug)]
pub struct ShelterAvailable;

#[derive(Clone, Component, Debug)]
pub struct ShelterUnavailable;

// Tag to indicate extreme thirst
#[derive(Clone, Component, Debug)]
pub struct Dehydrated;

#[derive(Clone, Component, Debug)]
pub struct MoveToInProgress;

#[derive(Clone, Component, Debug)]
pub struct Drink {
    pub until: f32,
}

#[derive(Clone, Component, Debug)]
pub struct MoveToWaterSource;

#[derive(Clone, Component, Debug)]
pub struct FindDrink;


#[derive(Clone, Component, Debug)]
pub struct TransferDrink;

#[derive(Component, Debug)]
pub struct Thirst {
    pub per_tick: f32,
    pub thirst: f32,
}

impl Thirst {
    pub fn new(thirst: f32, per_tick: f32) -> Self {
        Self { thirst, per_tick }
    }

    pub fn add(&mut self, value: f32) {
        if self.thirst + value > 100.0 {
            self.thirst = 100.0;
        } else if self.thirst + value < 0.0 {
            self.thirst = 0.0;
        } else {
            self.thirst += value;
        }
    }

    pub fn update_by_tick_amount(&mut self, extra_mod: f32) {
        Self::add(self, self.per_tick * extra_mod)        
    }
}

// Hunger
#[derive(Clone, Component, Debug)]
pub struct HungryScorer;

// Starving is an tag to indicate extreme hunger
#[derive(Clone, Component, Debug)]
pub struct Starving;

#[derive(Clone, Component, Debug)]
pub struct Eat;

#[derive(Clone, Component, Debug)]
pub struct MoveToFoodSource;

#[derive(Clone, Component, Debug)]
pub struct FindFood;

#[derive(Clone, Component, Debug)]
pub struct TransferFood;

#[derive(Component, Debug)]
pub struct Hunger {
    pub hunger: f32,
    pub per_tick: f32,
}

impl Hunger {
    pub fn new(hunger: f32, per_tick: f32) -> Self {
        Self { hunger, per_tick }
    }

    pub fn update(&mut self, value: f32) {
        if self.hunger + value > 100.0 {
            self.hunger = 100.0;
        } else if self.hunger + value < 0.0 {
            self.hunger = 0.0;
        } else {
            self.hunger += value;
        }
    }

    pub fn update_by_tick_amount(&mut self, extra_mod: f32) {
        Self::update(self, self.per_tick * extra_mod)        
    }
}

#[derive(Clone, Component, Debug)]
pub struct FindShelterScorer;

#[derive(Clone, Component, Debug)]
pub struct ShelterDistanceScorer;

// Sleep
#[derive(Clone, Component, Debug)]
pub struct DrowsyScorer;


#[derive(Clone, Component, Debug)]
pub struct FindShelter;

// Tag to indicate extreme drowsinest 
#[derive(Clone, Component, Debug)]
pub struct Exhausted;

#[derive(Clone, Component, Debug)]
pub struct Sleep;

#[derive(Clone, Component, Debug)]
pub struct MoveToSleepPos;

#[derive(Clone, Component, Debug)]
pub struct MoveToShelter {
    pub dest: Position
}

#[derive(Component, Debug)]
pub struct Tired {
    pub tired: f32,
    pub per_tick: f32,
}


impl Tired {
    pub fn new(tired: f32, per_tick: f32) -> Self {
        Self { tired, per_tick }
    }

    pub fn update(&mut self, value: f32) {
        if self.tired + value > 100.0 {
            self.tired = 100.0;
        } else if self.tired + value < 0.0 {
            self.tired = 0.0;
        } else {
            self.tired += value;
        }
    }    

    pub fn update_by_tick_amount(&mut self, extra_mod: f32) {
        Self::update(self, self.per_tick * extra_mod)        
    }
}

#[derive(Clone, Component, Debug)]
pub struct ProcessOrder;

#[derive(Clone, Component, Debug)]
pub struct GoodMorale;

#[derive(Component, Debug)]
pub struct Morale {
    pub morale: f32,
}

impl Morale {
    pub fn new(morale: f32) -> Self {
        Self { morale }
    }
}

