use crate::{
    contact::{Contact, ContactInfo},
    contact_manager::ContactManager,
    contact_plan::ContactPlan,
    node::{Node, NodeInfo},
    parsing::{Parser, StaticMarkerMap},
    types::{NodeID, NodeIDMap, NodeName},
    vertex::Vertex,
    vnode::{VirtualNodeInfo, VirtualNodeMap},
};
use crate::{
    node_manager::NodeManager,
    parsing::{DispatchParser, Lexer, ParsingState, parse_components},
};
use std::collections::{HashMap, HashSet};

enum RealNodeType {
    Node,
    Enode,
}

impl RealNodeType {
    fn to_vertex<NM: NodeManager>(&self, node: Node<NM>) -> Vertex<NM> {
        match self {
            RealNodeType::Node => Vertex::INode(node),
            RealNodeType::Enode => Vertex::ENode(node),
        }
    }
}

struct Builder<NM: NodeManager, CM: ContactManager> {
    // for output
    vertices: Vec<Vertex<NM>>,
    vnode_map: NodeIDMap,
    contacts: Vec<Contact<NM, CM>>,

    // Unicity
    node_names: HashSet<NodeName>,
}

impl<NM: NodeManager, CM: ContactManager> Builder<NM, CM> {
    fn new() -> Self {
        Self {
            vertices: Vec::new(),
            vnode_map: HashMap::new(),
            contacts: Vec::new(),
            node_names: HashSet::new(),
        }
    }

    #[inline(always)]
    fn real_nodes_count(&self) -> usize {
        self.vertices.len() - self.vnode_map.len()
    }

    // Checkers

    fn check_real_id(&self, id: NodeID) -> Result<(), String> {
        if (id as usize) >= self.real_nodes_count() {
            return Err(
                "Contact tx/rx ids or virtual node rids must match an already declared real node id".to_string(),
            );
        }
        Ok(())
    }

    fn check_new_real_id(&self, id: NodeID) -> Result<(), String> {
        if (id as usize) != self.real_nodes_count() {
            return Err(
                "Declare real nodes before virtual nodes, in increasing id order".to_string(),
            );
        }
        Ok(())
    }

    fn check_new_virtual_id(&self, id: NodeID) -> Result<(), String> {
        if (id as usize) != self.vertices.len() {
            return Err(
                "Declare virtual nodes after the real nodes, in increasing id order".to_string(),
            );
        }
        Ok(())
    }

    fn check_enodes_have_vnodes(&self) -> Result<(), String> {
        for vertex in &self.vertices {
            if let Vertex::ENode(enode) = vertex
                && !self
                    .vnode_map
                    .values()
                    .any(|rids| rids.contains(&enode.info.id))
            {
                return Err(format!(
                    "ENode '{}' (id: {}) is not labeled by any vnode",
                    enode.info.name, enode.info.id,
                ));
            }
        }
        Ok(())
    }

    // Adders

    fn register_name(&mut self, name: String) -> Result<(), String> {
        if !self.node_names.insert(name) {
            return Err("Another vertex shares this name".to_string());
        }
        Ok(())
    }

    fn add_real_node(&mut self, node: Node<NM>, node_type: RealNodeType) -> Result<(), String> {
        self.check_new_real_id(node.get_node_id())?;
        self.register_name(node.get_node_name())?;
        self.vertices.push(node_type.to_vertex(node));
        Ok(())
    }

    fn add_contact(&mut self, contact: Contact<NM, CM>) -> Result<(), String> {
        self.check_real_id(contact.info.tx_node_id)?;
        self.check_real_id(contact.info.rx_node_id)?;
        self.contacts.push(contact);
        Ok(())
    }

    fn add_virtual_node(&mut self, vnode: VirtualNodeInfo) -> Result<(), String> {
        self.check_new_virtual_id(vnode.vid)?;
        self.register_name(vnode.name)?;
        for rid in &vnode.rids {
            self.check_real_id(*rid)?;
        }
        self.vertices.push(Vertex::VNode(vnode.vid));
        self.vnode_map.insert(vnode.vid, vnode.rids);
        Ok(())
    }

    // Builder
    fn build(self) -> Result<ContactPlan<NM, CM>, String> {
        self.check_enodes_have_vnodes()?;
        ContactPlan::new(
            self.vertices,
            self.contacts,
            Some(VirtualNodeMap::new(self.vnode_map)),
        )
        .map_err(|_| "Failed to create contact plan".to_string())
    }
}

pub struct ASABRContactPlan {}

impl ASABRContactPlan {
    // Helper for enode/node duplication
    fn parse_node<NM: NodeManager + DispatchParser<NM> + Parser<NM>>(
        lexer: &mut dyn Lexer,
        node_marker_map: &Option<&StaticMarkerMap<NM>>,
    ) -> Result<Node<NM>, String> {
        let node_opt = parse_components::<NodeInfo, NM>(lexer, *node_marker_map);
        match node_opt {
            ParsingState::EOF => Err("Unexpected EOF".to_string()),
            ParsingState::Error(msg) => Err(msg),
            ParsingState::Finished((info, manager)) => {
                let Some(node) = Node::try_new(info, manager) else {
                    return Err(format!("Malformed node ({})", lexer.get_current_position()));
                };
                Ok(node)
            }
        }
    }

    pub fn parse<
        NM: NodeManager + DispatchParser<NM> + Parser<NM>,
        CM: ContactManager + DispatchParser<CM> + Parser<CM>,
    >(
        lexer: &mut dyn Lexer,
        node_marker_map: Option<&StaticMarkerMap<NM>>,
        contact_marker_map: Option<&StaticMarkerMap<CM>>,
    ) -> Result<ContactPlan<NM, CM>, String> {
        let mut builder = Builder::new();

        loop {
            let res = lexer.consume_next_token();

            match res {
                ParsingState::EOF => {
                    break;
                }
                ParsingState::Error(msg) => {
                    return Err(msg);
                }
                ParsingState::Finished(element_type) => match element_type.as_str() {
                    "contact" => {
                        let contact_opt =
                            parse_components::<ContactInfo, CM>(lexer, contact_marker_map);
                        match contact_opt {
                            ParsingState::EOF => {
                                break;
                            }
                            ParsingState::Error(msg) => {
                                return Err(msg);
                            }
                            ParsingState::Finished((info, manager)) => {
                                let Some(contact) = Contact::try_new(info, manager) else {
                                    return Err(format!(
                                        "Malformed contact ({})",
                                        lexer.get_current_position()
                                    ));
                                };
                                builder.add_contact(contact)?;
                            }
                        }
                    }
                    "node" => {
                        builder.add_real_node(
                            Self::parse_node(lexer, &node_marker_map)?,
                            RealNodeType::Node,
                        )?;
                    }
                    "enode" => {
                        builder.add_real_node(
                            Self::parse_node(lexer, &node_marker_map)?,
                            RealNodeType::Enode,
                        )?;
                    }
                    "vnode" => {
                        let vnode_opt =
                            parse_components::<VirtualNodeInfo, NM>(lexer, node_marker_map);
                        match vnode_opt {
                            ParsingState::EOF => {
                                break;
                            }
                            ParsingState::Error(msg) => {
                                return Err(msg);
                            }
                            ParsingState::Finished((info, _)) => {
                                builder.add_virtual_node(info)?;
                            }
                        }
                    }
                    _ => {
                        return Err(format!(
                            "Unrecognized CP element ({})",
                            lexer.get_current_position()
                        ));
                    }
                },
            }
        }
        builder.build()
    }
}
