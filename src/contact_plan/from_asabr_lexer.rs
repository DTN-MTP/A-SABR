use crate::{
    contact::{Contact, ContactInfo},
    contact_manager::ContactManager,
    contact_plan::ContactPlan,
    errors::ASABRError,
    node::{Node, NodeInfo},
    parsing::{Parser, StaticMarkerMap},
    types::{NodeID, NodeIDMap, NodeName},
    vertex::Vertex,
    vnode::{VirtualNodeInfo, VirtualNodeMap},
};
use crate::{
    node_manager::NodeManager,
    parsing::{DispatchParser, Lexer, LexerOutput, parse_components},
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
    vnode_to_rids_map: NodeIDMap,
    rid_to_vnodes_map: NodeIDMap,
    contacts: Vec<Contact<NM, CM>>,

    // Unicity
    node_names: HashSet<NodeName>,
}

impl<NM: NodeManager, CM: ContactManager> Builder<NM, CM> {
    fn new() -> Self {
        Self {
            vertices: Vec::new(),
            vnode_to_rids_map: HashMap::new(),
            rid_to_vnodes_map: HashMap::new(),
            contacts: Vec::new(),
            node_names: HashSet::new(),
        }
    }

    #[inline(always)]
    fn real_nodes_count(&self) -> usize {
        self.vertices.len() - self.vnode_to_rids_map.len()
    }

    // Checkers

    fn check_real_id(&self, id: NodeID) -> Result<(), ASABRError> {
        if (id as usize) >= self.real_nodes_count() {
            return Err(ASABRError::ParsingError(
                "Contact tx/rx ids or virtual node rids must match an already declared real node id".to_string(),
            ));
        }
        Ok(())
    }

    fn check_new_real_id(&self, id: NodeID) -> Result<(), ASABRError> {
        if (id as usize) != self.real_nodes_count() {
            return Err(ASABRError::ParsingError(
                "Declare real nodes before virtual nodes, in increasing id order".to_string(),
            ));
        }
        Ok(())
    }

    fn check_new_virtual_id(&self, id: NodeID) -> Result<(), ASABRError> {
        if (id as usize) != self.vertices.len() {
            return Err(ASABRError::ParsingError(
                "Declare virtual nodes after the real nodes, in increasing id order".to_string(),
            ));
        }
        Ok(())
    }

    fn check_enodes_have_vnodes(&self) -> Result<(), ASABRError> {
        for vertex in &self.vertices {
            if let Vertex::ENode(enode) = vertex
                && !self.rid_to_vnodes_map.contains_key(&enode.info.id)
            {
                return Err(ASABRError::ParsingError(format!(
                    "ENode '{}' (id: {}) is not labeled by any vnode",
                    enode.info.name, enode.info.id,
                )));
            }
        }
        Ok(())
    }

    // Adders

    fn register_name(&mut self, name: String) -> Result<(), ASABRError> {
        if !self.node_names.insert(name) {
            return Err(ASABRError::ParsingError(
                "Another vertex shares this name".to_string(),
            ));
        }
        Ok(())
    }

    fn add_real_node(&mut self, node: Node<NM>, node_type: RealNodeType) -> Result<(), ASABRError> {
        self.check_new_real_id(node.get_node_id())?;
        self.register_name(node.get_node_name())?;
        self.vertices.push(node_type.to_vertex(node));
        Ok(())
    }

    fn add_contact(&mut self, contact: Contact<NM, CM>) -> Result<(), ASABRError> {
        self.check_real_id(contact.info.tx_node_id)?;
        self.check_real_id(contact.info.rx_node_id)?;
        self.contacts.push(contact);
        Ok(())
    }

    fn add_virtual_node(&mut self, vnode: VirtualNodeInfo) -> Result<(), ASABRError> {
        self.check_new_virtual_id(vnode.vid)?;
        self.register_name(vnode.name)?;
        for rid in &vnode.rids {
            self.check_real_id(*rid)?;
            self.rid_to_vnodes_map
                .entry(*rid)
                .or_default()
                .push(vnode.vid);
        }
        self.vertices.push(Vertex::VNode(vnode.vid));
        self.vnode_to_rids_map.insert(vnode.vid, vnode.rids);
        Ok(())
    }

    // Builder
    fn build(self) -> Result<ContactPlan<NM, CM>, ASABRError> {
        self.check_enodes_have_vnodes()?;
        ContactPlan::new(
            self.vertices,
            self.contacts,
            Some(VirtualNodeMap::new(
                self.vnode_to_rids_map,
                self.rid_to_vnodes_map,
            )),
        )
    }
}

pub struct ASABRContactPlan {}

impl ASABRContactPlan {
    // Helper for enode/node duplication
    fn parse_node<NM: NodeManager + DispatchParser<NM> + Parser<NM>>(
        lexer: &mut dyn Lexer,
        node_marker_map: &Option<&StaticMarkerMap<NM>>,
    ) -> Result<Node<NM>, ASABRError> {
        let (info, manager) = parse_components::<NodeInfo, NM>(lexer, *node_marker_map)?;
        let Some(node) = Node::try_new(info, manager) else {
            return Err(ASABRError::ParsingError(format!(
                "Malformed node ({})",
                lexer.get_current_position()
            )));
        };
        Ok(node)
    }

    pub fn parse<
        NM: NodeManager + DispatchParser<NM> + Parser<NM>,
        CM: ContactManager + DispatchParser<CM> + Parser<CM>,
    >(
        lexer: &mut dyn Lexer,
        node_marker_map: Option<&StaticMarkerMap<NM>>,
        contact_marker_map: Option<&StaticMarkerMap<CM>>,
    ) -> Result<ContactPlan<NM, CM>, ASABRError> {
        let mut builder = Builder::new();

        loop {
            let element_type = match lexer.consume_next_token()? {
                LexerOutput::EOF => break,
                LexerOutput::Finished(t) => t,
            };

            match element_type.as_str() {
                "contact" => {
                    let (info, manager) =
                        parse_components::<ContactInfo, CM>(lexer, contact_marker_map)?;
                    let Some(contact) = Contact::try_new(info, manager) else {
                        return Err(ASABRError::ParsingError(format!(
                            "Malformed contact ({})",
                            lexer.get_current_position()
                        )));
                    };
                    builder.add_contact(contact)?;
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
                    let (info, _) =
                        parse_components::<VirtualNodeInfo, NM>(lexer, node_marker_map)?;
                    builder.add_virtual_node(info)?;
                }
                _ => {
                    return Err(ASABRError::ParsingError(format!(
                        "Unrecognized CP element ({})",
                        lexer.get_current_position()
                    )));
                }
            }
        }
        builder.build()
    }
}
