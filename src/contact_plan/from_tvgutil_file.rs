use crate::{
    contact::{Contact, ContactInfo},
    contact_manager::{
        ContactManager,
        legacy::{
            eto::{ETOManager, PETOManager},
            evl::{EVLManager, PEVLManager},
            qd::{PQDManager, QDManager},
        },
        segmentation::{Segment, seg::SegmentationManager},
    },
    contact_plan::ContactPlan,
    node::{Node, NodeInfo},
    node_manager::{NodeManager, none::NoManagement},
    types::{DataRate, Date, Duration, NodeID},
    vertex::Vertex,
    vnode::VirtualNodeMap,
};

use std::{collections::HashMap, io};

use serde_json::Value;
use std::fs;

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct TVGUtilContactData {
    tx_start: Date,
    tx_end: Date,
    tx_node_id: NodeID,
    rx_node_id: NodeID,
    delay: Duration,
    data_rate: DataRate,
    _confidence: f32,
}

fn contact_info_from_tvg_data(data: &TVGUtilContactData) -> ContactInfo {
    ContactInfo::new(data.tx_node_id, data.rx_node_id, data.tx_start, data.tx_end)
}

pub trait FromTVGUtilContactData<NM: NodeManager, CM: ContactManager> {
    fn tvg_convert(data: TVGUtilContactData) -> Option<Contact<NoManagement, CM>>;
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
generate_for_evl_variants!(NoManagement, PEVLManager);
generate_for_evl_variants!(NoManagement, PETOManager);
generate_for_evl_variants!(NoManagement, PQDManager);

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
        Contact::try_new(contact_info, manager)
    }
}

pub struct TVGUtilContactPlan {}

impl TVGUtilContactPlan {
    pub fn parse<NM: NodeManager, CM: FromTVGUtilContactData<NM, CM> + ContactManager>(
        filename: &str,
    ) -> io::Result<ContactPlan<NoManagement, CM>> {
        let mut vertices: Vec<Vertex<NoManagement>> = Vec::new();
        let mut contacts: Vec<Contact<NoManagement, CM>> = Vec::new();

        let mut map_id_map: HashMap<&str, NodeID> = HashMap::new();
        let mut vnode_to_rids_map: HashMap<NodeID, Vec<NodeID>> = HashMap::new();
        let mut rid_to_vnodes_map: HashMap<NodeID, Vec<NodeID>> = HashMap::new();

        let json_data = fs::read_to_string(filename)?;
        let parsed: Value = serde_json::from_str(&json_data).unwrap();
        let json_nodes = parsed["vertices"].as_object().unwrap();

        for (node_id, (node_name, node_data)) in json_nodes.iter().enumerate() {
            map_id_map.insert(node_name, node_id as NodeID);
            let node = Node::try_new(
                NodeInfo {
                    id: node_id as NodeID,
                    name: node_name.to_string(),
                    excluded: false,
                },
                NoManagement {},
            )
            .unwrap();
            let vertex = match node_data["type"].as_str().unwrap_or("inode") {
                "enode" => Vertex::ENode(node),
                _ => Vertex::INode(node),
            };
            vertices.push(vertex);
        }

        let real_node_count = vertices.len();

        if let Some(json_vnodes) = parsed["vnodes"].as_array() {
            for (vnode_offset, vnode_data) in json_vnodes.iter().enumerate() {
                let vid = (real_node_count + vnode_offset) as NodeID;
                let vnode_obj = vnode_data.as_object().unwrap();
                let members = vnode_obj["members"].as_array().unwrap();
                let mut rids: Vec<NodeID> = Vec::new();
                for member in members {
                    let member_name = member.as_str().unwrap();
                    let rid = *map_id_map.get(member_name).ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("VNode member '{}' is not a known vertex", member_name),
                        )
                    })?;
                    rids.push(rid);
                    rid_to_vnodes_map.entry(rid).or_default().push(vid);
                }
                vnode_to_rids_map.insert(vid, rids);
                vertices.push(Vertex::VNode(vid));
            }
        }

        for vertex in &vertices {
            if let Vertex::ENode(enode) = vertex
                && !rid_to_vnodes_map.contains_key(&enode.info.id)
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "ENode '{}' (id: {}) is not labeled by any vnode",
                        enode.info.name, enode.info.id
                    ),
                ));
            }
        }

        let json_contacts = parsed["edges"].as_array().unwrap();
        for nodes_pair in json_contacts {
            let data = nodes_pair.as_object().unwrap();
            let pair = data["vertices"].as_array().unwrap();
            let tx_node_id = map_id_map.get(pair[0].as_str().unwrap()).unwrap();
            let rx_node_id = map_id_map.get(pair[1].as_str().unwrap()).unwrap();

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
                    tx_node_id: *tx_node_id,
                    rx_node_id: *rx_node_id,
                    delay,
                    data_rate,
                    _confidence: confidence,
                };

                let contact = CM::tvg_convert(tvgcontact).unwrap();

                contacts.push(contact);
            }
        }

        let vnode_map = if vnode_to_rids_map.is_empty() {
            None
        } else {
            Some(VirtualNodeMap::new(vnode_to_rids_map, rid_to_vnodes_map))
        };

        Ok(ContactPlan::new(vertices, contacts, vnode_map)?)
    }
}
