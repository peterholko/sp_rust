use bevy::prelude::*;
use big_brain::prelude::*;

use crate::game::Position;


#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct EnemyDistanceScorer;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct IdleScorer;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct ThirstyScorer;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct FindDrinkScorer;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct DrinkDistanceScorer;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct TransferDrinkScorer;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct HasDrinkScorer;

// Hunger
#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct HungryScorer;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct FindFoodScorer;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct FoodDistanceScorer;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct TransferFoodScorer;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct HasFoodScorer;

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct Flee;

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct MoveToDrink {
    pub dest: Position
}

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct MoveToFood {
    pub dest: Position
}

#[derive(Debug, Clone, Component)]
pub struct ShelterAvailable;

#[derive(Debug, Clone, Component)]
pub struct ShelterUnavailable;

// Tag to indicate extreme thirst
#[derive(Debug, Clone, Component)]
pub struct Dehydrated;

#[derive(Debug, Clone, Component)]
pub struct MoveToInProgress;

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct Drink {
    pub until: f32,
}

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct MoveToWaterSource;

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct FindDrink;

#[derive(Debug, Clone, Component)]
pub struct NoDrinks;

#[derive(Debug, Clone, Component)]
pub struct NoFood;


#[derive(Debug, Clone, Component, ActionBuilder)]
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



// Starving is an tag to indicate extreme hunger
#[derive(Debug, Clone, Component)]
pub struct Starving;

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct Eat;

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct MoveToFoodSource;

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct FindFood;

#[derive(Debug, Clone, Component, ActionBuilder)]
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

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct FindShelterScorer;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct ShelterDistanceScorer;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct NearShelterScorer;

// Sleep
#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct DrowsyScorer;


#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct FindShelter;

// Tag to indicate extreme drowsinest 
#[derive(Debug, Clone, Component)]
pub struct Exhausted;

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct Sleep;

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct MoveToSleepPos;

#[derive(Debug, Clone, Component)]
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

#[derive(Component, Debug)]
pub struct Heat {
    pub heat: f32,
}

impl Heat {
    pub fn new(heat: f32) -> Self {
        Self { heat }
    }

    pub fn update(&mut self, value: f32) {
        if self.heat + value > 100.0 {
            self.heat = 100.0;
        } else if self.heat + value < 0.0 {
            self.heat = 0.0;
        } else {
            self.heat += value;
        }
    }    
}

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct ProcessOrder;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct   GoodMorale;

#[derive(Component, Debug)]
pub struct Morale {
    pub morale: f32,
}

impl Morale {
    pub fn new(morale: f32) -> Self {
        Self { morale }
    }
}

