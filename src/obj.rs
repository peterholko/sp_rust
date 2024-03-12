use bevy::prelude::*;

use std::collections::HashMap;

use rand::{random, Rng};

use crate::ids::Ids;
use crate::effect::Effects;
use crate::event::{MapEvent, MapEvents, VisibleEvent};
use crate::game::{
    self, BaseAttrs, Class, GameTick, Id, Misc, Name, PlayerId, Position, State, Stats, Subclass, Template, Viewshed
};
use crate::item::{Item, Items};
use crate::map::TileType;
use crate::skill::{Skill, Skills};
use crate::templates::{ObjTemplate, ObjTemplates, SkillTemplate, SkillTemplates, Templates};

pub const TEMPLATE: &str = "template";
pub const POSITION: &str = "position";

pub const CLASS_STRUCTURE: &str = "structure";
pub const CLASS_UNIT: &str = "unit";
pub const CLASS_CORPSE: &str = "corpse";

pub const SUBCLASS_HERO: &str = "hero";
pub const SUBCLASS_VILLAGER: &str = "villager";
pub const SUBCLASS_SHELTER: &str = "shelter";
pub const SUBCLASS_MERCHANT: &str = "merchant";

// States
pub const STATE_NONE: &str = "none";
pub const STATE_MOVING: &str = "moving";
pub const STATE_ATTACKING: &str = "attacking";
pub const STATE_CASTING: &str = "casting";
pub const STATE_DEAD: &str = "dead";
pub const STATE_FOUNDED: &str = "founded";
pub const STATE_PROGRESSING: &str = "progressing";
pub const STATE_BUILDING: &str = "building";
pub const STATE_UPGRADING: &str = "upgrading";
pub const STATE_STALLED: &str = "stalled";
pub const STATE_GATHERING: &str = "gathering";
pub const STATE_REFINING: &str = "refining";
pub const STATE_CRAFTING: &str = "crafting";
pub const STATE_EXPLORING: &str = "exploring";
pub const STATE_DRINKING: &str = "drinking";
pub const STATE_EATING: &str = "eating";
pub const STATE_SLEEPING: &str = "sleeping";

// Attributes
pub const CREATIVITY: &str = "Creativity";
pub const DEXTERITY: &str = "Dexterity";
pub const ENDURANCE: &str = "Endurance";
pub const FOCUS: &str = "Focus";
pub const INTELLECT: &str = "Intellect";
pub const SPIRIT: &str = "Spirit";
pub const STRENGTH: &str = "Strength";
pub const TOUGHNESS: &str = "Toughness";

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum HeroClassList {
    Warrior,
    Ranger,
    Mage,
    None,
}

#[derive(Bundle, Clone)]
pub struct Obj {
    pub id: Id,
    pub player_id: PlayerId,
    pub position: Position,
    pub name: Name,
    pub template: Template,
    pub class: Class,
    pub subclass: Subclass,
    pub state: State,
    pub viewshed: Viewshed,
    pub misc: Misc,
    pub stats: Stats,
    pub effects: Effects,
}

impl Obj {
    pub fn create(
        player_id: i32,
        template_name: String,
        pos: Position,
        state: State,
        commands: &mut Commands,
        ids: &mut ResMut<Ids>,
        map_events: &mut ResMut<MapEvents>,
        game_tick: &Res<GameTick>,
        templates: &Res<Templates>,
    ) -> (i32, Entity) {
        let template = ObjTemplate::get_template_by_name(template_name, &templates);
        let obj_id = ids.new_obj_id();

        let obj = Obj {
            id: Id(obj_id),
            player_id: PlayerId(player_id),
            position: pos,
            name: Name(template.name),
            template: Template(template.template.clone()),
            class: Class(template.class),
            subclass: Subclass(template.subclass),
            state: state,
            viewshed: Viewshed {
                range: template.base_vision.unwrap_or(0) as u32,
            },
            misc: Misc {
                image: str::replace(template.template.as_str(), " ", "").to_lowercase(),
                hsl: Vec::new(),
                groups: Vec::new(),
            },
            stats: Stats {
                hp: template.base_hp.unwrap_or(100),
                base_hp: template.base_hp.unwrap_or(100),
                stamina: template.base_stamina,
                base_stamina: template.base_stamina,
                base_def: template.base_def.unwrap_or(0),
                base_damage: template.base_dmg,
                damage_range: template.dmg_range,
                base_speed: template.base_speed,
                base_vision: template.base_vision,
            },
            effects: Effects(HashMap::new()),
        };

        // Spawn entity
        let entity_id = commands.spawn(obj).id();

        // Create mappings
        ids.new_obj(obj_id, player_id, entity_id);

        // Create new object event
        map_events.new(
            obj_id,
            game_tick.0 + 1,
            VisibleEvent::NewObjEvent { new_player: false }
        );

        (obj_id, entity_id)
    }

    pub fn create_nospawn(
        ids: &mut ResMut<Ids>,
        player_id: i32,
        template_name: String,
        pos: Position,
        state: State,
        templates: &Res<Templates>,
    ) -> Obj {
        let template = ObjTemplate::get_template_by_name(template_name, &templates);
        let obj_id = ids.new_obj_id();

        let obj = Obj {
            id: Id(obj_id),
            player_id: PlayerId(player_id),
            position: pos,
            name: Name(template.name),
            template: Template(template.template.clone()),
            class: Class(template.class),
            subclass: Subclass(template.subclass),
            state: state,
            viewshed: Viewshed {
                range: template.base_vision.unwrap_or(0) as u32,
            },
            misc: Misc {
                image: str::replace(template.template.as_str(), " ", "").to_lowercase(),
                hsl: Vec::new(),
                groups: Vec::new(),
            },
            stats: Stats {
                hp: template.base_hp.unwrap_or(100),
                base_hp: template.base_hp.unwrap_or(100),
                stamina: template.base_stamina,
                base_stamina: template.base_stamina,
                base_def: template.base_def.unwrap_or(0),
                base_damage: template.base_dmg,
                damage_range: template.dmg_range,
                base_speed: template.base_speed,
                base_vision: template.base_vision,
            },
            effects: Effects(HashMap::new()),
        };

        return obj;
    }

    pub fn state_to_enum(state: String) -> State {
        match state.as_str() {
            STATE_NONE => State::None,
            STATE_MOVING => State::Moving,
            STATE_DEAD => State::Dead,
            STATE_FOUNDED => State::Founded,
            STATE_PROGRESSING => State::Progressing,
            STATE_BUILDING => State::Building,
            STATE_UPGRADING => State::Upgrading,
            STATE_STALLED => State::Stalled,
            STATE_GATHERING => State::Gathering,
            STATE_REFINING => State::Refining,
            STATE_CRAFTING => State::Crafting,
            STATE_EXPLORING => State::Exploring,
            STATE_DRINKING => State::Drinking,
            STATE_EATING => State::Eating,
            STATE_SLEEPING => State::Sleeping,
            STATE_CASTING => State::Casting,
            _ => State::None,
        }
    }

    pub fn state_to_str(state: State) -> String {
        let state_string = match state {
            State::None => STATE_NONE,
            State::Moving => STATE_MOVING,
            State::Dead => STATE_DEAD,
            State::Founded => STATE_FOUNDED,
            State::Progressing => STATE_PROGRESSING,
            State::Building => STATE_BUILDING,
            State::Upgrading => STATE_UPGRADING,
            State::Stalled => STATE_STALLED,
            State::Gathering => STATE_GATHERING,
            State::Refining => STATE_REFINING,
            State::Crafting => STATE_CRAFTING,
            State::Exploring => STATE_EXPLORING,
            State::Drinking => STATE_DRINKING,
            State::Eating => STATE_EATING,
            State::Sleeping => STATE_SLEEPING,
            State::Casting => STATE_CASTING,
            _ => STATE_NONE,
        };

        return state_string.to_string();
    }

    pub fn is_dead(obj_state: &State) -> bool {
        return *obj_state == State::Dead;
    }

    pub fn get_capacity(template: &String, obj_templates: &ObjTemplates) -> i32 {
        for obj_template in obj_templates.iter() {
            if obj_template.template == *template {
                if let Some(capacity) = obj_template.capacity {
                    return capacity;
                } else {
                    info!(
                        "No capacity found for obj template: {:?} defaulting to 0",
                        template
                    );
                    return 0;
                }
            }
        }

        info!("No template found for {:?}", template);

        return 0;
    }

    pub fn add_sound_obj_event(
        game_tick: i32,
        sound: String,
        obj_id: &Id,
        map_events: &mut ResMut<MapEvents>,
    ) {
        let sound_event = VisibleEvent::SoundObjEvent {
            sound: sound,
            intensity: 2,
        };

        map_events.new(obj_id.0, game_tick, sound_event);
    }

    pub fn generate_hero_attrs() -> BaseAttrs {
        let attrs = BaseAttrs {
            creativity: 10,
            dexterity: 10,
            endurance: 10,
            focus: 10,
            intellect: 10,
            spirit: 10,
            strength: 10,
            toughness: 10,
        };
    
        return attrs;
    }
}
