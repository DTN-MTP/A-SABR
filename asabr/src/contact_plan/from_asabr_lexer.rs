extern crate alloc;
use core::mem;

use alloc::vec::Vec;

use crate::contact::ContactInfo;
use crate::contact_plan::RealNode;
use crate::node::NodeInfo;
use crate::node_manager::NodeManager;
use crate::node_manager::none::NoManagement;
use crate::parse_single_tok;
use crate::parsing::{CMDynStandard, EOF, INVALID_STATE, LexFrom};
use crate::{
    contact::Contact, contact_manager::ContactManager, contact_plan::ContactPlan, node::Node,
    parsing::Parse, types::NodeID, vnode::VirtualNodeInfo,
};

enum RealNodeType {
    Node,
    Enode,
}

impl RealNodeType {
    fn to_rnode<NM: NodeManager>(&self, node: Node<NM>) -> RealNode<NM> {
        match self {
            RealNodeType::Node => RealNode::Inode(node),
            RealNodeType::Enode => RealNode::Enode(node),
        }
    }
}

struct Builder<NM: NodeManager, CM: ContactManager> {
    rnodes: Vec<RealNode<NM>>,
    vnodes: Vec<VirtualNodeInfo>,
    contacts: Vec<(Contact<CM>, usize, usize)>,
}

impl<NM: NodeManager, CM: ContactManager> Builder<NM, CM> {
    fn new() -> Self {
        Self {
            rnodes: Vec::new(),
            vnodes: Vec::new(),
            contacts: Vec::new(),
        }
    }

    #[inline(always)]
    fn real_nodes_count(&self) -> usize {
        self.rnodes.len()
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
        if (id as usize) != self.rnodes.len() + self.vnodes.len() {
            return Err(
                "Declare virtual nodes after the real nodes, in increasing id order. ID,vertice_count:",
            );
        }
        Ok(())
    }

    fn check_enodes_have_vnodes(&self) -> Result<(), &'static str> {
        for vertex in &self.rnodes {
            if let RealNode::Enode(enode) = vertex
                && !enode.info.excluded
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
        self.rnodes.push(node_type.to_rnode(node));
        Ok(())
    }

    fn add_contact(&mut self, contact: (Contact<CM>, usize, usize)) -> Result<(), &'static str> {
        self.check_real_id(contact.1 as u16)?;
        self.check_real_id(contact.2 as u16)?;
        self.contacts.push(contact);
        Ok(())
    }

    fn add_virtual_node(&mut self, vnode: VirtualNodeInfo) -> Result<(), &'static str> {
        self.check_new_virtual_id(vnode.vid)?;
        for rid in &vnode.rids {
            self.check_real_id(*rid)?;
            if let RealNode::Enode(node) = &mut self.rnodes[*rid as usize] {
                node.info.excluded = true
            }
        }
        self.vnodes.push(vnode);
        Ok(())
    }

    // Builder
    fn build(mut self) -> Result<ContactPlan<NM, CM>, &'static str> {
        self.check_enodes_have_vnodes()?;
        for node in self.rnodes.iter_mut() {
            if let RealNode::Enode(node) = node {
                node.info.excluded = false
            }
        }
        Ok(ContactPlan::new(self.rnodes, self.vnodes, self.contacts))
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
    VNode(<VirtualNodeInfo as Parse>::Parser),
    RNode(RealNodeType, <NodeInfo as Parse>::Parser),
    NM(RealNodeType, NodeInfo, NM::Parser),
    Contact(<ContactInfo as Parse>::Parser),
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
    VNode(<VirtualNodeInfo as Parse>::Token),
    RNode(<NodeInfo as Parse>::Token),
    NM(NMTok),
    CM(CMTok),
    Contact(<ContactInfo as Parse>::Token),
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
                if VirtualNodeInfo::feed(tok, sub)? {
                    let InBuild::VNode(sub) = mem::replace(&mut parser.in_build, InBuild::None)
                    else {
                        unreachable!()
                    };
                    parser
                        .builder
                        .add_virtual_node(VirtualNodeInfo::parse(sub)?)?;
                }
            }
            (InBuild::RNode(_, sub), ASABRTokens::RNode(tok)) => {
                if NodeInfo::feed(tok, sub)? {
                    let InBuild::RNode(ty, sub) = mem::replace(&mut parser.in_build, InBuild::None)
                    else {
                        unreachable!()
                    };

                    if NM::NOFEED {
                        let node = NodeInfo::parse(sub)?;
                        let manager = NM::parse(Default::default())?;
                        parser.builder.add_real_node(
                            Node::try_new(node, manager).ok_or("Could not build the node")?,
                            ty,
                        )?;
                        parser.in_build = InBuild::None
                    } else {
                        parser.in_build = InBuild::NM(ty, NodeInfo::parse(sub)?, Default::default())
                    }
                }
            }
            (InBuild::Contact(sub), ASABRTokens::Contact(tok)) => {
                if ContactInfo::feed(tok, sub)? {
                    if CM::NOFEED {
                        let contact = ContactInfo::parse(*sub)?;
                        let manager = CM::parse(Default::default())?;
                        parser.builder.add_contact(
                            Contact::try_new(contact, manager)
                                .ok_or("Could not build the contact")?,
                        )?;
                        parser.in_build = InBuild::None
                    } else {
                        parser.in_build =
                            InBuild::CM(ContactInfo::parse(*sub)?, Default::default());
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
            _ => return Err(INVALID_STATE),
        }
        Ok(false)
    }
}

impl<T: ?Sized, NM: NodeManager + Parse, CM: ContactManager + Parse> LexFrom<T>
    for ContactPlan<NM, CM>
where
    ASABRPlanInfoKind: LexFrom<T>,
    VirtualNodeInfo: LexFrom<T>,
    NodeInfo: LexFrom<T>,
    NM: LexFrom<T>,
    ContactInfo: LexFrom<T>,
    CM: LexFrom<T>,
{
    fn lex(t: &T, p: &Self::Parser) -> Result<Self::Token, &'static str> {
        Ok(match &p.in_build {
            InBuild::None => ASABRTokens::Keywords(ASABRPlanInfoKind::lex(t, &None)?),
            InBuild::VNode(p) => ASABRTokens::VNode(VirtualNodeInfo::lex(t, p)?),
            InBuild::RNode(_, p) => ASABRTokens::RNode(NodeInfo::lex(t, p)?),
            InBuild::NM(_, _, p) => ASABRTokens::NM(NM::lex(t, p)?),
            InBuild::Contact(p) => ASABRTokens::Contact(ContactInfo::lex(t, p)?),
            InBuild::CM(_, p) => ASABRTokens::CM(CM::lex(t, p)?),
        })
    }
}

assert_impl_all! {
    ContactPlan<NoManagement,CMDynStandard>: Parse,
    LexFrom<str>
}
