use bevy::prelude::*;
use big_brain::prelude::*;

use std::collections::{HashMap, HashSet};

use crate::ai::{Drink, HighMorale, Morale, ProcessOrder, Thirst, Thirsty};
use crate::game::{
    is_pos_empty, Class, ClassStructure, Clients, GameTick, Hp, Id, Ids, MapEvent, MapEvents,
    MapObjQuery, Misc, Name, Obj, OrderFollow, PlayerId, Position, State, StructureAttrs, Subclass,
    SubclassHero, SubclassVillager, Template, Viewshed, VisibleEvent, VisibleEvents, BUILDING,
    DEAD, MOVING, NONE, PROGRESSING, NetworkReceiver,
};
use crate::item::{Item, Items};
use crate::map::Map;
use crate::network::{self, send_to_client, ResponsePacket};
use crate::resource::{Resource, Resources};
use crate::skill::Skills;
use crate::structure::Structure;
use crate::templates::{SkillTemplate, SkillTemplates, Templates};

#[derive(Resource, Deref, DerefMut)]
pub struct PlayerEvents(pub Vec<PlayerEvent>);

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
}

// Used as temporary obj storage for system
pub struct CoreObjComp {
    pub entity: Entity,
    pub obj_id: Id,
    pub player_id: PlayerId,
    pub pos: Position,
    pub state: State,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        // Initialize events
        let player_events: PlayerEvents = PlayerEvents(Vec::new());

        app.
            add_system(message_broker_system)
          .add_system(new_player_system)
            .add_system(move_system)
            .add_system(attack_system)
            .add_system(gather_system)
            .add_system(info_obj_system)
            .add_system(info_tile_system)
            .add_system(info_item_system)
            .add_system(item_split_system)
            .add_system(order_follow_system)
            .add_system(structure_list_system)
            .add_system(create_foundation_system)
            .add_system(build_system)
            .add_system(explore_system)
            .insert_resource(player_events);
    }
}

fn message_broker_system(
  client_to_game_receiver: Res<NetworkReceiver>,
  mut player_events: ResMut<PlayerEvents>,
) {
  if let Ok(evt) = client_to_game_receiver.try_recv() {
      println!("{:?}", evt);

      player_events.push(evt.clone());
  }
}

fn new_player_system(
    mut events: ResMut<PlayerEvents>,
    mut commands: Commands,
    game_tick: ResMut<GameTick>,
    mut ids: ResMut<Ids>,
    mut map_events: ResMut<MapEvents>,
    mut items: ResMut<Items>,
    templates: Res<Templates>,
) {
    let mut events_to_remove: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match event {
            PlayerEvent::NewPlayer { player_id } => {
                new_player(
                    *player_id,
                    &mut commands,
                    &mut ids,
                    &mut map_events,
                    &mut items,
                    &templates,
                    &game_tick,
                );

                events_to_remove.push(index);
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(*index);
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
    let mut events_to_remove: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match event {
            PlayerEvent::Move { player_id, x, y } => {
                debug!("Move Event: {:?}", event);
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
                        return;
                    };

                    if !is_pos_empty(*player_id, *x, *y, &query) {
                        println!("Position is not empty");
                        let error = ResponsePacket::Error {
                            errmsg: "Tile is occupied.".to_owned(),
                        };
                        send_to_client(*player_id, error, &clients);
                        return;
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
                        game_tick.0 + 4, // in the future
                        move_event,
                    );
                }

                events_to_remove.push(index);
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(*index);
    }
}

fn attack_system(
    mut events: ResMut<PlayerEvents>,
    game_tick: ResMut<GameTick>,
    mut ids: ResMut<Ids>,
    clients: Res<Clients>,
    mut map_events: ResMut<MapEvents>,
    mut visible_events: ResMut<VisibleEvents>,
    map: Res<Map>,
    mut query: Query<(Entity, &Id, &Position, &PlayerId, &State, &mut Hp)>,
) {
    let mut events_to_remove: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match event {
            PlayerEvent::Attack {
                player_id,
                attack_type,
                source_id,
                target_id,
            } => {
                events_to_remove.push(index);

                let mut attacker: Option<CoreObjComp> = None;
                let mut target: Option<CoreObjComp> = None;

                // Get attacker
                for (entity, id, pos, player_id, state, hitpoints) in query.iter() {
                    if id.0 == *source_id {
                        attacker = Some(CoreObjComp {
                            entity: entity,
                            obj_id: id.clone(),
                            player_id: player_id.clone(),
                            pos: pos.clone(),
                            state: state.clone(),
                        });
                    }
                }

                // Get target
                for (entity, id, pos, player_id, state, hitpoints) in query.iter() {
                    if id.0 == *target_id {
                        target = Some(CoreObjComp {
                            entity: entity,
                            obj_id: id.clone(),
                            player_id: player_id.clone(),
                            pos: pos.clone(),
                            state: state.clone(),
                        });
                    }
                }

                if let (Some(attacker), Some(target)) = (attacker, target) {
                    // Check if attacker is owned by player
                    if attacker.player_id.0 != *player_id {
                        let packet = ResponsePacket::Error {
                            errmsg: "Attacker not owned by player.".to_string(),
                        };
                        send_to_client(*player_id, packet, &clients);
                        break;
                    }

                    // Is target adjacent
                    if Map::dist(attacker.pos, target.pos) > 1 {
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

                    let packet = ResponsePacket::Attack {
                        sourceid: *source_id,
                        attacktype: attack_type.clone(),
                        cooldown: 20,
                        stamina_cost: 5,
                    };

                    send_to_client(*player_id, packet, &clients);

                    let dmg = 10;

                    if let Ok((entity, id, pos, player_id, state, mut hitpoints)) =
                        query.get_mut(target.entity)
                    {
                        hitpoints.0 -= dmg;

                        debug!("hitpoints: {:?}", hitpoints);

                        let damage_event = VisibleEvent::DamageEvent {
                            target_id: id.0,
                            target_pos: pos.clone(),
                            attack_type: attack_type.clone(),
                            damage: dmg,
                            state: state.0.clone(),
                        };

                        let map_event = MapEvent {
                            event_id: ids.new_map_event_id(),
                            entity_id: attacker.entity,
                            obj_id: attacker.obj_id.0,
                            player_id: attacker.player_id.0,
                            pos_x: attacker.pos.x,
                            pos_y: attacker.pos.y,
                            run_tick: game_tick.0,
                            map_event_type: damage_event,
                        };

                        visible_events.push(map_event);
                    }
                }
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(*index);
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
    let mut events_to_remove: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match event {
            PlayerEvent::Gather {
                player_id,
                source_id,
                res_type,
            } => {
                debug!("PlayerEvent::Gather");

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

                events_to_remove.push(index);
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(*index);
    }
}

fn info_obj_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    query: Query<MapObjQuery>,
) {
    let mut events_to_remove: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match event {
            PlayerEvent::InfoObj { player_id, id } => {
                for q in &query {
                    if q.id.0 == *id {
                        let info_obj_packet: ResponsePacket = ResponsePacket::InfoObj {
                            id: q.id.0,
                            name: q.name.0.to_owned(),
                            template: q.template.0.to_owned(),
                            class: q.class.0.to_owned(),
                            subclass: q.subclass.0.to_owned(),
                            state: q.state.0.to_owned(),
                        };

                        send_to_client(*player_id, info_obj_packet, &clients);
                    }
                }

                events_to_remove.push(index);
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(*index);
    }
}

fn info_tile_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    map: Res<Map>,
    resources: Res<Resources>,
) {
    let mut events_to_remove: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match event {
            PlayerEvent::InfoTile { player_id, x, y } => {
                debug!("PlayerEvent::InfoTile x: {:?} y: {:?}", *x, *y);

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

                events_to_remove.push(index);
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(*index);
    }
}

fn info_item_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    mut items: ResMut<Items>,
) {
    let mut events_to_remove: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match event {
            PlayerEvent::InfoInventory { player_id, id } => {
                debug!("PlayerEvent::InfoInventory id: {:?}", id);

                let inventory_items = Item::get_by_owner_packet(*id, &items);

                let info_inventory_packet: ResponsePacket = ResponsePacket::InfoInventory {
                    id: *id,
                    cap: 100,
                    tw: 100,
                    items: inventory_items,
                };

                send_to_client(*player_id, info_inventory_packet, &clients);

                events_to_remove.push(index);
            }
            PlayerEvent::InfoItem { player_id, id } => {
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

                events_to_remove.push(index);
            }
            PlayerEvent::InfoItemByName { player_id, name } => {
                debug!("PlayerEvent::InfoItemByName name: {:?}", name.clone());

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

                events_to_remove.push(index);
            }
            PlayerEvent::InfoItemTransfer {
                player_id,
                source_id,
                target_id,
            } => {
                debug!(
                    "PlayerEvent::InfoItemTransfer sourceid: {:?} targetid: {:?}",
                    *source_id, *target_id
                );

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
                    items: target_items,
                };

                let info_item_transfer_packet: ResponsePacket = ResponsePacket::InfoItemTransfer {
                    sourceid: *source_id,
                    sourceitems: source_inventory,
                    targetid: *target_id,
                    targetitems: target_inventory,
                    reqitems: Vec::new(),
                };

                send_to_client(*player_id, info_item_transfer_packet, &clients);

                events_to_remove.push(index);
            }
            PlayerEvent::ItemTransfer {
                player_id,
                target_id,
                item_id,
            } => {
                if let Some(item) = Item::find_by_id(*item_id, &items) {
                    Item::transfer(*item_id, *target_id, &mut items);

                    let source_items = Item::get_by_owner_packet(item.owner, &items);
                    let target_items = Item::get_by_owner_packet(*target_id, &items);

                    let source_inventory = network::Inventory {
                        id: item.owner,
                        cap: 100,
                        tw: 5,
                        items: source_items,
                    };

                    let target_inventory = network::Inventory {
                        id: *target_id,
                        cap: 100,
                        tw: 5,
                        items: target_items,
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

                events_to_remove.push(index);
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(*index);
    }
}

fn item_split_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    mut ids: ResMut<Ids>,
    mut items: ResMut<Items>,
    templates: Res<Templates>,
) {
    let mut events_to_remove: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match event {
            PlayerEvent::ItemSplit {
                player_id,
                item_id,
                quantity,
            } => {
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

                events_to_remove.push(index);
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(*index);
    }
}

fn order_follow_system(
    mut events: ResMut<PlayerEvents>,
    mut commands: Commands,
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
    let mut events_to_remove: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match event {
            PlayerEvent::OrderFollow {
                player_id,
                source_id,
            } => {
                for (
                    entity,
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

                    for q in &query {
                        if q.id.0 == *source_id {
                            commands
                                .entity(q.entity)
                                .insert(OrderFollow { target: entity });
                        }
                    }
                }

                events_to_remove.push(index);
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(*index);
    }
}

fn structure_list_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    templates: Res<Templates>,
) {
    let mut events_to_remove: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match event {
            PlayerEvent::StructureList { player_id } => {
                events_to_remove.push(index);
                let structure_list = Structure::available_to_build(&templates.obj_templates);

                let structure_list = ResponsePacket::StructureList {
                    result: structure_list,
                };

                send_to_client(*player_id, structure_list, &clients);
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(*index);
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
    let mut events_to_remove: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match event {
            PlayerEvent::CreateFoundation {
                player_id,
                source_id,
                structure_name,
            } => {
                debug!("CreateFoundation");
                events_to_remove.push(index);

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
                    // Check if player matches
                    if *player_id != obj_player_id.0 {
                        continue;
                    }

                    let structure_id = ids.new_obj_id();

                    if let Some(structure_template) =
                        Structure::get(structure_name.clone(), &templates.obj_templates)
                    {
                        let structure = Obj {
                            id: Id(structure_id),
                            player_id: PlayerId(*player_id),
                            position: Position { x: pos.x, y: pos.y },
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
                            hp: Hp(1), // Starts with 1 HP
                        };

                        let structure_attrs = StructureAttrs {
                            start_time: 0,
                            end_time: 0,
                            build_time: structure_template.build_time.unwrap(), // Structure must have build time
                            builder: *source_id,
                            progress: 0,
                        };

                        let structure_entity_id = commands
                            .spawn((structure, structure_attrs, ClassStructure))
                            .id();

                        // Insert new obj event
                        let new_obj_event = VisibleEvent::NewObjEvent { new_player: false };
                        let map_event_id = ids.new_map_event_id();

                        let map_state_event = MapEvent {
                            event_id: map_event_id,
                            entity_id: structure_entity_id,
                            obj_id: structure_id,
                            player_id: *player_id,
                            pos_x: pos.x,
                            pos_y: pos.y,
                            run_tick: game_tick.0 + 1, // Add one game tick
                            map_event_type: new_obj_event,
                        };

                        map_events.insert(map_event_id, map_state_event);

                        let packet = ResponsePacket::CreateFoundation {
                            result: "success".to_string(),
                        };

                        send_to_client(*player_id, packet, &clients)
                    }
                }
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(*index);
    }
}

fn build_system(
    mut events: ResMut<PlayerEvents>,
    clients: Res<Clients>,
    game_tick: ResMut<GameTick>,
    mut map_events: ResMut<MapEvents>,
    mut ids: ResMut<Ids>,
    builder_query: Query<
        (Entity, &Id, &PlayerId, &Position, &State),
        Or<(With<SubclassHero>, With<SubclassVillager>)>,
    >,
    mut structure_query: Query<
        (
            Entity,
            &Id,
            &PlayerId,
            &Position,
            &State,
            &mut StructureAttrs,
        ),
        With<ClassStructure>,
    >,
) {
    let mut events_to_remove: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match event {
            PlayerEvent::Build {
                player_id,
                source_id,
                structure_id,
            } => {
                debug!("Build");
                events_to_remove.push(index);

                let mut builder: Option<CoreObjComp> = None;
                let mut structure: Option<CoreObjComp> = None;

                // Get builder
                for (entity, obj_id, player, pos, state) in builder_query.iter() {
                    if obj_id.0 == *source_id {
                        // Get builder
                        builder = Some(CoreObjComp {
                            entity: entity,
                            obj_id: obj_id.clone(),
                            player_id: player.clone(),
                            pos: pos.clone(),
                            state: state.clone(),
                        });
                    }
                }

                // Get structure
                for (entity, obj_id, player_id, pos, state, _structure_attrs) in
                    structure_query.iter()
                {
                    if obj_id.0 == *structure_id {
                        structure = Some(CoreObjComp {
                            entity: entity,
                            obj_id: obj_id.clone(),
                            player_id: player_id.clone(),
                            pos: pos.clone(),
                            state: state.clone(),
                        });
                    }
                }

                if let (Some(builder), Some(structure)) = (builder, structure) {
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
                    if builder.pos != structure.pos {
                        let packet = ResponsePacket::Error {
                            errmsg: "Builder must be on the same position as structure."
                                .to_string(),
                        };
                        send_to_client(*player_id, packet, &clients);
                        break;
                    }

                    if let Ok((entity, obj_id, player_id, pos, state, mut structure_attrs)) =
                        structure_query.get_mut(structure.entity)
                    {
                        // Builder State Change Event to Building
                        let state_change_event = VisibleEvent::StateChangeEvent {
                            new_state: BUILDING.to_string(),
                        };

                        map_events.new(
                            ids.new_map_event_id(),
                            builder.entity,
                            &builder.obj_id,
                            &builder.player_id,
                            &builder.pos,
                            game_tick.0 + 1, // in the future
                            state_change_event,
                        );

                        structure_attrs.start_time = game_tick.0;
                        structure_attrs.end_time = game_tick.0 + structure_attrs.build_time * 2;
                        structure_attrs.builder = *source_id;

                        // Structure State Change Event to Progressing
                        let structure_state_change = VisibleEvent::StateChangeEvent {
                            new_state: PROGRESSING.to_string(),
                        };

                        map_events.new(
                            ids.new_map_event_id(),
                            entity,
                            obj_id,
                            player_id,
                            pos,
                            game_tick.0 + 1, // in the future
                            structure_state_change,
                        );

                        // Structure State Change Event to None as it completed
                        let structure_state_change = VisibleEvent::StateChangeEvent {
                            new_state: NONE.to_string(),
                        };

                        map_events.new(
                            ids.new_map_event_id(),
                            entity,
                            obj_id,
                            player_id,
                            pos,
                            structure_attrs.end_time, // in the future
                            structure_state_change,
                        );

                        let packet = ResponsePacket::Build {
                            build_time: structure_attrs.build_time / 5, // TODO: Build time in obj_template.yaml should be revisited.
                        };

                        send_to_client(player_id.0, packet, &clients);
                    }
                }
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(*index);
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
    let mut events_to_remove: Vec<usize> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match event {
            PlayerEvent::Explore { player_id } => {
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

                events_to_remove.push(index);
            }
            _ => {}
        }
    }

    for index in events_to_remove.iter() {
        events.remove(*index);
    }
}

fn new_player(
    player_id: i32,
    mut commands: &mut Commands,
    mut ids: &mut ResMut<Ids>,
    mut map_events: &mut ResMut<MapEvents>,
    mut items: &mut ResMut<Items>,
    templates: &Res<Templates>,
    game_tick: &ResMut<GameTick>,
) {
    let start_x = 16;
    let start_y = 36;
    let range = 2;

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
        template: Template("Novice Warrior".into()),
        class: Class("unit".into()),
        subclass: Subclass("hero".into()),
        state: State("none".into()),
        viewshed: Viewshed { range: range },
        misc: Misc {
            image: "novicewarrior".into(),
            hsl: Vec::new().into(),
            groups: Vec::new().into(),
        },
        hp: Hp(100),
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

    items.push(berries);
    items.push(water);

    // Spawn hero
    let hero_entity_id = commands
        .spawn((
            hero,
            SubclassHero, // Hero component tag
        ))
        .id();

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
        hp: Hp(100),
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

    //create_item(commands, heroId, "Honeybell Berries".to_owned(), "Food".to_owned(), "Berry".to_owned(), "honeybellberries".to_owned(), 5, 10);
}


