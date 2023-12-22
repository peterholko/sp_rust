use bevy::prelude::*;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::templates::Templates;

pub const BLEED: &str = "Bleed";
pub const DEEPWOUND: &str = "Deep Wound";
pub const CONCUSSED: &str = "Concussed";
pub const IMPALED: &str = "Impaled";
pub const BACKSTABBED: &str = "Backstabbed";
pub const DAZED: &str = "Dazed";
pub const DISARMED: &str = "Disarmed";
pub const DEMORALIZINGSHOUT: &str = "Demoralizing Shout";
pub const EXPOSEDARMOR: &str = "Exposed Armor";
pub const HAMSTRUNG: &str = "Hamstrung";
pub const FEAR: &str = "Fear";
pub const STUNNED: &str = "Stunned";


#[derive(Debug, Clone, Reflect, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Effect {
    Bleed,
    DeepWound,
    Concussed,
    Impaled,
    Backstabbed,
    Dazed,
    Disarmed,
    DemoralizingShout,
    ExposedArmor,
    Hamstrung,
    Fear,
    Stunned
}

impl Effect {

    pub fn to_str(self) -> String {
        match self {
            Effect::Bleed => BLEED.to_string(),
            Effect::DeepWound => DEEPWOUND.to_string(),
            Effect::Concussed => CONCUSSED.to_string(),
            Effect::Impaled => IMPALED.to_string(),
            Effect::Backstabbed => BACKSTABBED.to_string(),
            Effect::Dazed => DAZED.to_string(),
            Effect::Disarmed => DISARMED.to_string(),
            Effect::DemoralizingShout => DEMORALIZINGSHOUT.to_string(),
            Effect::ExposedArmor => EXPOSEDARMOR.to_string(),
            Effect::Hamstrung => HAMSTRUNG.to_string(),
            Effect::Fear => FEAR.to_string(),
            Effect::Stunned => STUNNED.to_string()
        }
    }

    pub fn from_string(effect_string: &String) -> Self {
        match effect_string.as_str() {
            BLEED => Effect::Bleed,
            DEEPWOUND => Effect::DeepWound,
            CONCUSSED => Effect::Concussed,
            IMPALED => Effect::Impaled,
            BACKSTABBED => Effect::Backstabbed,
            DAZED => Effect::Dazed,
            DISARMED => Effect::Disarmed,
            DEMORALIZINGSHOUT => Effect::DemoralizingShout,
            EXPOSEDARMOR => Effect::ExposedArmor,
            HAMSTRUNG => Effect::Hamstrung,
            FEAR => Effect::Fear,
            STUNNED => Effect::Stunned,
            _ => panic!("Invalid Effect"),
        }
    }
}

type Duration = i32;
type Amplifier = f32;
type Stacks = i32;

#[derive(Debug, Component, Clone)]
pub struct Effects(pub HashMap<Effect, (Duration, Amplifier, Stacks)>);

impl Effects {

        // Value returned is between 0.0 and 1.0
        fn get_damage_effects(self, templates: &Res<Templates>) -> f32 {
            for (effect, (_duration, _amplifier, _stacks)) in self.0.iter() {
                let effect_template = templates
                    .effect_templates
                    .get(&effect.clone().to_str())
                    .expect("Effect missing from templates");
    
                if let Some(effect_damage) = effect_template.damage {
                    let modifier = 1.0 + effect_damage; // atk is negative in the template file
                    return modifier;
                }
            }
    
            // No modifier if 1.0 is returned
            return 1.0;
        }
    
    pub fn get_speed_effects(&self, templates: &Res<Templates>) -> f32 {
        // Get effects
        for (effect, (_duration, _amplifier, _stackss)) in self.0.iter() {
            let effect_template = templates
            .effect_templates
            .get(&effect.clone().to_str())
            .expect("Effect missing from templates");

            if let Some(effect_speed) = effect_template.speed {
                let modifier = 1.0 + effect_speed;
                return modifier;
            }
        }

        return 1.0;
    }

    
}