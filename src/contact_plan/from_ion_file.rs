use crate::{
    contact::{self, Contact, ContactInfo},
    contact_manager::{
        eto::ETOManager,
        evl::EVLManager,
        qd::QDManager,
        seg::{Segment, SegmentationManager},
        ContactManager,
    },
    node::{Node, NodeInfo},
    node_manager::none::NoManagement,
    types::{DataRate, Date, Duration, NodeID},
};

use std::{cmp::Ordering, collections::HashMap, hash::RandomState};
use std::{
    fs::File,
    io::{self, BufRead, BufReader},
};

struct IONContactData {
    tx_start: Date,
    tx_end: Date,
    tx_node: NodeID,
    rx_node: NodeID,
    data_rate: DataRate,
    delay: Duration,
    confidence: f32,
}

// Implement `Ord` and `PartialOrd` for sorting
impl Ord for IONContactData {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.tx_start > other.tx_start {
            return Ordering::Greater;
        }
        if self.tx_start < other.tx_start {
            return Ordering::Less;
        }
        return Ordering::Equal;
    }
}

impl PartialOrd for IONContactData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
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
    tx_node: NodeID,
    rx_node: NodeID,
    delay: Duration,
}

fn contact_info_from_tvg_data(data: &IONContactData) -> ContactInfo {
    return ContactInfo::new(data.tx_node, data.rx_node, data.tx_start, data.tx_end);
}

pub trait FromIONContactData<CM: ContactManager> {
    fn tvg_convert(data: IONContactData) -> Option<Contact<CM>>;
}

macro_rules! generate_for_evl_variants {
    ($manager_name:ident) => {
        impl FromIONContactData<EVLManager> for $manager_name {
            fn tvg_convert(data: IONContactData) -> Option<Contact<$manager_name>> {
                let contact_info = contact_info_from_tvg_data(&data);
                let manager = $manager_name::new(data.data_rate, data.delay);
                return Contact::try_new(contact_info, manager);
            }
        }
    };
}

generate_for_evl_variants!(EVLManager);
generate_for_evl_variants!(ETOManager);
generate_for_evl_variants!(QDManager);

impl FromIONContactData<SegmentationManager> for SegmentationManager {
    fn tvg_convert(data: IONContactData) -> Option<Contact<SegmentationManager>> {
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

pub struct IONContactPlan {}

fn manage_aliases(
    map_id_map: &mut HashMap<String, NodeID>,
    candidate_name: &String,
    nodes: &mut Vec<Node<NoManagement>>,
) -> NodeID {
    if let Some(value) = map_id_map.get(candidate_name.as_str()) {
        return *value;
    } else {
        let next = map_id_map.len() as NodeID;
        map_id_map.insert(candidate_name.clone(), next);
        nodes.push(
            Node::try_new(
                NodeInfo {
                    id: next as NodeID,
                    name: candidate_name.to_string(),
                    excluded: false,
                },
                NoManagement {},
            )
            .unwrap(),
        );
        return next;
    }
}

fn manage_contacts(
    contact_map: &mut HashMap<NodeID, HashMap<NodeID, Vec<IONContactData>>>,
    contact: IONContactData,
) {
    let tx_node = contact.tx_node;
    let rx_node = contact.rx_node;

    if let Some(inner_map) = contact_map.get_mut(&tx_node) {
        inner_map
            .entry(rx_node)
            .or_insert_with(Vec::new)
            .push(contact);
    } else {
        let mut inner_map = HashMap::new();
        inner_map.insert(rx_node, vec![contact]);
        contact_map.insert(tx_node, inner_map);
    }
}

fn get_confidence(vec: &Vec<String>) -> f32 {
    if vec.len() >= 8 {
        vec[7].parse::<f32>().unwrap()
    } else {
        1.0
    }
}

impl IONContactData {
    pub fn parse<CM: FromIONContactData<CM> + ContactManager>(
        filename: &str,
    ) -> Result<(Vec<Node<NoManagement>>, Vec<Contact<CM>>), String> {
        let file = File::open(filename)?;
        let reader = BufReader::new(file);
        let mut map_id_map: HashMap<String, NodeID> = HashMap::new();

        let ranges = vec![];
        let mut contact_info_map: HashMap<NodeID, HashMap<NodeID, Vec<IONContactData>>> =
            HashMap::new();

        let mut contact_count = 0;
        let mut contacts = vec![];
        let mut nodes = vec![];

        loop {
            let mut line = String::new();
            let bytes_read = reader.read_line(&mut line)?;

            if bytes_read == 0 {
                break;
            }
            // Skip lines starting with '#'
            if line.trim_start().starts_with('#') {
                continue;
            }
            let words: Vec<String> = line.split_whitespace().rev().map(String::from).collect();

            if words[0].as_str() != "a" {
                continue;
            }
            if words[1].as_str() == "contact" {
                let tx_start: Date = words[2].parse().unwrap();
                let tx_end: Date = words[3].parse().unwrap();
                let tx_node = manage_aliases(&mut map_id_map, &words[4], &mut nodes);
                let rx_node = manage_aliases(&mut map_id_map, &words[5], &mut nodes);
                let data_rate: DataRate = words[6].parse().unwrap();
                let confidence = get_confidence(&words);
                contact_count += 1;

                manage_contacts(
                    &mut contact_info_map,
                    IONContactData {
                        tx_start,
                        tx_end,
                        tx_node,
                        rx_node,
                        data_rate,
                        delay: 0.0,
                        confidence,
                    },
                );
            }
            if words[1].as_str() == "range" {
                let tx_start: Date = words[2].parse().unwrap();
                let tx_end: Date = words[3].parse().unwrap();
                let tx_node = manage_aliases(&mut map_id_map, &words[4], &mut nodes);
                let rx_node = manage_aliases(&mut map_id_map, &words[5], &mut nodes);
                let delay: Duration = words[6].parse().unwrap();
                ranges.push(IONRangeData {
                    tx_start,
                    tx_end,
                    tx_node,
                    rx_node,
                    delay,
                });
            }
            continue;
        }

        for (_tx, map) in contact_info_map {
            for (_rx, contacts) in map {
                contacts.sort_unstable();
            }
        }

        for range in ranges {
            for mut contact in contact_info_map
                .get(&range.tx_node)
                .unwrap()
                .get(&range.rx_node)
                .unwrap()
            {
                if range.tx_start <= contact.tx_start && contact.tx_end <= range.tx_end {
                    contact.delay = range.delay;
                    contacts.push(contact.clone());
                } else {
                    panic!("This parser only support one range per contact");
                }
            }
        }

        if contacts.len() != contact_count {
            panic!("At least one contact has no range");
        }

        Ok((nodes, contacts))
    }
}
