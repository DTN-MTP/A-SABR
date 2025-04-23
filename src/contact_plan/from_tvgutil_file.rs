use crate::{
    contact::{Contact, ContactInfo},
    contact_manager::{
        eto::ETOManager,
        evl::EVLManager,
        qd::QDManager,
        seg::{Segment, SegmentationManager},
        ContactManager,
    },
    node::{Node, NodeInfo},
    node_manager::{none::NoManagement, NodeManager},
    types::{DataRate, Date, Duration, NodeID},
};

use std::{collections::HashMap, io};

use serde_json::Value;
use std::fs;

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct TVGUtilContactData {
    tx_start: Date,
    tx_end: Date,
    tx_node: NodeID,
    rx_node: NodeID,
    delay: Duration,
    data_rate: DataRate,
    confidence: f32,
}

fn contact_info_from_tvg_data(data: &TVGUtilContactData) -> ContactInfo {
    return ContactInfo::new(data.tx_node, data.rx_node, data.tx_start, data.tx_end);
}

pub trait FromTVGUtilContactData<NM: NodeManager, CM: ContactManager> {
    fn tvg_convert(data: TVGUtilContactData) -> Option<Contact<NM, CM>>;
}

macro_rules! generate_for_evl_variants {
    ($nm_name:ident, $cm_name:ident) => {
        impl FromTVGUtilContactData<$nm_name, $cm_name> for $cm_name {
            fn tvg_convert(data: TVGUtilContactData) -> Option<Contact<$nm_name, $cm_name>> {
                let contact_info = contact_info_from_tvg_data(&data);
                let manager = $cm_name::new(data.data_rate, data.delay);
                return Contact::try_new(contact_info, manager);
            }
        }
    };
}

generate_for_evl_variants!(NoManagement, EVLManager);
generate_for_evl_variants!(NoManagement, ETOManager);
generate_for_evl_variants!(NoManagement, QDManager);

impl FromTVGUtilContactData<NoManagement, SegmentationManager> for SegmentationManager {
    fn tvg_convert(data: TVGUtilContactData) -> Option<Contact<NoManagement, SegmentationManager>> {
        let contact_info = contact_info_from_tvg_data(&data);
        let manager = SegmentationManager::new(
            vec![Segment::<DataRate> {
                start: data.tx_start,
                end: data.tx_end,
                val: data.data_rate,
            }],
            vec![Segment::<Duration> {
                start: data.tx_start,
                end: data.tx_end,
                val: data.delay,
            }],
        );
        return Contact::try_new(contact_info, manager);
    }
}

pub struct TVGUtilContactPlan {}

impl TVGUtilContactPlan {
    pub fn parse<NM: NodeManager, CM: FromTVGUtilContactData<NM, CM> + ContactManager>(
        filename: &str,
    ) -> io::Result<(Vec<Node<NoManagement>>, Vec<Contact<NM, CM>>)> {
        let mut nodes: Vec<Node<NoManagement>> = Vec::new();
        let mut contacts: Vec<Contact<NM, CM>> = Vec::new();

        let mut map_id_map: HashMap<&str, NodeID> = HashMap::new();

        let json_data = fs::read_to_string(filename)?;
        let parsed: Value = serde_json::from_str(&json_data).unwrap();
        let json_nodes = parsed["vertices"].as_object().unwrap();

        for (node_id, (node_name, _node_data)) in json_nodes.iter().enumerate() {
            map_id_map.insert(&node_name, node_id as NodeID);
            nodes.push(
                Node::try_new(
                    NodeInfo {
                        id: node_id as NodeID,
                        name: node_name.to_string(),
                        excluded: false,
                    },
                    NoManagement {},
                )
                .unwrap(),
            );
        }

        let json_contacts = parsed["edges"].as_array().unwrap();
        for nodes_pair in json_contacts {
            let data = nodes_pair.as_object().unwrap();
            let pair = data["vertices"].as_array().unwrap();
            let tx_node = map_id_map.get(pair[0].as_str().unwrap()).unwrap();
            let rx_node = map_id_map.get(pair[1].as_str().unwrap()).unwrap();

            for contact_data in data["contacts"].as_array().unwrap() {
                let contact_array = contact_data.as_array().unwrap();
                let start = contact_array[2].as_f64().unwrap() as Date;
                let end = contact_array[3].as_f64().unwrap() as Date;
                let first_level_array = contact_array[4].as_array().unwrap();
                let second_level_array = first_level_array[0].as_array().unwrap();
                let confidence = second_level_array[1].as_f64().unwrap() as f32;
                let third_level_array = second_level_array[2].as_array().unwrap();
                let fourth_level_array = third_level_array[0].as_array().unwrap();
                let data_rate = fourth_level_array[1].as_f64().unwrap() as DataRate;
                let delay = fourth_level_array[2].as_f64().unwrap() as Duration;

                let tvgcontact = TVGUtilContactData {
                    tx_start: start,
                    tx_end: end,
                    tx_node: *tx_node,
                    rx_node: *rx_node,
                    delay,
                    data_rate,
                    confidence,
                };

                let contact = CM::tvg_convert(tvgcontact).unwrap();

                contacts.push(contact);
            }
        }
        Ok((nodes, contacts))
    }
}
