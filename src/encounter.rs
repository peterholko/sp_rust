use std::collections::HashMap;
use std::i32::MAX;

use bevy::prelude::*;
use big_brain::actions::{Steps};
use big_brain::prelude::{Highest, Thinker};

use rand::{Rng};

use crate::components::npc::{
    AtLanding, Destination, Forfeiture, Hide, Idle, IsAboard, IsTaxCollected, MoveToEmpire,
    MoveToPos, MoveToTarget, NoTaxesToCollect, OverdueTaxScorer, Talk, TaxCollector,
    TaxCollectorTransport, TaxesToCollect, Transport, VisibleTarget,
};
use crate::components::npc::{
    ChaseAndAttack, ChaseAndCast, FleeScorer, FleeToHome, RaiseDead, VisibleCorpse,
    VisibleCorpseScorer, VisibleTargetScorer,
};
use crate::effect::Effects;
use crate::event::{MapEvents, VisibleEvent};
use crate::game::{
    Class, GameTick, Home, Id, Minions, Misc, Name, PlayerId, Position, State, StateAboard, Stats,
    Subclass, SubclassNPC, Template, Viewshed,
};
use crate::ids::Ids;
use crate::item::{Items};
use crate::map::TileType;
use crate::obj::Obj;
use crate::plugins::ai::npc::NO_TARGET;

use crate::templates::{ObjTemplate, Templates};

#[derive(Debug, Clone)]
pub struct Encounter;

#[derive(Debug, Clone)]
struct Loot {
    item_name: String,
    drop_rate: f32,
    min: i32,
    max: i32,
}

impl Encounter {
    pub fn spawn_npc(
        player_id: i32,
        pos: Position,
        template: String,
        commands: &mut Commands,
        ids: &mut ResMut<Ids>,
        items: &mut ResMut<Items>,
        templates: &Res<Templates>,
    ) -> (Entity, Id, PlayerId, Position) {
        let npc_id = ids.new_obj_id();
        return Self::spawn_npc_with_id(
            npc_id, player_id, pos, template, commands, ids, items, templates,
        );
    }

    pub fn spawn_npc_with_id(
        npc_id: i32,
        player_id: i32,
        pos: Position,
        template: String,
        commands: &mut Commands,
        ids: &mut ResMut<Ids>,
        items: &mut ResMut<Items>,
        templates: &Res<Templates>,
    ) -> (Entity, Id, PlayerId, Position) {
        let npc_template = ObjTemplate::get_template(template, templates);

        let image: String = npc_template
            .template
            .to_lowercase()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();

        let npc = Obj {
            id: Id(npc_id),
            player_id: PlayerId(player_id),
            position: pos,
            name: Name(npc_template.name.clone()),
            template: Template(npc_template.template.clone()),
            class: Class(npc_template.class.clone()),
            subclass: Subclass(npc_template.subclass.clone()),
            state: State::None,
            viewshed: Viewshed { range: 2 },
            misc: Misc {
                image: image,
                hsl: Vec::new().into(),
                groups: Vec::new().into(),
            },
            stats: Stats {
                hp: npc_template.base_hp.unwrap(),
                base_hp: npc_template.base_hp.unwrap(),
                stamina: npc_template.base_stamina,
                base_stamina: npc_template.base_stamina,
                base_def: npc_template.base_def.unwrap(),
                base_damage: npc_template.base_dmg,
                damage_range: npc_template.dmg_range,
                base_speed: npc_template.base_speed,
                base_vision: npc_template.base_vision,
            },
            effects: Effects(HashMap::new()),
        };

        let entity = commands
            .spawn((
                npc,
                SubclassNPC,
                VisibleTarget::new(NO_TARGET),
                Thinker::build()
                    .label("NPC Chase")
                    .picker(Highest)
                    .when(VisibleTargetScorer, ChaseAndAttack),
            ))
            .id();

        Encounter::generate_loot(npc_id, ids, items, templates);

        ids.new_obj(npc_id, player_id, entity);

        return (entity, Id(npc_id), PlayerId(player_id), pos);
    }

    pub fn spawn_necromancer(
        player_id: i32,
        pos: Position,
        home_pos: Position,
        commands: &mut Commands,
        ids: &mut ResMut<Ids>,
        items: &mut ResMut<Items>,
        templates: &Res<Templates>,
    ) -> (Entity, Id, PlayerId, Position) {
        let necro_obj = Obj::create_nospawn(
            ids,
            player_id,
            "Necromancer".to_string(),
            //Position { x: 16, y: 33 },
            pos,
            State::None,
            templates,
        );

        let flee_and_hide = Steps::build()
            .label("Flee and Hide")
            .step(FleeToHome)
            .step(Hide)
            .step(Idle {
                start_time: 0,
                duration: MAX,
            });

        // Spawn Necromancer
        let necro_entity = commands
            .spawn((
                necro_obj.clone(),
                SubclassNPC,
                Minions { ids: Vec::new() },
                Home { pos: home_pos },
                VisibleTarget::new(NO_TARGET),
                VisibleCorpse::new(NO_TARGET),
                Thinker::build()
                    .label("Necromancer")
                    .picker(Highest)
                    .when(VisibleTargetScorer, ChaseAndCast { start_time: MAX })
                    .when(VisibleCorpseScorer, RaiseDead { start_time: MAX })
                    .when(FleeScorer, flee_and_hide),
            ))
            .id();

        ids.new_obj(necro_obj.id.0, player_id, necro_entity);

        Encounter::generate_loot(necro_obj.id.0, ids, items, templates);

        return (necro_entity, necro_obj.id, PlayerId(player_id), pos);
    }

    pub fn spawn_tax_collector(
        player_id: i32,
        landing_pos: Position,
        empire_pos: Position,
        target_player: i32,
        commands: &mut Commands,
        ids: &mut ResMut<Ids>,
        items: &mut ResMut<Items>,
        templates: &Res<Templates>,
        game_tick: &Res<GameTick>,
        map_events: &mut ResMut<MapEvents>,
    ) {
        let tax_collector_ship_obj = Obj::create_nospawn(
            ids,
            player_id,
            "Tax Ship".to_string(),
            empire_pos,
            State::None,
            templates,
        );

        let tax_collector_obj = Obj::create_nospawn(
            ids,
            player_id,
            "Tax Collector".to_string(),
            empire_pos,
            State::None,
            templates,
        );

        let move_to_empire_and_idle = Steps::build()
            .label("MoveToEmpire and Idle")
            .step(MoveToEmpire)
            .step(Idle {
                start_time: 0,
                duration: 100,
            });

        let move_to_landing_and_idle = Steps::build()
            .label("MoveToPos and Idle")
            .step(MoveToPos)
            .step(Idle {
                start_time: 0,
                duration: 100,
            });

        // Spawn Tax Collector Ship
        let tax_collector_ship_entity = commands
            .spawn((
                tax_collector_ship_obj.clone(),
                SubclassNPC,
                Transport {
                    route: Vec::new(),
                    next_stop: 0,
                    hauling: vec![tax_collector_obj.id.0],
                },
                Destination { pos: landing_pos },
                TaxCollectorTransport {
                    tax_collector_id: tax_collector_obj.id.0,
                },
                Thinker::build()
                    .label("Tax Collector Ship")
                    .picker(Highest)
                    .when(NoTaxesToCollect, move_to_empire_and_idle)
                    .when(TaxesToCollect, move_to_landing_and_idle),
            ))
            .id();

        ids.new_obj(
            tax_collector_ship_obj.id.0,
            player_id,
            tax_collector_ship_entity,
        );

        map_events.new(
            tax_collector_ship_obj.id.0,
            game_tick.0 + 1,
            VisibleEvent::NewObjEvent { new_player: false },
        );

        let target_hero_id = ids
            .get_hero(target_player)
            .expect("Cannot find hero for player");

        let move_to_hero_and_idle = Steps::build()
            .label("MoveToTarget and Idle")
            .step(MoveToTarget {
                target: target_hero_id,
            })
            .step(Idle {
                start_time: 0,
                duration: 100,
            });

        let move_to_ship_and_idle = Steps::build()
            .label("MoveToTarget and Idle")
            .step(Talk {
                speech: "The poor rabble actually paid, shocking!".to_string(),
            })
            .step(MoveToTarget {
                target: tax_collector_ship_obj.id.0,
            })
            .step(Idle {
                start_time: 0,
                duration: 100,
            });

        let forfeiture = Steps::build()
            .label("Forfeiture")
            .step(MoveToTarget {
                target: target_hero_id,
            })
            .step(Forfeiture);

        // Spawn Tax Collector
        let tax_collector_entity = commands
            .spawn((
                tax_collector_obj.clone(),
                SubclassNPC,
                TaxCollector {
                    target_player: target_player,
                    collection_amount: 0,
                    debt_amount: 0,
                    last_collection_time: game_tick.0 - 1000,
                    landing_pos: landing_pos,
                    transport_id: tax_collector_ship_obj.id.0,
                    last_demand_time: 0,
                },
                StateAboard {
                    transport_id: tax_collector_ship_obj.id.0,
                },
                Thinker::build()
                    .label("Tax Collector")
                    .picker(Highest)
                    .when(
                        IsAboard,
                        Idle {
                            start_time: 0,
                            duration: 100,
                        },
                    )
                    .when(AtLanding, move_to_hero_and_idle)
                    .when(IsTaxCollected, move_to_ship_and_idle)
                    .when(OverdueTaxScorer, forfeiture),
            ))
            .id();

        ids.new_obj(tax_collector_obj.id.0, player_id, tax_collector_entity);

        Encounter::generate_loot(tax_collector_obj.id.0, ids, items, templates);

        map_events.new(
            tax_collector_obj.id.0,
            game_tick.0 + 1,
            VisibleEvent::NewObjEvent { new_player: false },
        );
    }

    pub fn generate_loot(
        npc_id: i32,
        _ids: &mut ResMut<Ids>,
        items: &mut ResMut<Items>,
        _templates: &Res<Templates>,
    ) {
        let mut rng = rand::thread_rng();

        let loot_list = Self::loot_list();

        for loot in loot_list.iter() {
            let random_num = rng.gen::<f32>();

            if loot.drop_rate > random_num {
                let item_quantity = rng.gen_range(loot.min..loot.max);

                items.create(
                    npc_id,
                    loot.item_name.clone(),
                    item_quantity, //TODO should this be only 1 ?
                );
            }
        }
    }

    pub fn npc_list(tile_type: TileType) -> Vec<&'static str> {
        match tile_type {
            TileType::DeciduousForest => return vec!["Spider", "Wose", "Skeleton"],
            TileType::Snow => return vec!["Wolf", "Yeti"],
            TileType::HillsSnow => return vec!["Wolf", "Yeti"],
            TileType::FrozenForest => return vec!["Wose", "Yeti", "Spider"],
            TileType::Desert => return vec!["Scorpion", "Giant Rat", "Skeleton"],
            TileType::HillsDesert => return vec!["Scorpion", "Giant Rat", "Skeleton"],
            //_ => return vec!["Giant Rat", "Wolf", "Skeleton"],
            _ => return vec!["Wolf"],
        }
    }

    fn loot_list() -> Vec<Loot> {
        let copper_dust = Loot {
            item_name: "Valleyrun Copper Dust".to_string(),
            drop_rate: 0.2,
            min: 1,
            max: 5,
        };

        let grape = Loot {
            item_name: "Amitanian Grape".to_string(),
            drop_rate: 0.5,
            min: 1,
            max: 3,
        };

        let training_axe = Loot {
            item_name: "Copper Training Axe".to_string(),
            drop_rate: 0.02,
            min: 1,
            max: 2,
        };

        let berries = Loot {
            item_name: "Honeybell Berries".to_string(),
            drop_rate: 0.99,
            min: 5,
            max: 10,
        };

        let mana = Loot {
            item_name: "Mana".to_string(),
            drop_rate: 0.75,
            min: 1,
            max: 3,
        };

        let coins = Loot {
            item_name: "Gold Coins".to_string(),
            drop_rate: 0.99,
            min: 1,
            max: 10,
        };

        return vec![copper_dust, grape, training_axe, berries, mana, coins];
    }
}
