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
    contact_plan::{ContactPlan, RealNode},
    errors::ASABRError,
    node::{Node, NodeInfo},
    node_manager::{NodeManager, none::NoManagement},
    types::{DataRate, Date, Duration, NodeID},
};

extern crate alloc;
use alloc::{
    collections::BTreeMap as HashMap,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use core::cmp::Ordering;

pub struct IONContactData {
    tx_start: Date,
    tx_end: Date,
    tx_node_id: NodeID,
    rx_node_id: NodeID,
    data_rate: DataRate,
    delay: Duration,
    _confidence: f32,
}

// Implement `Ord` and `PartialOrd` for sorting
impl Ord for IONContactData {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).expect("NaN in date?!")
    }
}

#[allow(clippy::non_canonical_partial_ord_impl)]
impl PartialOrd for IONContactData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.tx_start.partial_cmp(&other.tx_start)
    }
}

impl PartialEq for IONContactData {
    fn eq(&self, other: &Self) -> bool {
        self.tx_start == other.tx_start
    }
}

impl Eq for IONContactData {}

struct IONRangeData {
    tx_start: Date,
    tx_end: Date,
    tx_node_id: NodeID,
    rx_node_id: NodeID,
    delay: Duration,
}

fn contact_info_from_tvg_data(data: &IONContactData) -> ContactInfo {
    ContactInfo::new(data.tx_node_id, data.rx_node_id, data.tx_start, data.tx_end)
}

pub trait FromIONContactData<CM: ContactManager> {
    fn ion_convert(data: &IONContactData) -> Option<(Contact<CM>, usize, usize)>;
}

macro_rules! generate_for_evl_variants {
    ($cm_name:ident) => {
        impl FromIONContactData<$cm_name> for $cm_name {
            fn ion_convert(data: &IONContactData) -> Option<(Contact<$cm_name>, usize, usize)> {
                let contact_info = contact_info_from_tvg_data(&data);
                let manager = $cm_name::new(data.data_rate, data.delay);
                return Contact::try_new(contact_info, manager);
            }
        }
    };
}

generate_for_evl_variants!(EVLManager);
generate_for_evl_variants!(ETOManager);
generate_for_evl_variants!(QDManager);
generate_for_evl_variants!(PEVLManager);
generate_for_evl_variants!(PETOManager);
generate_for_evl_variants!(PQDManager);

impl FromIONContactData<SegmentationManager> for SegmentationManager {
    fn ion_convert(data: &IONContactData) -> Option<(Contact<SegmentationManager>, usize, usize)> {
        let contact_info = contact_info_from_tvg_data(data);
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

pub struct IONContactPlan {}

fn manage_aliases(
    name_to_id_map: &mut HashMap<String, NodeID>,
    candidate_name: &str,
    vertices: &mut Vec<RealNode<NoManagement>>,
) -> NodeID {
    if let Some(value) = name_to_id_map.get(candidate_name) {
        *value
    } else {
        let next = name_to_id_map.len().into();
        name_to_id_map.insert(candidate_name.to_string(), next);
        vertices.push(RealNode::Inode(
            Node::try_new(
                NodeInfo {
                    id: next as NodeID,
                    name: candidate_name.into(),
                    excluded: false,
                },
                NoManagement {},
            )
            .unwrap(),
        ));
        next
    }
}

fn manage_contacts(
    contact_map: &mut HashMap<NodeID, HashMap<NodeID, Vec<IONContactData>>>,
    contact: IONContactData,
) {
    let tx_node_id = contact.tx_node_id;
    let rx_node_id = contact.rx_node_id;

    if let Some(inner_map) = contact_map.get_mut(&tx_node_id) {
        inner_map
            .entry(rx_node_id)
            .or_insert_with(Vec::new)
            .push(contact);
    } else {
        let mut inner_map = HashMap::new();
        inner_map.insert(rx_node_id, vec![contact]);
        contact_map.insert(tx_node_id, inner_map);
    }
}

fn get_confidence(vec: &[&str]) -> f32 {
    if vec.len() >= 8 {
        vec[7].parse::<f32>().unwrap()
    } else {
        1.0
    }
}

impl IONContactPlan {
    pub fn parse<NM: NodeManager, CM: FromIONContactData<CM> + ContactManager>(
        content: impl Iterator<Item: AsRef<str>>,
    ) -> Result<ContactPlan<NoManagement, CM>, ASABRError> {
        let reader = content;
        let mut map_id_map = HashMap::new();

        let mut ranges = vec![];
        let mut contact_info_map: HashMap<NodeID, HashMap<NodeID, Vec<IONContactData>>> =
            HashMap::new();

        let mut contact_count = 0;
        let mut contacts = vec![];
        let mut vertices = vec![];

        for line in reader {
            // Skip lines starting with '#'
            if line.as_ref().trim_start().starts_with('#') {
                continue;
            }
            let words: Vec<_> = line.as_ref().split_whitespace().collect();

            if words.is_empty() {
                continue;
            }

            if words[0] != "a" {
                continue;
            }

            if words[1] == "contact" {
                let tx_start: Date = words[2].parse().unwrap();
                let tx_end: Date = words[3].parse().unwrap();
                let tx_node_id = manage_aliases(&mut map_id_map, words[4], &mut vertices);
                let rx_node_id = manage_aliases(&mut map_id_map, words[5], &mut vertices);
                let data_rate: DataRate = words[6].parse().unwrap();
                let confidence = get_confidence(words.as_slice());
                contact_count += 1;

                manage_contacts(
                    &mut contact_info_map,
                    IONContactData {
                        tx_start,
                        tx_end,
                        tx_node_id,
                        rx_node_id,
                        data_rate,
                        delay: 0,
                        _confidence: confidence,
                    },
                );
            }
            if words[1] == "range" {
                let tx_start: Date = words[2].parse().unwrap();
                let tx_end: Date = words[3].parse().unwrap();
                let tx_node_id = manage_aliases(&mut map_id_map, words[4], &mut vertices);
                let rx_node_id = manage_aliases(&mut map_id_map, words[5], &mut vertices);
                let delay: Duration = words[6].parse().unwrap();
                ranges.push(IONRangeData {
                    tx_start,
                    tx_end,
                    tx_node_id,
                    rx_node_id,
                    delay,
                });
            }
        }

        for map in contact_info_map.values_mut() {
            for contacts in map.values_mut() {
                contacts.sort_unstable();
            }
        }

        for range in &ranges {
            if let Some(tx_map) = contact_info_map.get_mut(&range.tx_node_id)
                && let Some(contact_vec) = tx_map.get_mut(&range.rx_node_id)
            {
                for contact in contact_vec.iter_mut() {
                    if range.tx_start <= contact.tx_start && contact.tx_end <= range.tx_end {
                        contact.delay = range.delay;
                        contacts.push(CM::ion_convert(contact).unwrap());
                    } else {
                        return Err(ASABRError::ContactPlanError(
                            "This parser only supports one range per contact",
                        ));
                    }
                }
            }
        }

        if contacts.len() != contact_count {
            return Err(ASABRError::ContactPlanError(
                "At least one contact has no range",
            ));
        }

        Ok(ContactPlan::new(vertices, Vec::new(), contacts))
    }
}
