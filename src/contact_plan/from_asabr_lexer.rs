extern crate alloc;
use core::mem;

use alloc::{collections::BTreeMap as HashMap, vec::Vec};

use crate::contact::{ContactInfo, ContactInfoParse};
use crate::node::{NodeInfo, NodeInfoParse};
use crate::node_manager::NodeManager;
use crate::node_manager::none::NoManagement;
use crate::parse_single_tok;
use crate::parsing::{CMDynStandard, EOF, LexFrom, MORON};
use crate::vnode::VNodeInfoParse;
use crate::{
    contact::Contact,
    contact_manager::ContactManager,
    contact_plan::ContactPlan,
    node::Node,
    parsing::Parse,
    types::{NodeID, NodeIDMap},
    vertex::Vertex,
    vnode::{VirtualNodeInfo, VirtualNodeMap},
};

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
}

impl<NM: NodeManager, CM: ContactManager> Builder<NM, CM> {
    fn new() -> Self {
        Self {
            vertices: Vec::new(),
            vnode_to_rids_map: HashMap::new(),
            rid_to_vnodes_map: HashMap::new(),
            contacts: Vec::new(),
        }
    }

    #[inline(always)]
    fn real_nodes_count(&self) -> usize {
        self.vertices.len() - self.vnode_to_rids_map.len()
    }

    // Checkers

    fn check_real_id(&self, id: NodeID) -> Result<(), &'static str> {
        if (id as usize) >= self.real_nodes_count() {
            return Err(
                "Contact tx/rx ids or virtual node rids must match an already declared real node id. ID,node_count:",
            );
        }
        Ok(())
    }

    fn check_new_real_id(&self, id: NodeID) -> Result<(), &'static str> {
        if (id as usize) != self.real_nodes_count() {
            return Err(
                "Declare real nodes before virtual nodes, in increasing id order. ID,node_count:",
            );
        }
        Ok(())
    }

    fn check_new_virtual_id(&self, id: NodeID) -> Result<(), &'static str> {
        if (id as usize) != self.vertices.len() {
            return Err(
                "Declare virtual nodes after the real nodes, in increasing id order. ID,vertice_count:",
            );
        }
        Ok(())
    }

    fn check_enodes_have_vnodes(&self) -> Result<(), &'static str> {
        for vertex in &self.vertices {
            if let Vertex::ENode(enode) = vertex
                && !self.rid_to_vnodes_map.contains_key(&enode.info.id)
            {
                return Err("an ENode is not labeled by any vnode. Id:");
            }
        }
        Ok(())
    }

    // Adders
    fn add_real_node(
        &mut self,
        node: Node<NM>,
        node_type: RealNodeType,
    ) -> Result<(), &'static str> {
        self.check_new_real_id(node.get_node_id())?;
        self.vertices.push(node_type.to_vertex(node));
        Ok(())
    }

    fn add_contact(&mut self, contact: Contact<NM, CM>) -> Result<(), &'static str> {
        self.check_real_id(contact.info.tx_node_id)?;
        self.check_real_id(contact.info.rx_node_id)?;
        self.contacts.push(contact);
        Ok(())
    }

    fn add_virtual_node(&mut self, vnode: VirtualNodeInfo) -> Result<(), &'static str> {
        self.check_new_virtual_id(vnode.vid)?;
        for rid in &vnode.rids {
            self.check_real_id(*rid)?;
            self.rid_to_vnodes_map
                .entry(*rid)
                .or_default()
                .push(vnode.vid);
        }
        self.vertices.push(Vertex::VNode((vnode.name, vnode.vid)));
        self.vnode_to_rids_map.insert(vnode.vid, vnode.rids);
        Ok(())
    }

    // Builder
    fn build(self) -> Result<ContactPlan<NM, CM>, &'static str> {
        self.check_enodes_have_vnodes()?;
        Ok(ContactPlan::new(
            self.vertices,
            self.contacts,
            Some(VirtualNodeMap::new(
                self.vnode_to_rids_map,
                self.rid_to_vnodes_map,
            )),
        ))
    }
}

#[derive(Clone, Copy)]
pub enum ASABRPlanInfoKind {
    Contact,
    Node,
    ENode,
    VNode,
}

parse_single_tok!(ASABRPlanInfoKind, ASABRPlanInfoKind);

impl TryFrom<&str> for ASABRPlanInfoKind {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "contact" => Self::Contact,
            "node" => Self::Node,
            "enode" => Self::ENode,
            "vnode" => Self::VNode,
            _ => return Err(()),
        })
    }
}

#[derive(Default)]
enum InBuild<NM: NodeManager + Parse, CM: ContactManager + Parse> {
    #[default]
    None,
    VNode(<VNodeInfoParse as Parse>::Parser),
    RNode(RealNodeType, <NodeInfoParse as Parse>::Parser),
    NM(RealNodeType, NodeInfo, NM::Parser),
    Contact(<ContactInfoParse as Parse>::Parser),
    CM(ContactInfo, CM::Parser),
}

pub struct ASABRParser<NM: NodeManager + Parse, CM: ContactManager + Parse> {
    builder: Builder<NM, CM>,
    in_build: InBuild<NM, CM>,
}

impl<NM: NodeManager + Parse, CM: ContactManager + Parse> Default for ASABRParser<NM, CM> {
    fn default() -> Self {
        Self {
            builder: Builder::new(),
            in_build: InBuild::None,
        }
    }
}

#[derive(Clone)]
pub enum ASABRTokens<NMTok: Clone, CMTok: Clone> {
    VNode(<VNodeInfoParse as Parse>::Token),
    RNode(<NodeInfoParse as Parse>::Token),
    NM(NMTok),
    CM(CMTok),
    Contact(<ContactInfoParse as Parse>::Token),
    Keywords(ASABRPlanInfoKind),
}

impl<NM: NodeManager + Parse, CM: ContactManager + Parse> Parse for ContactPlan<NM, CM> {
    type Token = ASABRTokens<NM::Token, CM::Token>;
    type Parser = ASABRParser<NM, CM>;

    fn parse(p: Self::Parser) -> Result<Self, &'static str> {
        match p.in_build {
            InBuild::None => p.builder.build(),
            _ => Err(EOF),
        }
    }

    fn feed(tok: Self::Token, parser: &mut Self::Parser) -> Result<bool, &'static str> {
        match (&mut parser.in_build, tok) {
            (InBuild::None, ASABRTokens::Keywords(kind)) => match kind {
                ASABRPlanInfoKind::Contact => {
                    parser.in_build = InBuild::Contact(Default::default())
                }
                ASABRPlanInfoKind::Node => {
                    parser.in_build = InBuild::RNode(RealNodeType::Node, Default::default())
                }
                ASABRPlanInfoKind::ENode => {
                    parser.in_build = InBuild::RNode(RealNodeType::Enode, Default::default())
                }
                ASABRPlanInfoKind::VNode => parser.in_build = InBuild::VNode(Default::default()),
            },

            (InBuild::VNode(sub), ASABRTokens::VNode(tok)) => {
                if VNodeInfoParse::feed(tok, sub)? {
                    let InBuild::VNode(sub) = mem::replace(&mut parser.in_build, InBuild::None)
                    else {
                        unreachable!()
                    };
                    parser
                        .builder
                        .add_virtual_node(VNodeInfoParse::parse(sub)?.into())?;
                }
            }
            (InBuild::RNode(_, sub), ASABRTokens::RNode(tok)) => {
                if NodeInfoParse::feed(tok, sub)? {
                    let InBuild::RNode(ty, sub) = mem::replace(&mut parser.in_build, InBuild::None)
                    else {
                        unreachable!()
                    };

                    if NM::NOFEED {
                        let node = NodeInfoParse::parse(sub)?.into();
                        let manager = NM::parse(Default::default())?;
                        parser.builder.add_real_node(
                            Node::try_new(node, manager).ok_or("Could not build the node")?,
                            ty,
                        )?;
                        parser.in_build = InBuild::None
                    } else {
                        parser.in_build =
                            InBuild::NM(ty, NodeInfoParse::parse(sub)?.into(), Default::default())
                    }
                }
            }
            (InBuild::Contact(sub), ASABRTokens::Contact(tok)) => {
                if ContactInfoParse::feed(tok, sub)? {
                    if CM::NOFEED {
                        let contact = ContactInfoParse::parse(*sub)?.into();
                        let manager = CM::parse(Default::default())?;
                        parser.builder.add_contact(
                            Contact::try_new(contact, manager)
                                .ok_or("Could not build the contact")?,
                        )?;
                        parser.in_build = InBuild::None
                    } else {
                        parser.in_build =
                            InBuild::CM(ContactInfoParse::parse(*sub)?.into(), Default::default());
                    }
                }
            }
            (InBuild::CM(_, sub), ASABRTokens::CM(tok)) => {
                if CM::feed(tok, sub)? {
                    let InBuild::CM(contact, sub) =
                        mem::replace(&mut parser.in_build, InBuild::None)
                    else {
                        unreachable!();
                    };
                    parser.builder.add_contact(
                        Contact::try_new(contact, CM::parse(sub)?)
                            .ok_or("Could not build the contact")?,
                    )?
                }
            }
            (InBuild::NM(_, _, sub), ASABRTokens::NM(tok)) => {
                if NM::feed(tok, sub)? {
                    let InBuild::NM(ty, node, sub) =
                        mem::replace(&mut parser.in_build, InBuild::None)
                    else {
                        unreachable!()
                    };
                    parser.builder.add_real_node(
                        Node::try_new(node, NM::parse(sub)?).ok_or("Could not build the node")?,
                        ty,
                    )?
                }
            }
            _ => return Err(MORON),
        }
        Ok(false)
    }
}

impl<T: ?Sized, NM: NodeManager + Parse, CM: ContactManager + Parse> LexFrom<T>
    for ContactPlan<NM, CM>
where
    ASABRPlanInfoKind: LexFrom<T>,
    VNodeInfoParse: LexFrom<T>,
    NodeInfoParse: LexFrom<T>,
    NM: LexFrom<T>,
    ContactInfoParse: LexFrom<T>,
    CM: LexFrom<T>,
{
    fn lex(t: &T, p: &Self::Parser) -> Result<Self::Token, &'static str> {
        Ok(match &p.in_build {
            InBuild::None => ASABRTokens::Keywords(ASABRPlanInfoKind::lex(t, &None)?),
            InBuild::VNode(p) => ASABRTokens::VNode(VNodeInfoParse::lex(t, p)?),
            InBuild::RNode(_, p) => ASABRTokens::RNode(NodeInfoParse::lex(t, p)?),
            InBuild::NM(_, _, p) => ASABRTokens::NM(NM::lex(t, p)?),
            InBuild::Contact(p) => ASABRTokens::Contact(ContactInfoParse::lex(t, p)?),
            InBuild::CM(_, p) => ASABRTokens::CM(CM::lex(t, p)?),
        })
    }
}

assert_impl_all! {
    ContactPlan<NoManagement,CMDynStandard>: Parse,
    LexFrom<str>
}
