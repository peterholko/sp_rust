use bevy::ecs::query::WorldQuery;
use bevy::prelude::*;
use big_brain::prelude::*;
use pathfinding::prelude::directions::E;

use std::collections::{HashMap, HashSet};

use crate::ai::{Drink, HighMorale, Morale, ProcessOrder, Thirst, Thirsty};
use crate::combat::{Combat, CombatQuery};
use crate::game::{
    is_none_state, is_pos_empty, BaseAttrs, Class, ClassStructure, Clients, ExploredMap, GameTick,
    HeroClassList, Id, Ids, MapEvent, MapEvents, MapObjQuery, Misc, Name, NetworkReceiver, Obj,
    Order, PlayerId, Position, State, Stats, StructureAttrs, Subclass, SubclassHero,
    SubclassVillager, Template, Viewshed, VillagerAttrs, VisibleEvent, VisibleEvents, BUILDING,
    CREATIVITY, DEAD, DEXTERITY, ENDURANCE, FOCUS, FOUNDED, INTELLECT, MOVING, NONE, PROGRESSING,
    SPIRIT, STRENGTH, TOUGHNESS,
};
use crate::item::{self, Item, Items};
use crate::map::Map;
use crate::network::{self, send_to_client, ResponsePacket, StructureList};
use crate::obj::{self};
use crate::recipe::{Recipe, Recipes};
use crate::resource::{Resource, Resources};
use crate::skill::{Skill, Skills};
use crate::structure::{self, Structure, Deed, Deeds};
use crate::templates::{ObjTemplate, ResReq, SkillTemplate, SkillTemplates, Templates};
use crate::villager::Villager;

#[derive(Resource, Deref, DerefMut)]
pub struct PlayerEvents(pub HashMap<i32, PlayerEvent>);

#[derive(Resource, Clone, Debug)]
pub enum PlayerEvent {
    NewPlayer {
        player_id: i32,
    },
    Move {
        player_id: i32,
        x: i32,
        y: i32,
    },
    Attack {
        player_id: i32,
        attack_type: String,
        source_id: i32,
        target_id: i32,
    },
    Combo {
        player_id: i32,
        source_id: i32,
        combo_type: String,
    },
    Gather {
        player_id: i32,
        source_id: i32,
        res_type: String,
    },
    InfoObj {
        player_id: i32,
        id: i32,
    },
    InfoSkills {
        player_id: i32,
        id: i32,
    },
    InfoAttrs {
        player_id: i32,
        id: i32,
    },
    InfoAdvance {
        player_id: i32,
        id: i32,
    },
    InfoTile {
        player_id: i32,
        x: i32,
        y: i32,
    },
    InfoInventory {
        player_id: i32,
        id: i32,
    },
    InfoItem {
        player_id: i32,
        id: i32,
    },
    InfoItemByName {
        player_id: i32,
        name: String,
    },
    InfoItemTransfer {
        player_id: i32,
        source_id: i32,
        target_id: i32,
    },
    ItemTransfer {
        player_id: i32,
        target_id: i32,
        item_id: i32,
    },
    ItemSplit {
        player_id: i32,
        item_id: i32,
        quantity: i32,
    },
    OrderFollow {
        player_id: i32,
        source_id: i32,
    },
    OrderGather {
        player_id: i32,
        source_id: i32,
        res_type: String,
    },
    OrderRefine {
        player_id: i32,
        structure_id: i32,
    },
    OrderCraft {
        player_id: i32,
        structure_id: i32,
        recipe_name: String,
    },
    OrderExplore {
        player_id: i32,
        villager_id: i32,
    },
    StructureList {
        player_id: i32,
    },
    CreateFoundation {
        player_id: i32,
        source_id: i32,
        structure_name: String,
    },
    Build {
        player_id: i32,
        source_id: i32,
        structure_id: i32,
    },
    Survey {
        player_id: i32,
        source_id: i32,
    },
    Explore {
        player_id: i32,
    },
    AssignList {
        player_id: i32,
    },
    Assign {
        player_id: i32,
        source_id: i32,
        target_id: i32,
    },
    Equip {
        player_id: i32,
        item_id: i32,
        status: bool,
    },
    RecipeList {
        player_id: i32,
        structure_id: i32,
    },
    Use {
        player_id: i32,
        item_id: i32,
    },
    Remove {
        player_id: i32,
        structure_id: i32,
    },
    Advance {
        player_id: i32,
        id: i32,
    },    
}

#[derive(WorldQuery)]
struct CoreQuery {
    entity: Entity,
    id: &'static Id,
    player_id: &'static PlayerId,
    pos: &'static Position,
    name: &'static Name,
    class: &'static Class,
    subclass: &'static Subclass,
    template: &'static Template,
    state: &'static State,
    misc: &'static Misc,
}

#[derive(WorldQuery)]
struct ItemTransferQuery {
    entity: Entity,
    id: &'static Id,
    player_id: &'static PlayerId,
    pos: &'static Position,
    name: &'static Name,
    class: &'static Class,
    subclass: &'static Subclass,
    state: &'static State,
    structure_attrs: Option<&'static StructureAttrs>,
}

#[derive(WorldQuery)]
#[world_query(mutable, derive(Debug))]
struct StructureQuery {
    entity: Entity,
    id: &'static Id,
    player_id: &'static PlayerId,
    pos: &'static Position,
    name: &'static Name,
    class: &'static Class,
    subclass: &'static Subclass,
    template: &'static Template,
    state: &'static State,
    attrs: &'static mut StructureAttrs,
}

#[derive(WorldQuery)]
#[world_query(mutable, derive(Debug))]
struct VillagerQuery {
    entity: Entity,
    id: &'static Id,
    player_id: &'static PlayerId,
    pos: &'static Position,
    name: &'static Name,
    class: &'static Class,
    subclass: &'static Subclass,
    state: &'static State,
    misc: &'static Misc,
    attrs: &'static mut VillagerAttrs,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        // Initialize events
        let player_events: PlayerEvents = PlayerEvents(HashMap::new());

        app.add_system(message_broker_system)
            .add_system(new_player_system)
            .add_system(move_system)
            .add_system(attack_system)
            .add_system(gather_system)
            .add_system(info_obj_system)
            .add_system(info_skills_system)
            .add_system(info_attrs_system)
            .add_system(info_advance_system)
            .add_system(info_tile_system)
            .add_system(info_item_system)
            .add_system(item_transfer_system)
            .add_system(item_split_system)
            .add_system(order_follow_system)
            .add_system(order_gather_system)
            .add_system(order_refine_system)
            .add_system(order_craft_system)
            .add_system(structure_list_system)
            .add_system(create_foundation_system)
            .add_system(build_system)
            .add_system(explore_system)
            .add_system(assign_list_system)
            .add_system(assign_system)
            .add_system(equip_system)
            .add_system(recipe_list_system)
            .add_system(order_explore_system)
            .add_system(use_item_system)
            .insert_resource(player_events);
    }
}

fn message_broker_system(
    client_to_game_receiver: Res<NetworkReceiver>,
    mut player_events: ResMut<PlayerEvents>,
    mut ids: ResMut<Ids>,
) {
    if let Ok(evt) = client_to_game_receiver.try_recv() {
        println!("{:?}", evt);

        player_events.insert(ids.player_event, evt.clone());

        ids.player_event += 1;
    }
}

fn new_player_system(
    mut events: ResMut<PlayerEvents>,
    mut commands: Commands,
    game_tick: ResMut<GameTick>,
    mut ids: ResMut<Ids>,
    mut map_events: ResMut<MapEvents>,
    mut items: ResMut<Items>,
    mut skills: ResMut<Skills>,
    mut recipes: ResMut<Recipes>,
    mut deeds: ResMut<Deeds>,
    templates: Res<Templates>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::NewPlayer { player_id } => {
                events_to_remove.push(*event_id);

                new_player(
                    *player_id,
                    &mut commands,
                    &mut ids,
                    &mut map_events,
                    &mut items,
                    &mut skills,
                    &mut recipes,
                    &mut deeds,
                    &templates,
                    &game_tick,
                );
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(index);
    }
}

fn move_system(
    mut events: ResMut<PlayerEvents>,
    game_tick: ResMut<GameTick>,
    mut ids: ResMut<Ids>,
    clients: Res<Clients>,
    mut map_events: ResMut<MapEvents>,
    map: Res<Map>,
    hero_query: Query<
        (
            Entity,
            &Id,
            &Position,
            &PlayerId,
            &Name,
            &Template,
            &Class,
            &Subclass,
            &State,
        ),
        With<SubclassHero>,
    >,
    query: Query<MapObjQuery>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::Move { player_id, x, y } => {
                debug!("Move Event: {:?}", event);
                events_to_remove.push(*event_id);

                let player_id = player_id;

                for (
                    entity_id,
                    obj_id,
                    pos,
                    obj_player_id,
                    _name,
                    _template,
                    _class,
                    _subclass,
                    _state,
                ) in hero_query.iter()
                {
                    // Check find hero from Move Event player
                    if *player_id != obj_player_id.0 {
                        continue;
                    }

                    if !Map::is_passable(*x, *y, &map) {
                        println!("Position is not passable");
                        let error = ResponsePacket::Error {
                            errmsg: "Tile is not passable.".to_owned(),
                        };
                        send_to_client(*player_id, error, &clients);
                        break;
                    };

                    if !is_pos_empty(*player_id, *x, *y, &query) {
                        println!("Position is not empty");
                        let error = ResponsePacket::Error {
                            errmsg: "Tile is occupied.".to_owned(),
                        };
                        send_to_client(*player_id, error, &clients);
                        break;
                    }

                    // Add State Change Event to Moving
                    let state_change_event = VisibleEvent::StateChangeEvent {
                        new_state: MOVING.to_string(),
                    };

                    map_events.new(
                        ids.new_map_event_id(),
                        entity_id,
                        obj_id,
                        obj_player_id,
                        pos,
                        game_tick.0,
                        state_change_event,
                    );

                    // Add Move Event
                    let move_event = VisibleEvent::MoveEvent {
                        dst_x: *x,
                        dst_y: *y,
                    };

                    map_events.new(
                        ids.new_map_event_id(),
                        entity_id,
                        obj_id,
                        obj_player_id,
                        pos,
                        game_tick.0 + 12, // in the future
                        move_event,
                    );
                }
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn attack_system(
    mut commands: Commands,
    mut events: ResMut<PlayerEvents>,
    game_tick: ResMut<GameTick>,
    mut ids: ResMut<Ids>,
    clients: Res<Clients>,
    mut map_events: ResMut<MapEvents>,
    mut items: ResMut<Items>,
    mut skills: ResMut<Skills>,
    templates: Res<Templates>,
    mut query: Query<CombatQuery>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::Attack {
                player_id,
                attack_type,
                source_id,
                target_id,
            } => {
                events_to_remove.push(*event_id);

                let Some(attacker_entity) = ids.get_entity(*source_id) else {
                    error!("Cannot find attacker entity from id: {:?}", source_id);
                    continue;
                };

                let Some(target_entity) = ids.get_entity(*target_id) else {
                    error!("Cannot find target entity from id: {:?}", target_id);
                    continue;
                };

                let entities = [attacker_entity, target_entity];

                for a in query.iter() {
                    debug!("CombatQueryItem: {:?}", a);
                }

                let Ok([mut attacker, mut target]) = query.get_many_mut(entities) else {
                    error!("Cannot find attacker or target from entities {:?}", entities);
                    continue;
                };

                // Check if attacker is owned by player
                if attacker.player_id.0 != *player_id {
                    let packet = ResponsePacket::Error {
                        errmsg: "Attacker not owned by player.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }

                // Is target adjacent
                if Map::dist(*attacker.pos, *target.pos) > 1 {
                    let packet = ResponsePacket::Error {
                        errmsg: "Target is not adjacent.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }

                // Check if target is dead
                if target.state.0 == DEAD {
                    let packet = ResponsePacket::Error {
                        errmsg: "Target is dead.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }

                // Calculate and process damage
                let (damage, skill_updated) = Combat::process_damage(
                    attack_type.to_string(),
                    &attacker,
                    &mut target,
                    &mut commands,
                    &mut items,
                    &templates,
                );

                // Add visible damage event to broadcast to everyone nearby
                Combat::add_damage_event(
                    ids.new_map_event_id(),
                    game_tick.0,
                    attack_type.to_string(),
                    damage,
                    &attacker,
                    &target,
                    &mut map_events,
                );

                // Response to client with attack response packet
                let packet = ResponsePacket::Attack {
                    sourceid: *source_id,
                    attacktype: attack_type.clone(),
                    cooldown: 20,
                    stamina_cost: 5,
                };

                send_to_client(*player_id, packet, &clients);

                debug!("Skill gain: {:?}", skill_updated);

                if let Some(skill_updated) = skill_updated {
                    Skill::update(
                        attacker.id.0,
                        skill_updated.xp_type.clone(),
                        skill_updated.xp,
                        &mut skills,
                        &templates.skill_templates,
                    );

                    let skill_updated_packet = ResponsePacket::Xp {
                        id: attacker.id.0,
                        xp_type: skill_updated.xp_type,
                        xp: skill_updated.xp,
                    };

                    send_to_client(*player_id, skill_updated_packet, &clients);
                };
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn gather_system(
    mut events: ResMut<PlayerEvents>,
    game_tick: ResMut<GameTick>,
    mut ids: ResMut<Ids>,
    mut map_events: ResMut<MapEvents>,
    mut skills: ResMut<Skills>,
    hero_query: Query<
        (
            Entity,
            &Id,
            &Position,
            &PlayerId,
            &Name,
            &Template,
            &Class,
            &Subclass,
            &State,
        ),
        With<SubclassHero>,
    >,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::Gather {
                player_id,
                source_id,
                res_type,
            } => {
                debug!("PlayerEvent::Gather");
                events_to_remove.push(*event_id);

                for (
                    entity_id,
                    obj_id,
                    pos,
                    obj_player_id,
                    _name,
                    _template,
                    _class,
                    _subclass,
                    _state,
                ) in hero_query.iter()
                {
                    // Check find hero from Gather event
                    if *player_id != obj_player_id.0 {
                        continue;
                    }

                    let gather_event = VisibleEvent::GatherEvent {
                        res_type: res_type.clone(),
                    };

                    /*Skill::update(
                        obj_id.0,
                        "Mining".to_string(),
                        100,
                        &templates.skill_templates,
                        &mut skills,
                    );*/

                    map_events.new(
                        ids.new_map_event_id(),
                        entity_id,
                        obj_id,
                        obj_player_id,
                        pos,
                        game_tick.0 + 8, // in the future
                        gather_event,
                    );

                    debug!("Skills: {:?}", skills);
                }
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn info_obj_system(
    mut events: ResMut<PlayerEvents>,
    ids: ResMut<Ids>,
    clients: Res<Clients>,
    items: ResMut<Items>,
    skills: Res<Skills>,
    query: Query<CoreQuery>,
    attrs_query: Query<&BaseAttrs>,
    stats_query: Query<&Stats>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::InfoObj { player_id, id } => {
                events_to_remove.push(*event_id);

                let Some(entity) = ids.get_entity(*id) else {
                    error!("Cannot find entity for {:?}", id);
                    break;
                };

                let Ok(obj) = query.get(entity) else {
                    error!("Cannot find villager for {:?}", entity);
                    break;
                };

                let mut info_obj_packet;

                if obj.player_id.0 == *player_id {
                    let items_packet = Some(Item::get_by_owner_packet(*id, &items));
                    let skills_packet = Some(Skill::get_levels_by_owner(*id, &skills));

                    let mut attributes: HashMap<String, i32> = HashMap::new();

                    let mut effects = Some(Vec::new());

                    // Required stats for all objects
                    let mut hp = None;
                    let mut base_hp = None;
                    let mut base_def = None;

                    let mut damage_range = None;
                    let mut base_damage = None;
                    let mut base_speed = None;
                    let mut base_vision = None;

                    let mut stamina = None;
                    let mut base_stamina = None;

                    let mut structure = None;
                    let mut action = None;
                    let mut shelter = None;

                    let mut morale = None;
                    let mut order = None;

                    let mut total_weight = None;
                    let mut capacity = None;

                    //if obj.class.0.to_string() ==

                    if let Ok(attrs) = attrs_query.get(obj.entity) {
                        attributes.insert(CREATIVITY.to_string(), attrs.creativity);
                        attributes.insert(DEXTERITY.to_string(), attrs.dexterity);
                        attributes.insert(ENDURANCE.to_string(), attrs.endurance);
                        attributes.insert(FOCUS.to_string(), attrs.focus);
                        attributes.insert(INTELLECT.to_string(), attrs.intellect);
                        attributes.insert(SPIRIT.to_string(), attrs.spirit);
                        attributes.insert(STRENGTH.to_string(), attrs.strength);
                        attributes.insert(TOUGHNESS.to_string(), attrs.toughness);
                    }

                    if let Ok(stats) = stats_query.get(obj.entity) {
                        hp = Some(stats.hp);
                        base_hp = Some(stats.base_hp);
                        base_def = Some(stats.base_def);

                        damage_range = stats.damage_range;
                        base_damage = stats.base_damage;
                        base_speed = stats.base_speed;
                        base_vision = stats.base_vision;
                    }

                    info_obj_packet = ResponsePacket::InfoObj {
                        id: obj.id.0,
                        name: obj.name.0.to_string(),
                        template: obj.template.0.to_string(),
                        class: obj.class.0.to_string(),
                        subclass: obj.subclass.0.to_string(),
                        state: obj.state.0.to_string(),
                        image: obj.misc.image.clone(),
                        hsl: obj.misc.hsl.clone(),
                        items: items_packet,
                        skills: skills_packet,
                        attributes: Some(attributes),
                        effects: effects,
                        hp: hp,
                        stamina: stamina,
                        base_stamina: base_stamina,
                        base_hp: base_hp,
                        base_def: base_def,
                        dmg_range: damage_range,
                        base_dmg: base_damage,
                        base_speed: base_speed,
                        base_vision: base_vision,
                        structure: structure,
                        shelter: shelter,
                        action: action,
                        morale: morale,
                        order: order,
                        total_weight: total_weight,
                        capacity: capacity,
                    };
                } else {
                    // TODO add effects
                    let effects = Vec::new();
                    let mut items_packet = None;

                    // Add items if object is dead
                    if obj.state.0 == DEAD {
                        items_packet = Some(Item::get_by_owner_packet(*id, &items));
                    }

                    // Non player owned object
                    info_obj_packet = ResponsePacket::InfoObj {
                        id: obj.id.0,
                        name: obj.name.0.to_string(),
                        template: obj.template.0.to_string(),
                        class: obj.class.0.to_string(),
                        subclass: obj.subclass.0.to_string(),
                        state: obj.state.0.to_string(),
                        image: obj.misc.image.clone(),
                        hsl: obj.misc.hsl.clone(),
                        items: items_packet,
                        skills: None,
                        attributes: None,
                        effects: Some(effects),
                        hp: None,
                        stamina: None,
                        base_stamina: None,
                        base_hp: None,
                        base_def: None,
                        dmg_range: None,
                        base_dmg: None,
                        base_speed: None,
                        base_vision: None,
                        structure: None,
                        shelter: None,
                        action: None,
                        morale: None,
                        order: None,
                        total_weight: None,
                        capacity: None,
                    };
                }

                send_to_client(*player_id, info_obj_packet, &clients);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn info_skills_system(
    mut events: ResMut<PlayerEvents>,
    ids: ResMut<Ids>,
    clients: Res<Clients>,
    skills: Res<Skills>,
    templates: Res<Templates>,
    query: Query<CoreQuery>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::InfoSkills { player_id, id } => {
                events_to_remove.push(*event_id);

                let Some(entity) = ids.get_entity(*id) else {
                    error!("Cannot find entity for {:?}", id);
                    continue;
                };

                let Ok(obj) = query.get(entity) else {
                    error!("Cannot find villager for {:?}", entity);
                    continue;
                };

                if obj.player_id.0 == *player_id {
                    let obj_skills =
                        Skill::get_by_owner_packet(obj.id.0, &skills, &templates.skill_templates);

                    let info_skills_packet = ResponsePacket::InfoSkills {
                        id: *id,
                        skills: obj_skills,
                    };

                    send_to_client(*player_id, info_skills_packet, &clients);
                } else {
                    error!("Object {:?} is not owned by player {:?}", id, player_id);
                }
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn info_attrs_system(
    mut events: ResMut<PlayerEvents>,
    ids: ResMut<Ids>,
    clients: Res<Clients>,
    query: Query<CoreQuery>,
    attr_query: Query<&BaseAttrs>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::InfoAttrs { player_id, id } => {
                events_to_remove.push(*event_id);

                let Some(entity) = ids.get_entity(*id) else {
                    error!("Cannot find entity for {:?}", id);
                    continue;
                };

                let Ok(obj) = query.get(entity) else {
                    error!("Cannot find villager for {:?}", entity);
                    continue;
                };

                if obj.player_id.0 == *player_id {
                    if let Ok(attrs) = attr_query.get(entity) {
                        let mut attrs_packet = HashMap::new();

                        attrs_packet.insert(CREATIVITY.to_string(), attrs.creativity);
                        attrs_packet.insert(DEXTERITY.to_string(), attrs.dexterity);
                        attrs_packet.insert(ENDURANCE.to_string(), attrs.endurance);
                        attrs_packet.insert(FOCUS.to_string(), attrs.focus);
                        attrs_packet.insert(INTELLECT.to_string(), attrs.intellect);
                        attrs_packet.insert(SPIRIT.to_string(), attrs.spirit);
                        attrs_packet.insert(TOUGHNESS.to_string(), attrs.toughness);

                        let info_attrs_packet = ResponsePacket::InfoAttrs {
                            id: *id,
                            attrs: attrs_packet,
                        };

                        send_to_client(*player_id, info_attrs_packet, &clients);
                    } else {
                        error!("Cannot find attributes for {:?}", id);
                    }
                } else {
                    error!("Object {:?} is not owned by player {:?}", id, player_id);
                }
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn info_advance_system(
    mut events: ResMut<PlayerEvents>,
    game_tick: Res<GameTick>,
    mut ids: ResMut<Ids>,
    clients: Res<Clients>,
    mut map_events: ResMut<MapEvents>,
    query: Query<CoreQuery>,
    skills: Res<Skills>,
    templates: Res<Templates>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::InfoAdvance { player_id, id } => {
                events_to_remove.push(*event_id);

                let Some(entity) = ids.get_entity(*id) else {
                    error!("Cannot find entity for {:?}", id);
                    continue;
                };

                let Ok(obj) = query.get(entity) else {
                    error!("Cannot find obj for {:?}", entity);
                    continue;
                };

                if obj.player_id.0 == *player_id {
                    let (next_template, required_xp) = Skill::hero_advance(obj.template.0.clone());

                    let info_advance_packet = ResponsePacket::InfoAdvance {
                        id: obj.id.0,
                        rank: obj.template.0.clone(),
                        next_rank: next_template,
                        total_xp: Skill::get_total_xp(*id, &skills, &templates.skill_templates),
                        req_xp: required_xp,
                    };

                    send_to_client(*player_id, info_advance_packet, &clients);
                } else {
                    error!("Object {:?} is not owned by player {:?}", id, player_id);
                }
            }
            PlayerEvent::Advance { player_id, id } => {

                let Some(entity) = ids.get_entity(*id) else {
                    error!("Cannot find entity for {:?}", id);
                    continue;
                };

                let Ok(obj) = query.get(entity) else {
                    error!("Cannot find obj for {:?}", entity);
                    continue;
                };

                if obj.player_id.0 == *player_id {
                    let (next_template, _required_xp) = Skill::hero_advance(obj.template.0.clone());

                    //Add obj update event
                    let obj_update_event = VisibleEvent::UpdateObjEvent {
                        attr: obj::TEMPLATE.to_string(),
                        value: next_template.clone()
                    };

                    map_events.new(
                        ids.new_map_event_id(),
                        entity,
                        &obj.id,
                        &obj.player_id,
                        obj.pos,
                        game_tick.0,
                        obj_update_event,
                    );                    

                    let (new_next_template, new_required_xp) = Skill::hero_advance(next_template.clone());

                    let advance_packet = ResponsePacket::InfoAdvance {
                        id: obj.id.0,
                        rank: next_template.clone(),
                        next_rank: new_next_template,
                        total_xp: 0, // Advancing resets to zero
                        req_xp: new_required_xp,
                    };
                    
                    send_to_client(*player_id, advance_packet, &clients);
                }

                events_to_remove.push(*event_id); 
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn info_tile_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    map: Res<Map>,
    resources: Res<Resources>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::InfoTile { player_id, x, y } => {
                debug!("PlayerEvent::InfoTile x: {:?} y: {:?}", *x, *y);
                events_to_remove.push(*event_id);

                let tile_type = Map::tile_type(*x, *y, &map);

                let info_tile_packet: ResponsePacket = ResponsePacket::InfoTile {
                    x: *x,
                    y: *y,
                    name: Map::tile_name(tile_type),
                    mc: Map::movement_cost(tile_type),
                    def: Map::def_bonus(tile_type),
                    unrevealed: Resource::num_unrevealed_on_tile(
                        Position { x: *x, y: *y },
                        &resources,
                    ),
                    sanctuary: "true".to_owned(),
                    passable: Map::is_passable(*x, *y, &map),
                    wildness: "high".to_owned(),
                    resources: Resource::get_on_tile(Position { x: *x, y: *y }, &resources),
                };

                send_to_client(*player_id, info_tile_packet, &clients);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn info_item_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    mut items: ResMut<Items>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::InfoInventory { player_id, id } => {
                debug!("PlayerEvent::InfoInventory id: {:?}", id);
                events_to_remove.push(*event_id);

                let inventory_items = Item::get_by_owner_packet(*id, &items);

                let info_inventory_packet: ResponsePacket = ResponsePacket::InfoInventory {
                    id: *id,
                    cap: 100,
                    tw: 100,
                    items: inventory_items,
                };

                send_to_client(*player_id, info_inventory_packet, &clients);
            }
            PlayerEvent::InfoItem { player_id, id } => {
                events_to_remove.push(*event_id);

                let item = Item::get_packet(*id, &items);

                if let Some(item) = item {
                    let info_item_packet: ResponsePacket = ResponsePacket::InfoItem {
                        id: item.id,
                        owner: item.owner,
                        name: item.name,
                        quantity: item.quantity,
                        class: item.class,
                        subclass: item.subclass,
                        image: item.image,
                        weight: item.weight,
                        equipped: item.equipped,
                    };

                    send_to_client(*player_id, info_item_packet, &clients);
                }
            }
            PlayerEvent::InfoItemByName { player_id, name } => {
                debug!("PlayerEvent::InfoItemByName name: {:?}", name.clone());
                events_to_remove.push(*event_id);

                let item = Item::get_by_name_packet(name.clone(), &items);

                if let Some(item) = item {
                    let info_item_packet: ResponsePacket = ResponsePacket::InfoItem {
                        id: item.id,
                        owner: item.owner,
                        name: item.name,
                        quantity: item.quantity,
                        class: item.class,
                        subclass: item.subclass,
                        image: item.image,
                        weight: item.weight,
                        equipped: item.equipped,
                    };

                    send_to_client(*player_id, info_item_packet, &clients);
                }
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn item_transfer_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    mut ids: ResMut<Ids>,
    mut items: ResMut<Items>,
    templates: Res<Templates>,
    query: Query<ItemTransferQuery>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::ItemTransfer {
                player_id,
                target_id,
                item_id,
            } => {
                events_to_remove.push(*event_id);

                if let Some(item) = Item::find_by_id(*item_id, &items) {
                    let Some(owner_entity) = ids.get_entity(item.owner) else {
                        error!("Cannot find owner entity from id: {:?}", item.owner);
                        continue;
                    };

                    let Some(target_entity) = ids.get_entity(*target_id) else {
                        error!("Cannot find target entity from id: {:?}", target_id);
                        continue;
                    };

                    let entities = [owner_entity, target_entity];

                    let Ok([mut owner, mut target]) = query.get_many(entities) else {
                        error!("Cannot find owner or target from entities {:?}", entities);
                        continue;
                    };

                    if !(owner.pos == target.pos || Map::is_adjacent(*owner.pos, *target.pos)) {
                        let packet = ResponsePacket::Error {
                            errmsg: "Item is not nearby.".to_string(),
                        };
                        send_to_client(*player_id, packet, &clients);
                        continue;
                    }

                    // Structure founded and under construction use case
                    if target.class.0 == "structure" && target.state.0 == FOUNDED {
                        let attrs = target.structure_attrs;

                        // Check if item is required for structure construction
                        if let Some(attrs) = attrs {
                            if !Item::is_req(item.clone(), attrs.req.clone()) {
                                info!("Item not required for construction: {:?}", item);
                                let packet = ResponsePacket::Error {
                                    errmsg: "Item not required for construction.".to_string(),
                                };
                                send_to_client(*player_id, packet, &clients);
                                break;
                            }
                        }

                        // Process item transfer and calculate the require item quantities
                        let req_items = process_item_transfer_structure(
                            item.clone(),
                            target, // target is the structure
                            &mut items,
                            &mut ids,
                            &templates,
                        );

                        let source_items = Item::get_by_owner_packet(item.owner, &items);
                        let target_items = Item::get_by_owner_packet(*target_id, &items);

                        let source_inventory = network::Inventory {
                            id: item.owner,
                            cap: 100,
                            tw: 5,
                            items: source_items.clone(),
                        };

                        let target_inventory = network::Inventory {
                            id: *target_id,
                            cap: 100,
                            tw: 5,
                            items: target_items.clone(),
                        };

                        let item_transfer_packet: ResponsePacket = ResponsePacket::ItemTransfer {
                            result: "success".to_string(),
                            sourceid: item.owner,
                            sourceitems: source_inventory,
                            targetid: *target_id,
                            targetitems: target_inventory,
                            reqitems: req_items,
                        };

                        send_to_client(*player_id, item_transfer_packet, &clients);
                    } else if owner.class.0 == "structure" && owner.state.0 == FOUNDED {
                        let attrs = target.structure_attrs;

                        // Check if item is required for structure construction
                        if let Some(attrs) = attrs {
                            if !Item::is_req(item.clone(), attrs.req.clone()) {
                                info!("Item not required for construction: {:?}", item);
                                let packet = ResponsePacket::Error {
                                    errmsg: "Item not required for construction.".to_string(),
                                };
                                send_to_client(*player_id, packet, &clients);
                                break;
                            }
                        }

                        if let Some(structure_attrs) = owner.structure_attrs {
                            Item::transfer(item.id, target.id.0, &mut items);

                            let structure_items = Item::get_by_owner(owner.id.0, &items);

                            let req_items = Structure::process_req_items(
                                structure_items,
                                structure_attrs.req.clone(),
                            );

                            let source_items = Item::get_by_owner_packet(item.owner, &items);
                            let target_items = Item::get_by_owner_packet(*target_id, &items);

                            let source_inventory = network::Inventory {
                                id: item.owner,
                                cap: 100,
                                tw: 5,
                                items: source_items.clone(),
                            };

                            let target_inventory = network::Inventory {
                                id: *target_id,
                                cap: 100,
                                tw: 5,
                                items: target_items.clone(),
                            };

                            let item_transfer_packet: ResponsePacket =
                                ResponsePacket::ItemTransfer {
                                    result: "success".to_string(),
                                    sourceid: item.owner,
                                    sourceitems: source_inventory,
                                    targetid: *target_id,
                                    targetitems: target_inventory,
                                    reqitems: req_items,
                                };

                            send_to_client(*player_id, item_transfer_packet, &clients);
                        } else {
                            error!("Obj is missing expected structure attributes");
                        }
                    } else {
                        Item::transfer(item.id, target.id.0, &mut items);

                        let source_items = Item::get_by_owner_packet(item.owner, &items);
                        let target_items = Item::get_by_owner_packet(*target_id, &items);

                        let source_inventory = network::Inventory {
                            id: item.owner,
                            cap: 100,
                            tw: 5,
                            items: source_items.clone(),
                        };

                        let target_inventory = network::Inventory {
                            id: *target_id,
                            cap: 100,
                            tw: 5,
                            items: target_items.clone(),
                        };

                        let item_transfer_packet: ResponsePacket = ResponsePacket::ItemTransfer {
                            result: "success".to_string(),
                            sourceid: item.owner,
                            sourceitems: source_inventory,
                            targetid: *target_id,
                            targetitems: target_inventory,
                            reqitems: Vec::new(),
                        };

                        send_to_client(*player_id, item_transfer_packet, &clients);
                    }
                } else {
                    error!("Failed to find item");
                }
            }
            PlayerEvent::InfoItemTransfer {
                player_id,
                source_id,
                target_id,
            } => {
                events_to_remove.push(*event_id);

                debug!(
                    "PlayerEvent::InfoItemTransfer sourceid: {:?} targetid: {:?}",
                    *source_id, *target_id
                );

                let Some(source_entity) = ids.get_entity(*source_id) else {
                    error!("Cannot find source entity from id: {:?}", source_id);
                    continue;
                };

                let Some(target_entity) = ids.get_entity(*target_id) else {
                    error!("Cannot find target entity from id: {:?}", target_id);
                    continue;
                };

                let entities = [source_entity, target_entity];

                let Ok([source, target]) = query.get_many(entities) else {
                    error!("Cannot find source or target from entities {:?}", entities);
                    continue;
                };

                if target.player_id.0 != *player_id && target.state.0 != DEAD.to_string() {
                    error!("Cannot transfer items from alive entity {:?}", target.id.0);
                    let packet = ResponsePacket::Error {
                        errmsg: "Cannot transfer items from alive entity".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                let source_items = Item::get_by_owner_packet(*source_id, &items);
                let target_items = Item::get_by_owner_packet(*target_id, &items);

                let source_inventory = network::Inventory {
                    id: *source_id,
                    cap: 100,
                    tw: 5,
                    items: source_items,
                };

                let target_inventory = network::Inventory {
                    id: *target_id,
                    cap: 100,
                    tw: 5,
                    items: target_items.clone(),
                };

                let req_items = get_current_req_quantities(target, &items);

                let info_item_transfer_packet: ResponsePacket = ResponsePacket::InfoItemTransfer {
                    sourceid: *source_id,
                    sourceitems: source_inventory,
                    targetid: *target_id,
                    targetitems: target_inventory,
                    reqitems: req_items,
                };

                send_to_client(*player_id, info_item_transfer_packet, &clients);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn item_split_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    mut ids: ResMut<Ids>,
    mut items: ResMut<Items>,
    templates: Res<Templates>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::ItemSplit {
                player_id,
                item_id,
                quantity,
            } => {
                events_to_remove.push(*event_id);

                if let Some(item) = Item::find_by_id(*item_id, &items) {
                    // TODO add checks if item_id is owned by player and if quantity is more than item quantity
                    Item::split(
                        *item_id,
                        *quantity,
                        ids.new_item_id(),
                        &mut items,
                        &templates.item_templates,
                    );

                    let item_split_packet: ResponsePacket = ResponsePacket::ItemSplit {
                        result: "success".to_string(),
                        owner: item.owner,
                    };

                    send_to_client(*player_id, item_split_packet, &clients);
                }
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn order_follow_system(
    mut events: ResMut<PlayerEvents>,
    ids: Res<Ids>,
    mut commands: Commands,
    query: Query<MapObjQuery>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::OrderFollow {
                player_id,
                source_id,
            } => {
                events_to_remove.push(*event_id);

                let Some(hero_id) = ids.get_hero(*player_id) else {
                    error!("Cannot find hero for player {:?}", *player_id);
                    break;
                };

                let Some(hero_entity) = ids.get_entity(hero_id) else {
                    error!("Cannot find hero entity for hero {:?}", hero_id);
                    break;
                };

                // Add OrderFollow component to source and set hero_entity as target
                for q in &query {
                    if q.id.0 == *source_id {
                        commands.entity(q.entity).insert(Order::Follow {
                            target: hero_entity,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn order_gather_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    ids: Res<Ids>,
    resources: Res<Resources>,
    mut commands: Commands,
    query: Query<CoreQuery, With<SubclassVillager>>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::OrderGather {
                player_id,
                source_id,
                res_type,
            } => {
                events_to_remove.push(*event_id);

                let Some(entity) = ids.get_entity(*source_id) else {
                    error!("Cannot find entity for {:?}", source_id);
                    break;
                };

                let Ok(villager) = query.get(entity) else {
                    error!("Cannot find villager for {:?}", entity);
                    break;
                };

                if villager.player_id.0 != *player_id {
                    error!("Villager not owned by player {:?}", *player_id);
                    let packet = ResponsePacket::Error {
                        errmsg: "Cannot order another player's villager".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }

                if Resource::is_valid_type(res_type.to_string(), *villager.pos, &resources) {
                    error!("Invalid resource type {:?}", res_type);
                    let packet = ResponsePacket::Error {
                        errmsg: "Invalid resource type {:?}".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }

                commands.entity(entity).insert(Order::Gather {
                    res_type: res_type.to_string(),
                });
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn structure_list_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    templates: Res<Templates>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::StructureList { player_id } => {
                events_to_remove.push(*event_id);
                let structure_list = Structure::available_to_build(&templates.obj_templates);

                let structure_list = StructureList {
                    result: structure_list,
                };

                let res_packet = ResponsePacket::StructureList(structure_list);

                send_to_client(*player_id, res_packet, &clients);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn create_foundation_system(
    mut events: ResMut<PlayerEvents>,
    mut commands: Commands,
    game_tick: ResMut<GameTick>,
    clients: Res<Clients>,
    mut ids: ResMut<Ids>,
    mut map_events: ResMut<MapEvents>,
    templates: Res<Templates>,
    hero_query: Query<CoreQuery, With<SubclassHero>>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::CreateFoundation {
                player_id,
                source_id,
                structure_name,
            } => {
                debug!("CreateFoundation");
                events_to_remove.push(*event_id);

                // Validation checks and get hero entity
                let Some(hero_entity) = ids.get_entity(*source_id) else {
                    error!("Cannot find hero entity for {:?}", source_id);
                    break;
                };

                let Ok(hero) = hero_query.get(hero_entity) else {
                    error!("Query failed to find entity {:?}", hero_entity);
                    break;
                };

                // Check if hero is owned by player
                if hero.player_id.0 != *player_id {
                    error!("Hero is not owned by player {:?}", *player_id);
                    break;
                }

                // Get structure template
                let Some(structure_template) = Structure::get(structure_name.clone(), &templates.obj_templates) else {
                    let packet = ResponsePacket::Error {
                        errmsg: "Invalid structure name".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                };

                let structure_id = ids.new_obj_id();

                let structure = Obj {
                    id: Id(structure_id),
                    player_id: PlayerId(*player_id),
                    position: Position {
                        x: hero.pos.x,
                        y: hero.pos.y,
                    },
                    name: Name(structure_name.clone()),
                    template: Template(structure_template.template.clone()),
                    class: Class(structure_template.class),
                    subclass: Subclass(structure_template.subclass),
                    state: State("founded".into()),
                    viewshed: Viewshed { range: 0 },
                    misc: Misc {
                        image: structure_template.template.to_string().to_lowercase(),
                        hsl: Vec::new().into(),
                        groups: Vec::new().into(),
                    },
                    stats: Stats {
                        hp: 1,
                        base_hp: structure_template.base_hp.unwrap(), // Convert option to non-option
                        base_def: 0,
                        base_damage: None,
                        damage_range: None,
                        base_speed: None,
                        base_vision: None,
                    },
                };

                let structure_attrs = StructureAttrs {
                    start_time: 0,
                    end_time: 0,
                    build_time: structure_template.build_time.unwrap(), // Structure must have build time
                    builder: *source_id,
                    progress: 0,
                    req: structure_template.req.unwrap(),
                };

                let structure_entity_id = commands
                    .spawn((structure, structure_attrs, ClassStructure))
                    .id();

                ids.new_entity_obj_mapping(structure_id, structure_entity_id);

                // Insert new obj event
                let new_obj_event = VisibleEvent::NewObjEvent { new_player: false };
                let map_event_id = ids.new_map_event_id();

                let map_state_event = MapEvent {
                    event_id: map_event_id,
                    entity_id: structure_entity_id,
                    obj_id: structure_id,
                    player_id: *player_id,
                    pos_x: hero.pos.x,
                    pos_y: hero.pos.y,
                    run_tick: game_tick.0 + 1, // Add one game tick
                    map_event_type: new_obj_event,
                };

                map_events.insert(map_event_id, map_state_event);

                let packet = ResponsePacket::CreateFoundation {
                    result: "success".to_string(),
                };

                send_to_client(*player_id, packet, &clients)
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn build_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    game_tick: ResMut<GameTick>,
    mut map_events: ResMut<MapEvents>,
    mut ids: ResMut<Ids>,
    mut items: ResMut<Items>,
    builder_query: Query<CoreQuery, Or<(With<SubclassHero>, With<SubclassVillager>)>>,
    mut structure_query: Query<StructureQuery, With<ClassStructure>>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::Build {
                player_id,
                source_id,
                structure_id,
            } => {
                debug!("Build");
                events_to_remove.push(*event_id);

                // Validation checks and get builder and structure entities
                let Some(builder_entity) = ids.get_entity(*source_id) else {
                    error!("Cannot find builder entity for {:?}", source_id);
                    break;
                };

                let Ok(builder) = builder_query.get(builder_entity) else {
                    error!("Query failed to find entity {:?}", builder_entity);
                    break;
                };

                let Some(structure_entity) = ids.get_entity(*structure_id) else {
                    error!("Cannot find structure entity for {:?}", structure_id);
                    break;
                };

                let Ok(mut structure) = structure_query.get_mut(structure_entity) else {
                    error!("Query failed to find entity {:?}", structure_entity);
                    break;
                };

                // Check if builder is owned by player
                if builder.player_id.0 != *player_id {
                    let packet = ResponsePacket::Error {
                        errmsg: "Builder not owned by player.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }

                // Check if structure is owned by player
                if structure.player_id.0 != *player_id {
                    let packet = ResponsePacket::Error {
                        errmsg: "Structure not owned by player.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }

                // Check if builder is on the same pos as structure
                if *builder.pos != *structure.pos {
                    let packet = ResponsePacket::Error {
                        errmsg: "Builder must be on the same position as structure.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }

                // Check if structure is missing required items
                if !Structure::has_req(structure.id.0, &mut structure.attrs.req, &mut items) {
                    let packet = ResponsePacket::Error {
                        errmsg: "Structure is missing required items.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }

                // Consume req items
                Structure::consume_reqs(structure.id.0, structure.attrs.req.clone(), &mut items);

                // Set structure building attributes
                structure.attrs.start_time = game_tick.0;
                structure.attrs.end_time = game_tick.0 + structure.attrs.build_time * 2;
                structure.attrs.builder = *source_id;

                // Builder State Change Event to Building
                let state_change_event = VisibleEvent::StateChangeEvent {
                    new_state: BUILDING.to_string(),
                };

                map_events.new(
                    ids.new_map_event_id(),
                    builder.entity,
                    &builder.id,
                    &builder.player_id,
                    &builder.pos,
                    game_tick.0 + 1, // in the future
                    state_change_event,
                );

                // Structure State Change Event to Progressing
                let structure_state_change = VisibleEvent::StateChangeEvent {
                    new_state: PROGRESSING.to_string(),
                };

                map_events.new(
                    ids.new_map_event_id(),
                    structure.entity,
                    &structure.id,
                    &structure.player_id,
                    &structure.pos,
                    game_tick.0 + 1, // in the future
                    structure_state_change,
                );

                // Structure State Change Event to None as it completed
                let structure_state_change = VisibleEvent::StateChangeEvent {
                    new_state: NONE.to_string(),
                };

                map_events.new(
                    ids.new_map_event_id(),
                    structure.entity,
                    &structure.id,
                    &structure.player_id,
                    &structure.pos,
                    structure.attrs.end_time, // in the future
                    structure_state_change,
                );

                let packet = ResponsePacket::Build {
                    build_time: structure.attrs.build_time / 5, // TODO: Build time in obj_template.yaml should be revisited.
                };

                send_to_client(*player_id, packet, &clients);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn explore_system(
    mut events: ResMut<PlayerEvents>,
    game_tick: Res<GameTick>,
    mut ids: ResMut<Ids>,
    mut map_events: ResMut<MapEvents>,
    hero_query: Query<
        (
            Entity,
            &Id,
            &Position,
            &PlayerId,
            &Name,
            &Template,
            &Class,
            &Subclass,
            &State,
        ),
        With<SubclassHero>,
    >,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::Explore { player_id } => {
                events_to_remove.push(*event_id);

                for (
                    entity_id,
                    obj_id,
                    pos,
                    obj_player_id,
                    _name,
                    _template,
                    _class,
                    _subclass,
                    _state,
                ) in hero_query.iter()
                {
                    if *player_id == obj_player_id.0 {
                        // Insert explore event
                        let explore_event = VisibleEvent::ExploreEvent;
                        let map_event_id = ids.new_map_event_id();

                        let map_state_event = MapEvent {
                            event_id: map_event_id,
                            entity_id: entity_id,
                            obj_id: obj_id.0,
                            player_id: *player_id,
                            pos_x: pos.x,
                            pos_y: pos.y,
                            run_tick: game_tick.0 + 1, // Add one game tick
                            map_event_type: explore_event,
                        };

                        map_events.insert(map_event_id, map_state_event);
                    }
                }
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn assign_list_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    mut ids: ResMut<Ids>,
    villager_query: Query<VillagerQuery, With<SubclassVillager>>,
    structure_query: Query<StructureQuery, With<ClassStructure>>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::AssignList { player_id } => {
                events_to_remove.push(*event_id);

                let mut assignments = Vec::new();

                for villager in villager_query.iter() {
                    if *player_id == villager.player_id.0 {
                        let mut structure_name = "None".to_string();

                        if villager.attrs.structure != -1 {
                            let Some(structure_entity) = ids.get_entity(villager.attrs.structure) else {
                                error!("Cannot find structure entity for {:?}", villager.attrs.structure);
                                continue;
                            };

                            let Ok(structure) = structure_query.get(structure_entity) else {
                                error!("Query failed to find entity {:?}", structure_entity);
                                continue;
                            };

                            structure_name = structure.name.0.to_string();
                        }

                        let assignment = network::Assignment {
                            id: villager.id.0,
                            name: villager.name.0.to_string(),
                            image: villager.misc.image.to_string(),
                            order: "none".to_string(), // Query order
                            structure: structure_name,
                        };

                        assignments.push(assignment);
                    }
                }

                let packet = ResponsePacket::AssignList {
                    result: assignments,
                };

                send_to_client(*player_id, packet, &clients);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn assign_system(
    mut commands: Commands,
    mut events: ResMut<PlayerEvents>,
    mut ids: ResMut<Ids>,
    clients: Res<Clients>,
    mut villager_query: Query<VillagerQuery, With<SubclassVillager>>,
    structure_query: Query<StructureQuery, With<ClassStructure>>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::Assign {
                player_id,
                source_id,
                target_id,
            } => {
                events_to_remove.push(*event_id);

                // Validation checks get source entity
                let Some(villager_entity) = ids.get_entity(*source_id) else {
                    error!("Cannot find villager entity for {:?}", source_id);
                    continue;
                };

                let Ok(mut villager) = villager_query.get_mut(villager_entity) else {
                    error!("Query failed to find entity {:?}", villager_entity);
                    continue;
                };

                let Some(structure_entity) = ids.get_entity(*target_id) else {
                    error!("Cannot find structure entity for {:?}", target_id);
                    continue;
                };

                let Ok(structure) = structure_query.get(structure_entity) else {
                    error!("Query failed to find entity {:?}", structure_entity);
                    continue;
                };

                // Check if builder is owned by player
                if villager.player_id.0 != *player_id {
                    let packet = ResponsePacket::Error {
                        errmsg: "Villager not owned by player.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                // Check if structure is owned by player
                if structure.player_id.0 != *player_id {
                    let packet = ResponsePacket::Error {
                        errmsg: "Structure not owned by player.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                debug!(
                    "Assign villager to structure class {:?} with id {:?}",
                    structure.subclass.0, structure.id.0
                );

                // Set villager structure
                villager.attrs.structure = structure.id.0;

                if structure.subclass.0 == structure::RESOURCE {
                    commands.entity(villager_entity).insert(Order::Operate);
                } else if structure.subclass.0 == structure::CRAFT {
                    commands.entity(villager_entity).insert(Order::Refine);
                } else {
                    error!("Can only assign to crafting or harvesting buildings");
                    continue;
                }

                let packet = ResponsePacket::Assign {
                    result: "success".to_string(),
                };

                send_to_client(*player_id, packet, &clients);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn equip_system(
    mut events: ResMut<PlayerEvents>,
    mut ids: ResMut<Ids>,
    clients: Res<Clients>,
    mut items: ResMut<Items>,
    query: Query<CoreQuery>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::Equip {
                player_id,
                item_id,
                status,
            } => {
                events_to_remove.push(*event_id);

                let Some(mut item) = Item::find_by_id(*item_id, &items) else {
                    debug!("Failed to find item: {:?}", item_id);
                    continue;
                };

                // Validation checks get source entity
                let Some(owner_entity) = ids.get_entity(item.owner) else {
                    error!("Cannot find villager entity for {:?}", item.owner);
                    continue;
                };

                let Ok(owner) = query.get(owner_entity) else {
                    error!("Query failed to find entity {:?}", owner_entity);
                    continue;
                };

                // Check if entity is owned by player
                if owner.player_id.0 != *player_id {
                    let packet = ResponsePacket::Error {
                        errmsg: "Item not owned by player.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                // Check if equipable
                if !Item::is_equipable(item.clone()) {
                    let packet = ResponsePacket::Error {
                        errmsg: "Item is not equipable.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                // Check if object is busy
                if !is_none_state(&owner.state.0) {
                    let packet = ResponsePacket::Error {
                        errmsg: "Item owner is busy".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }

                debug!("Equip packet: {:?}", status);
                // Equip if status is true
                if *status {
                    Item::equip(*item_id, *status, &mut items);
                } else {
                    Item::equip(*item_id, *status, &mut items);
                }

                let success_packet = ResponsePacket::Equip {
                    result: "success".to_string(),
                };

                send_to_client(*player_id, success_packet, &clients);

                let item_packet = Item::get_packet(item.id, &items).unwrap();

                let item_update_packet: ResponsePacket = ResponsePacket::InfoItemUpdate {
                    id: item.owner,
                    item: item_packet,
                    merged: true,
                };

                send_to_client(*player_id, item_update_packet, &clients);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn recipe_list_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    mut ids: ResMut<Ids>,
    recipes: Res<Recipes>,
    structure_query: Query<StructureQuery, With<ClassStructure>>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::RecipeList {
                player_id,
                structure_id,
            } => {
                events_to_remove.push(*event_id);

                let Some(structure_entity) = ids.get_entity(*structure_id) else {
                    error!("Cannot find structure entity for {:?}", structure_id);
                    continue;
                };

                let Ok(structure) = structure_query.get(structure_entity) else {
                    error!("Query failed to find entity {:?}", structure_entity);
                    continue;
                };

                let structure_recipes = Recipe::get_by_structure_packet(
                    *player_id,
                    structure.template.0.clone(),
                    &recipes,
                );

                let packet = ResponsePacket::RecipeList {
                    result: structure_recipes,
                };

                send_to_client(*player_id, packet, &clients);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn order_refine_system(
    mut commands: Commands,
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    villager_query: Query<VillagerQuery, With<SubclassVillager>>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::OrderRefine {
                player_id,
                structure_id,
            } => {
                events_to_remove.push(*event_id);

                let mut villager = None;

                //Find villager assigned to structure
                for villager_item in villager_query.iter() {
                    if villager_item.attrs.structure == *structure_id
                        && villager_item.player_id.0 == *player_id
                    {
                        villager = Some(villager_item);
                    }
                }

                if villager.is_none() {
                    error!(
                        "Cannot find a villager assigned to structure {:?}",
                        *structure_id
                    );
                    let packet = ResponsePacket::Error {
                        errmsg: "No villager assigned to structure to refine.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }

                if let Some(villager) = villager {
                    info!("Adding Order Refine to {:?}", villager.id);
                    commands.entity(villager.entity).insert(Order::Refine);
                }
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn order_craft_system(
    mut commands: Commands,
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    mut items: ResMut<Items>,
    recipes: Res<Recipes>,
    villager_query: Query<VillagerQuery, With<SubclassVillager>>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::OrderCraft {
                player_id,
                structure_id,
                recipe_name,
            } => {
                events_to_remove.push(*event_id);

                let mut villager = None;

                //Find villager assigned to structure
                for villager_item in villager_query.iter() {
                    if villager_item.attrs.structure == *structure_id
                        && villager_item.player_id.0 == *player_id
                    {
                        villager = Some(villager_item);
                    }
                }

                if villager.is_none() {
                    error!(
                        "Cannot find a villager assigned to structure {:?}",
                        *structure_id
                    );
                    let packet = ResponsePacket::Error {
                        errmsg: "No villager assigned to structure to refine.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }

                let recipe = Recipe::get_by_name(recipe_name.clone(), &recipes);

                if recipe.is_none() {
                    error!("Invalid recipe name {:?}", *recipe_name);
                    let packet = ResponsePacket::Error {
                        errmsg: "Invalid recipe".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }

                if let Some(mut recipe) = recipe {
                    //TODO consider if checking reqs is required here
                    if Structure::has_req(*structure_id, &mut recipe.req, &mut items) {
                        if let Some(villager) = villager {
                            info!("Adding Order Craft to {:?}", villager.id);
                            commands.entity(villager.entity).insert(Order::Craft {
                                recipe_name: recipe_name.clone(),
                            });
                        }
                    } else {
                        error!("Insufficient resources to craft {:?}", *recipe_name);
                        let packet = ResponsePacket::Error {
                            errmsg: "Insufficient resources to craft".to_string(),
                        };
                        send_to_client(*player_id, packet, &clients);
                        break;
                    }
                } else {
                    error!("Cannot find recipe: {:?}", *recipe_name);
                    let packet = ResponsePacket::Error {
                        errmsg: "Cannot find recipe".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn order_explore_system(
    mut events: ResMut<PlayerEvents>,
    ids: Res<Ids>,
    mut commands: Commands,
    clients: Res<Clients>,
    query: Query<MapObjQuery>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::OrderExplore {
                player_id,
                villager_id,
            } => {
                events_to_remove.push(*event_id);

                let Some(entity) = ids.get_entity(*villager_id) else {
                    error!("Cannot find entity for {:?}", villager_id);
                    break;
                };

                let Ok(villager) = query.get(entity) else {
                    error!("Cannot find villager for {:?}", entity);
                    break;
                };

                if villager.player_id.0 != *player_id {
                    error!("Villager not owned by player {:?}", *player_id);
                    let packet = ResponsePacket::Error {
                        errmsg: "Cannot order another player's villager".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    break;
                }

                // Add OrderFollow component to source and set hero_entity as target
                for q in &query {
                    if q.id.0 == *villager_id {
                        commands.entity(q.entity).insert(Order::Explore);
                    }
                }
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn use_item_system(
    mut events: ResMut<PlayerEvents>,
    game_tick: Res<GameTick>,
    mut ids: ResMut<Ids>,
    mut commands: Commands,
    clients: Res<Clients>,
    mut items: ResMut<Items>,
    mut map_events: ResMut<MapEvents>,
    query: Query<MapObjQuery>,
) {
    let mut events_to_remove: Vec<i32> = Vec::new();

    for (event_id, event) in events.iter() {
        match event {
            PlayerEvent::Use { player_id, item_id } => {
                events_to_remove.push(*event_id);

                let Some(mut item) = Item::find_by_id(*item_id, &items) else {
                    debug!("Failed to find item: {:?}", item_id);
                    continue;
                };

                // Validation checks get source entity
                let Some(owner_entity) = ids.get_entity(item.owner) else {
                    error!("Cannot find villager entity for {:?}", item.owner);
                    continue;
                };

                let Ok(owner) = query.get(owner_entity) else {
                    error!("Query failed to find entity {:?}", owner_entity);
                    continue;
                };

                // Check if entity is owned by player
                if owner.player_id.0 != *player_id {
                    let packet = ResponsePacket::Error {
                        errmsg: "Item not owned by player.".to_string(),
                    };
                    send_to_client(*player_id, packet, &clients);
                    continue;
                }
                // Insert explore event
                let use_item_event = VisibleEvent::UseItemEvent {
                    item_id: *item_id,
                    item_owner_id: owner.id.0,
                };

                let map_event_id = ids.new_map_event_id();

                let map_state_event = MapEvent {
                    event_id: map_event_id,
                    entity_id: owner_entity,
                    obj_id: owner.id.0,
                    player_id: *player_id,
                    pos_x: owner.pos.x,
                    pos_y: owner.pos.y,
                    run_tick: game_tick.0 + 1, // Add one game tick
                    map_event_type: use_item_event,
                };

                map_events.insert(map_event_id, map_state_event);
            }
            _ => {}
        }
    }

    for event_id in events_to_remove.iter() {
        events.remove(event_id);
    }
}

fn new_player(
    player_id: i32,
    mut commands: &mut Commands,
    mut ids: &mut ResMut<Ids>,
    mut map_events: &mut ResMut<MapEvents>,
    mut items: &mut ResMut<Items>,
    mut skills: &mut ResMut<Skills>,
    mut recipes: &mut ResMut<Recipes>,
    mut deeds: &mut ResMut<Deeds>,
    templates: &Res<Templates>,
    game_tick: &ResMut<GameTick>,
) {
    let start_x = 16;
    let start_y = 36;
    let range = 4;

    let hero_template_name = "Novice Warrior".to_string();
    let hero_template = ObjTemplate::get_template(hero_template_name.clone(), templates);

    // Create Hero Obj
    let hero_id = ids.new_obj_id();

    let hero = Obj {
        id: Id(hero_id),
        player_id: PlayerId(player_id),
        position: Position {
            x: start_x,
            y: start_y,
        },
        name: Name("Peter".into()),
        template: Template(hero_template_name),
        class: Class("unit".into()),
        subclass: Subclass("hero".into()),
        state: State("none".into()),
        viewshed: Viewshed { range: range },
        misc: Misc {
            image: "novicewarrior".into(),
            hsl: Vec::new().into(),
            groups: Vec::new().into(),
        },
        stats: Stats {
            hp: hero_template.base_hp.unwrap(),
            base_hp: hero_template.base_hp.unwrap(),
            base_def: hero_template.base_def.unwrap(),
            base_damage: hero_template.base_dmg,
            damage_range: hero_template.dmg_range,
            base_speed: hero_template.base_speed,
            base_vision: hero_template.base_vision,
        },
    };

    // Create hero items
    let berries = Item::new(
        ids.new_item_id(),
        hero_id,
        "Honeybell Berries".to_string(),
        25,
        &templates.item_templates,
    );
    let water = Item::new(
        ids.new_item_id(),
        hero_id,
        "Spring Water".to_string(),
        25,
        &templates.item_templates,
    );
    let wood1 = Item::new(
        ids.new_item_id(),
        hero_id,
        "Cragroot Maple Wood".to_string(),
        3,
        &templates.item_templates,
    );
    let wood2 = Item::new(
        ids.new_item_id(),
        hero_id,
        "Cragroot Maple Wood".to_string(),
        2,
        &templates.item_templates,
    );
    let wood3 = Item::new(
        ids.new_item_id(),
        hero_id,
        "Cragroot Maple Wood".to_string(),
        5,
        &templates.item_templates,
    );
    let hide = Item::new(
        ids.new_item_id(),
        hero_id,
        "Windstride Raw Hide".to_string(),
        5,
        &templates.item_templates,
    );
    let ingot = Item::new(
        ids.new_item_id(),
        hero_id,
        "Valleyrun Copper Ingot".to_string(),
        25,
        &templates.item_templates,
    );

    let timber = Item::new(
        ids.new_item_id(),
        hero_id,
        "Cragroot Maple Timber".to_string(),
        50,
        &templates.item_templates,
    );

    items.push(berries);
    items.push(water);
    items.push(wood1);
    items.push(wood2);
    items.push(wood3);
    items.push(hide);
    items.push(ingot);
    items.push(timber);

    let mut item_attrs = HashMap::new();
    item_attrs.insert(item::DAMAGE, 11.0);

    Item::new_with_attrs(
        ids.new_item_id(),
        hero_id,
        "Copper Training Axe".to_string(),
        1,
        item_attrs,
        &templates.item_templates,
        items,
    );

    let mut item_attrs2 = HashMap::new();
    item_attrs2.insert(item::HEALING, 10.0);

    Item::new_with_attrs(
        ids.new_item_id(),
        hero_id,
        "Health Potion".to_string(),
        1,
        item_attrs2,
        &templates.item_templates,
        items,
    );

    // Spawn hero
    let hero_entity_id = commands
        .spawn((
            hero,
            SubclassHero, // Hero component tag
        ))
        .id();

    ids.new_player_hero_mapping(player_id, hero_id);
    ids.new_entity_obj_mapping(hero_id, hero_entity_id);

    // Insert new obj event
    let new_obj_event = VisibleEvent::NewObjEvent { new_player: true };
    let map_event_id = ids.new_map_event_id();

    let map_state_event = MapEvent {
        event_id: map_event_id,
        entity_id: hero_entity_id,
        obj_id: hero_id,
        player_id: player_id,
        pos_x: start_x,
        pos_y: start_y,
        run_tick: game_tick.0 + 1, // Add one game tick
        map_event_type: new_obj_event,
    };

    map_events.insert(map_event_id, map_state_event);

    // Villager obj
    let villager_id = ids.new_obj_id();

    let villager_template_name = "Human Villager".to_string();
    let villager_template = ObjTemplate::get_template(villager_template_name.clone(), templates);

    let villager = Obj {
        id: Id(villager_id),
        player_id: PlayerId(player_id),
        position: Position { x: 16, y: 35 },
        name: Name("Villager 1".into()),
        template: Template("Human Villager".into()),
        class: Class("unit".into()),
        subclass: Subclass("villager".into()),
        state: State("none".into()),
        viewshed: Viewshed { range: 2 },
        misc: Misc {
            image: "humanvillager1".into(),
            hsl: Vec::new().into(),
            groups: Vec::new().into(),
        },
        stats: Stats {
            hp: villager_template.base_hp.unwrap(),
            base_hp: villager_template.base_hp.unwrap(),
            base_def: villager_template.base_def.unwrap(),
            base_damage: villager_template.base_dmg,
            damage_range: villager_template.dmg_range,
            base_speed: villager_template.base_speed,
            base_vision: villager_template.base_vision,
        },
    };

    // Villager generate skills
    Villager::generate_skills(villager_id, skills, &templates.skill_templates);

    // Villager create attributes components ```
    let base_attrs = Villager::generate_attributes(1);

    let villager_attrs = VillagerAttrs {
        shelter: "None".to_string(),
        structure: -1,
    };

    let water_villager = Item::new(
        ids.new_item_id(),
        villager_id,
        "Spring Water".to_string(),
        50,
        &templates.item_templates,
    );

    items.push(water_villager);

    let villager_entity_id = commands
        .spawn((
            villager,
            SubclassVillager,
            base_attrs,
            villager_attrs,
            Morale::new(100.0, 1.0),
            Thirst::new(0.0, 0.1),
            Thinker::build()
                .label("My Thinker")
                .picker(FirstToScore { threshold: 0.8 })
                .when(
                    Thirsty,
                    Drink {
                        until: 70.0,
                        per_tick: 10.0,
                    },
                )
                .when(HighMorale, ProcessOrder),
        ))
        .id();

    ids.new_entity_obj_mapping(villager_id, villager_entity_id);

    // Insert state change event
    let new_obj_event = VisibleEvent::NewObjEvent { new_player: false };
    let map_event_id = ids.new_map_event_id();

    let map_state_event = MapEvent {
        event_id: map_event_id,
        entity_id: villager_entity_id,
        obj_id: villager_id,
        player_id: player_id,
        pos_x: 16,
        pos_y: 35,
        run_tick: game_tick.0 + 1, // Add one game tick
        map_event_type: new_obj_event,
    };

    map_events.insert(map_event_id, map_state_event);

    // Starting recipes
    Recipe::create(
        player_id,
        "Training Pick Axe".to_string(),
        &templates.recipe_templates,
        recipes,
    );

    //Starting deeds 

    

}

fn get_current_req_quantities(target: ItemTransferQueryItem, items: &ResMut<Items>) -> Vec<ResReq> {
    if target.class.0 == "structure" && target.state.0 == FOUNDED {
        if let Some(attrs) = target.structure_attrs {
            let target_items = Item::get_by_owner(target.id.0, &items);
            let mut req_items = attrs.req.clone();

            // Check current required quantity from structure items
            for req_item in req_items.iter_mut() {
                let mut req_quantity = req_item.quantity;

                for target_item in target_items.iter() {
                    if req_item.req_type == target_item.name
                        || req_item.req_type == target_item.class
                        || req_item.req_type == target_item.subclass
                    {
                        if req_quantity - target_item.quantity > 0 {
                            req_quantity -= target_item.quantity;
                        } else {
                            req_quantity = 0;
                        }
                    }
                }

                req_item.cquantity = Some(req_quantity);
            }

            return req_items;
        }
    }

    // Return empty vector
    return Vec::new();
}

fn process_item_transfer_structure(
    item: Item,
    mut structure: ItemTransferQueryItem,
    mut items: &mut ResMut<Items>,
    mut ids: &mut ResMut<Ids>,
    templates: &Res<Templates>,
) -> Vec<ResReq> {
    if let Some(attrs) = structure.structure_attrs {
        let structure_items = Item::get_by_owner(structure.id.0, &items);
        let mut req_items = Structure::process_req_items(structure_items, attrs.req.clone());

        // Find first matching req item
        let matching_req_item = req_items.iter_mut().find(|r| {
            r.req_type == item.name || r.req_type == item.class || r.req_type == item.subclass
        });

        if let Some(matching_req_item) = matching_req_item {
            if let Some(match_req_item_cquantity) = &mut matching_req_item.cquantity {
                if *match_req_item_cquantity > 0 {
                    if *match_req_item_cquantity == item.quantity {
                        // Transfer entire item
                        Item::transfer(item.id, structure.id.0, items);

                        // Set current quantity to 0
                        *match_req_item_cquantity = 0;
                    } else if *match_req_item_cquantity > item.quantity {
                        // Transfer entire item
                        Item::transfer(item.id, structure.id.0, &mut items);

                        // Subtract current quantity
                        *match_req_item_cquantity -= item.quantity;
                    } else if *match_req_item_cquantity < item.quantity {
                        let new_item_id = ids.new_item_id();

                        // Split to create new item. Required here as item quantity is greater than req quantity
                        Item::split(
                            item.id,
                            *match_req_item_cquantity,
                            new_item_id,
                            &mut items,
                            &templates.item_templates,
                        );

                        // Transfer the new item
                        Item::transfer(new_item_id, structure.id.0, &mut items);

                        // Set current quantity to 0
                        *match_req_item_cquantity = 0;
                    }
                }

                // Return required items list
                return req_items;
            } else {
                error!("Matching current quantity is unexpected None.")
            }
        } else {
            error!("Item transfer is invalid due to lack of matching req item")
        }
    } else {
        error!(
            "Missing structure attributes on structure {:?}",
            structure.id
        );
    }

    return Vec::new();
}