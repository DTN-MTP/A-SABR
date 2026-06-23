extern crate alloc;

use alloc::vec::Vec;
use core::{
    fmt::Display,
    iter,
    ops::{Index, IndexMut},
};
use generativity::{Guard, Id};
use itertools::Itertools;

use super::node::Node;
use crate::contact_manager::ContactManager;
use crate::contact_plan::{ContactPlan, RealNode};
use crate::errors::ASABRError;
use crate::node_manager::NodeManager;
use crate::types::*;
use crate::{contact::Contact, parsing::Either};

/// Represents a multigraph structure, where each node can have multiple connections.
#[derive(Debug)]
pub struct Multigraph<'id, NM: NodeManager, CM: ContactManager> {
    // TODO: better contact management.
    /// The list of node objects.
    real_nodes: Vec<(Node<NM>, Vec<(RNodeRef<'id>, Vec<Contact<CM>>)>)>,
    virtual_nodes: Vec<Vec<RNodeRef<'id>>>,
    /// ZST graph id
    id: Id<'id>,
}

/// A reference to a real node in the graph with the same id
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RNodeRef<'id> {
    /// The node index in the graph vector
    index: usize,
    /// ZST graph id
    id: Id<'id>,
}

/// A reference to a virtual node in the graph with the same id
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VNodeRef<'id> {
    /// The node index in the graph vector
    index: usize,
    /// ZST graph id
    id: Id<'id>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeRef<'id> {
    R(RNodeRef<'id>),
    V(VNodeRef<'id>),
}

/// A reference to a contact in the graph with the same id
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContactRef<'id> {
    node: RNodeRef<'id>,
    index: (usize, usize),
    /// ZST graph id
    id: Id<'id>,
}

impl<'id, NM: NodeManager, CM: ContactManager> Multigraph<'id, NM, CM> {
    /// Creates a new `Multigraph` from a contact plan.
    ///
    /// Note: For Dijkstra, we need fast access for the senders. To this end, the index
    /// in the "senders" Vec matches the  transmitter NodeID. There is a small memory
    /// overhead if some nodes are not transmitters in the contacts. Regarding the
    /// receivers, only fast iteration is required. The indices of the senders[tx_id].receivers
    /// Vec do not match the receivers NodeID, and no entry exists if a node never receives.
    pub fn new(
        id_guard: Guard<'id>,
        ContactPlan {
            realnodes,
            vnodes,
            mut contacts,
        }: ContactPlan<NM, CM>,
    ) -> Result<Self, ASABRError> {
        let id = id_guard.into();

        let mut r = Self {
            real_nodes: Vec::with_capacity(realnodes.len()),
            virtual_nodes: Vec::with_capacity(vnodes.len()),
            id: id,
        };

        for node in realnodes {
            match node {
                RealNode::Enode(node) | RealNode::Inode(node) => {
                    r.real_nodes.push((node, Vec::new()))
                }
            }
        }
        for vnode in vnodes {
            let new = r
                .virtual_nodes
                .push_mut(Vec::with_capacity(vnode.rids.len()));
            for rid in vnode.rids {
                if rid as usize >= r.real_nodes.len() {
                    return Err(ASABRError::ContactPlanError("illegal node id"));
                }
                new.push(RNodeRef {
                    index: rid.into(),
                    id,
                });
            }
        }

        contacts.sort_unstable_by_key(|contact| (contact.1, contact.2));

        let contact_groups = contacts.into_iter().chunk_by(|ct| (ct.1, ct.2));

        for ((rx, tx), ct_g) in contact_groups.into_iter() {
            if rx >= r.real_nodes.len() || tx >= r.real_nodes.len() {
                return Err(ASABRError::ContactPlanError("illegal node id"));
            }

            let new = &mut r.real_nodes[rx]
                .1
                .push_mut((RNodeRef { index: tx, id }, Vec::new()))
                .1;

            for ct in ct_g {
                new.push(ct.0);
            }
        }

        Ok(r)
    }

    /// The unsafe version of new that does not require a Guard, and produce a Multigraph for any given lifetime, including 'static.
    /// This method is intended to make it easy to store / return multigraphs, and/or pass them to C.
    /// # Safety
    /// Using this method make it your responsability to associate all other taged information with the correct graph instead of relying on the compiler.
    /// Using any of the tagged structures (ContactRef and NodeRef, Pathfinding implementation ...) with an incorrect graph
    /// can result in UB or Panic even while using the safe interface.
    ///
    /// Note that, if you do not 'id in any way, the compiler will not check for either of these:
    ///  - Using structures associated with this unsafely constructed graph with a safely constructed graph
    ///  - Using structures associated with a safely constructed graph with this unsafely constructed one
    ///  - Using structures associated with another unsafely constructed graph with this one
    ///
    /// It is guaranteed that restricting 'id to be 'static make the first two cases impossible, but the last one is still your responsability to avoid.
    /// For more fine restriction, check the generativity crate
    pub unsafe fn new_unguarded(contact_plan: ContactPlan<NM, CM>) -> Result<Self, ASABRError> {
        let guard = unsafe { Guard::<'id>::new(Id::new()) };
        Self::new(guard, contact_plan)
    }

    pub fn node_id_ref(&self, id: NodeID) -> Result<NodeRef<'id>, ASABRError> {
        let mut id = id as usize;
        if id < self.real_nodes.len() {
            Ok(NodeRef::R(RNodeRef {
                index: id,
                id: self.id,
            }))
        } else {
            id -= self.real_nodes.len();
            if id < self.virtual_nodes.len() {
                Ok(NodeRef::V(VNodeRef {
                    index: id,
                    id: self.id,
                }))
            } else {
                Err(ASABRError::ContactPlanError("illegal node id"))
            }
        }
    }

    /// Applies exclusions to the nodes based on the provided sorted exclusions.
    ///
    /// Marks nodes as excluded if their index is in the `exclusions` list, otherwise unmarks them.
    ///
    /// # Parameters
    ///
    /// * `exclusions: &[NodeID]` - A sorted list of node IDs to exclude.
    pub fn mark_excluded(&mut self, exclusions: &[RNodeRef<'id>]) {
        for (node, _) in self.real_nodes.iter_mut() {
            node.info.excluded = false;
        }
        for node in exclusions {
            self[node].info.excluded = true;
        }
    }

    /// Retrieves the total number of vertices in the multigraph (rnode + vnode).
    pub fn get_vertex_count(&self) -> usize {
        self.real_nodes.len() + self.virtual_nodes.len()
    }

    /// Retrieve the total number of real node in the multigraph (enode + node)
    pub fn get_rnode_count(&self) -> usize {
        self.real_nodes.len()
    }

    /// Retrieves a copy of the Id<'id>
    pub fn id(&self) -> Id<'id> {
        self.id
    }

    pub fn iter_contacts_withnodes(
        &self,
        tx: RNodeRef<'id>,
        rx: RNodeRef<'id>,
    ) -> impl Iterator<Item = (ContactRef<'id>, &Contact<CM>, &Node<NM>, &Node<NM>)> {
        let txcontacts = &self.real_nodes[tx.index].1;
        let iter = txcontacts
            .iter()
            .enumerate()
            .filter(move |(_, (id, _))| *id == rx);
        iter.flat_map(move |(id, (_, ve))| {
            (0..ve.len()).map(move |i| {
                (
                    ContactRef {
                        node: tx,
                        index: (id, i),
                        id: self.id,
                    },
                    &ve[i],
                    &self.real_nodes[tx.index].0,
                    &self.real_nodes[rx.index].0,
                )
            })
        })
    }

    pub fn iter_contacts(
        &self,
        tx: RNodeRef<'id>,
        rx: RNodeRef<'id>,
    ) -> impl Iterator<Item = ContactRef<'id>> {
        let txcontacts = &self.real_nodes[tx.index].1;
        let iter = txcontacts
            .iter()
            .enumerate()
            .filter(move |(_, (id, _))| *id == rx);
        iter.flat_map(move |(id, (_, ve))| {
            (0..ve.len()).map(move |i| ContactRef {
                node: tx,
                index: (id, i),
                id: self.id,
            })
        })
    }
    pub fn iter_contacts_mut(
        &mut self,
        tx: RNodeRef<'id>,
        rx: RNodeRef<'id>,
    ) -> impl Iterator<Item = (ContactRef<'id>, &mut Contact<CM>)> {
        let gid = self.id;
        let txcontacts = &mut self.real_nodes[tx.index].1;
        let iter = txcontacts
            .iter_mut()
            .enumerate()
            .filter(move |(_, (id, _))| *id == rx);
        iter.flat_map(move |(id, (_, ve))| {
            ve.iter_mut().enumerate().map(move |(i, ct)| {
                (
                    ContactRef {
                        node: tx,
                        index: (id, i),
                        id: gid,
                    },
                    ct,
                )
            })
        })
    }
    pub fn iter_virtualnode(&self, node: VNodeRef<'id>) -> impl Iterator<Item = RNodeRef<'id>> {
        self.virtual_nodes[node.index].iter().copied()
    }
    pub fn iter_node(&self, node: NodeRef<'id>) -> impl Iterator<Item = RNodeRef<'id>> {
        match node {
            NodeRef::R(rnode_ref) => Either::Left(iter::once(rnode_ref)),
            NodeRef::V(vnode_ref) => Either::Right(self.iter_virtualnode(vnode_ref)),
        }
    }
    pub fn iter_iter_contacts(
        &self,
        noderef: RNodeRef<'id>,
    ) -> (
        &Node<NM>,
        impl Iterator<
            Item = (
                RNodeRef<'id>,
                &Node<NM>,
                impl Iterator<Item = (ContactRef<'id>, &Contact<CM>)>,
            ),
        >,
    ) {
        let id = self.id;
        let (node, neigbhours) = &self.real_nodes[noderef.index];
        let neighboor_iter = neigbhours
            .iter()
            .enumerate()
            .map(move |(outer, (neig, contacts))| {
                (
                    *neig,
                    &self.real_nodes[neig.index].0,
                    contacts.iter().enumerate().map(move |(inner, contact)| {
                        (
                            ContactRef {
                                node: noderef,
                                index: (outer, inner),
                                id,
                            },
                            contact,
                        )
                    }),
                )
            });
        (node, neighboor_iter)
    }
}

impl<'id, NM: NodeManager, CM: ContactManager> Index<RNodeRef<'id>> for Multigraph<'id, NM, CM> {
    type Output = Node<NM>;
    fn index(&self, index: RNodeRef<'id>) -> &Self::Output {
        &self.real_nodes[index.index].0
    }
}
impl<'id, NM: NodeManager, CM: ContactManager> IndexMut<RNodeRef<'id>> for Multigraph<'id, NM, CM> {
    fn index_mut(&mut self, index: RNodeRef<'id>) -> &mut Self::Output {
        &mut self.real_nodes[index.index].0
    }
}

impl<'id, NM: NodeManager, CM: ContactManager> Index<VNodeRef<'id>> for Multigraph<'id, NM, CM> {
    type Output = [RNodeRef<'id>];
    fn index(&self, index: VNodeRef<'id>) -> &Self::Output {
        self.virtual_nodes[index.index].as_slice()
    }
}
impl<'id, NM: NodeManager, CM: ContactManager> IndexMut<VNodeRef<'id>> for Multigraph<'id, NM, CM> {
    fn index_mut(&mut self, index: VNodeRef<'id>) -> &mut Self::Output {
        self.virtual_nodes[index.index].as_mut_slice()
    }
}

impl<'id, NM: NodeManager, CM: ContactManager> Index<ContactRef<'id>> for Multigraph<'id, NM, CM> {
    type Output = Contact<CM>;
    fn index(&self, index: ContactRef<'id>) -> &Self::Output {
        &self.real_nodes[index.node.index].1[index.index.0].1[index.index.1]
    }
}
impl<'id, NM: NodeManager, CM: ContactManager> IndexMut<ContactRef<'id>>
    for Multigraph<'id, NM, CM>
{
    fn index_mut(&mut self, index: ContactRef<'id>) -> &mut Self::Output {
        &mut self.real_nodes[index.node.index].1[index.index.0].1[index.index.1]
    }
}

impl<'id, NM: NodeManager, CM: ContactManager, I> Index<&I> for Multigraph<'id, NM, CM>
where
    I: Copy,
    Multigraph<'id, NM, CM>: Index<I>,
{
    type Output = <Multigraph<'id, NM, CM> as Index<I>>::Output;
    fn index(&self, index: &I) -> &Self::Output {
        &self[*index]
    }
}

impl<'id, NM: NodeManager, CM: ContactManager, I> IndexMut<&I> for Multigraph<'id, NM, CM>
where
    I: Copy,
    Multigraph<'id, NM, CM>: IndexMut<I>,
{
    fn index_mut(&mut self, index: &I) -> &mut Self::Output {
        &mut self[*index]
    }
}

impl<'id> From<RNodeRef<'id>> for NodeID {
    fn from(value: RNodeRef<'id>) -> NodeID {
        value.index as NodeID
    }
}

impl<'id, NM: NodeManager, CM: ContactManager> Display for Multigraph<'id, NM, CM> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(
            f,
            "Multigraph: {} vertices ({} real node(s), {} vnode(s))",
            self.real_nodes.len() + self.virtual_nodes.len(),
            self.real_nodes.len(),
            self.virtual_nodes.len(),
        )?;

        writeln!(f, "Vnodes:")?;
        for vnode in self.virtual_nodes.iter().enumerate() {
            write!(f, "id: {}, rids: [", vnode.0 + self.real_nodes.len())?;
            for rid in vnode.1 {
                write!(f, "{}, ", rid.index)?;
            }
            writeln!(f, "]")?;
        }

        writeln!(f, "\nNodes:")?;
        for rnode in self.real_nodes.iter().enumerate() {
            writeln!(f, "id: {}", rnode.0)?;
            for ctg in &rnode.1.1 {
                writeln!(f, " -> node {} ", ctg.0.index)?;
                for ct in &ctg.1 {
                    writeln!(f, "  - Contact during {} ", ct.lifespan)?;
                }
            }
        }

        Ok(())
    }
}

impl<'id> NodeRef<'id> {
    pub fn real(self) -> Option<RNodeRef<'id>> {
        match self {
            NodeRef::R(rnode_ref) => Some(rnode_ref),
            NodeRef::V(_vnode_ref) => None,
        }
    }
    pub fn virt(self) -> Option<VNodeRef<'id>> {
        match self {
            NodeRef::R(_rnode_ref) => None,
            NodeRef::V(vnode_ref) => Some(vnode_ref),
        }
    }
}
