#![allow(clippy::type_complexity)]

extern crate alloc;

use alloc::{vec, vec::Vec};
use core::{
    fmt::Display,
    hint::cold_path,
    iter,
    ops::{Index, IndexMut, Range},
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

#[derive(Debug)]
struct Neigbhoors<'id> {
    reals: Vec<(INodeRef<'id>, Range<usize>)>,
    virtuals: Vec<(VNodeRef<'id>, Vec<(RealNodeRef<'id>, usize)>)>,
}

/// Represents a multigraph structure, where each node can have multiple connections.
#[derive(Debug)]
pub struct Multigraph<'id, NM: NodeManager, CM: ContactManager> {
    // TODO: better contact management.
    /// The list of node objects.
    internal_nodes: Vec<(Node<NM>, Neigbhoors<'id>)>,
    virtual_nodes: Vec<Vec<RealNodeRef<'id>>>,
    external_nodes: Vec<Node<NM>>,
    contacts: Vec<Contact<CM>>,
    /// ZST graph id
    id: Id<'id>,
}

/// A reference to a real node in the graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RealNodeRef<'id> {
    /// External node reference.
    E(ENodeRef<'id>),
    /// Internal node reference.
    I(INodeRef<'id>),
}

/// A routable node (internal or virtual)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RoutableNodeRef<'id> {
    /// Internal node reference.
    I(INodeRef<'id>),
    /// Virtual node reference.
    V(VNodeRef<'id>),
}

/// A reference to an external real node in a graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ENodeRef<'id> {
    index: usize,
    id: Id<'id>,
}

/// A reference to an internal real node in a graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct INodeRef<'id> {
    index: usize,
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

/// A reference to any node in the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeRef<'id> {
    /// Internal node reference.
    I(INodeRef<'id>),
    /// Virtual node reference.
    V(VNodeRef<'id>),
    /// External node reference.
    E(ENodeRef<'id>),
}

/// A reference to a contact in the graph with the same id
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContactRef<'id> {
    index: usize,
    /// ZST graph id
    id: Id<'id>,
}

impl<'id> Neigbhoors<'id> {
    fn new() -> Self {
        Self {
            reals: Vec::new(),
            virtuals: Vec::new(),
        }
    }
}

impl<'id, NM: NodeManager, CM: ContactManager> Multigraph<'id, NM, CM> {
    /// Creates a new `Multigraph` from a contact plan.
    ///
    /// Note: For Dijkstra, we need fast access for the senders. To this end, the index
    /// in the "senders" Vec matches the  transmitter NodeID. There is a small memory
    /// overhead if some nodes are not transmitters in the contacts. Regarding the
    /// receivers, only fast iteration is required. The indices of the `senders[tx_id].receivers`
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
            internal_nodes: Vec::with_capacity(realnodes.len()),
            external_nodes: Vec::with_capacity(realnodes.len()),
            contacts: Vec::with_capacity(realnodes.len().next_power_of_two()),
            virtual_nodes: Vec::with_capacity(vnodes.len()),
            id,
        };

        // add inode and enode
        for node in realnodes {
            match node {
                RealNode::Inode(node) => r.internal_nodes.push((node, Neigbhoors::new())),
                RealNode::Enode(node) => {
                    r.external_nodes.push(node);
                }
            }
        }
        r.internal_nodes.shrink_to_fit();
        r.external_nodes.shrink_to_fit();

        let mut node_vnode_members = vec![Vec::new(); r.get_nonvirtualnode_count()];
        let rnode_count = r.get_nonvirtualnode_count();

        // add vnode
        for vnode in vnodes {
            let new_id = r.virtual_nodes.len();
            let new = r
                .virtual_nodes
                .push_mut(Vec::with_capacity(vnode.rids.len()));
            for rid in vnode.rids {
                let rid = usize::from(rid);
                if rid >= rnode_count {
                    cold_path();
                    return Err(ASABRError::ContactPlanError("illegal node id"));
                }
                if rid >= r.internal_nodes.len() {
                    node_vnode_members[rid].push(new_id);
                    let rid = rid - r.internal_nodes.len();
                    // ENODE
                    new.push(RealNodeRef::E(ENodeRef { index: rid, id }));
                } else {
                    node_vnode_members[rid].push(new_id);
                    new.push(RealNodeRef::I(INodeRef { index: rid, id }));
                }
            }
        }

        // sort contacts by (tx,rx,start,end)
        contacts.sort_unstable_by_key(|contact| {
            (
                contact.1,
                contact.2,
                contact.0.lifespan.start,
                contact.0.lifespan.end,
            )
        });

        let contact_groups = contacts.into_iter().chunk_by(|ct| (ct.1, ct.2));

        for ((rx, tx), ct_g) in contact_groups.into_iter() {
            if rx >= r.get_internal_count() || tx >= r.get_nonvirtualnode_count() {
                return Err(ASABRError::ContactPlanError("illegal node id for contact"));
            }

            let start = r.contacts.len();
            r.contacts.extend(ct_g.map(|ct| ct.0));
            let end = r.contacts.len();

            let tx_ref;

            if tx >= r.get_internal_count() {
                // ENODE
                tx_ref = RealNodeRef::E(ENodeRef {
                    index: tx - r.get_internal_count(),
                    id,
                });
            } else {
                // INODE
                tx_ref = RealNodeRef::I(INodeRef { index: tx, id });

                r.internal_nodes[rx]
                    .1
                    .reals
                    .push((INodeRef { index: tx, id }, start..end));
            }

            let virtuals_neigh = &mut r.internal_nodes[rx].1.virtuals;
            {
                for vnode in node_vnode_members[tx].iter().copied() {
                    match virtuals_neigh.iter_mut().find(|neig| neig.0.index == vnode) {
                        Some(neig) => neig.1.extend((start..end).map(|idx| (tx_ref, idx))),
                        None => {
                            let neig = virtuals_neigh
                                .push_mut((VNodeRef { index: vnode, id }, Vec::new()));
                            neig.1.extend((start..end).map(|idx| (tx_ref, idx)))
                        }
                    }
                }
            }
        }

        for node in &mut r.internal_nodes {
            for vneig in &mut node.1.virtuals {
                vneig
                    .1
                    .sort_unstable_by_key(|elt| r.contacts[elt.1].lifespan);
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

    /// Returns the graph node reference for a node ID.
    pub fn node_id_ref(&self, id: NodeID) -> Result<NodeRef<'id>, ASABRError> {
        let mut id = usize::from(id);
        if id < self.get_internal_count() {
            Ok(NodeRef::I(INodeRef {
                index: id,
                id: self.id,
            }))
        } else {
            id -= self.get_internal_count();
            if id < self.get_external_count() {
                Ok(NodeRef::E(ENodeRef {
                    index: id,
                    id: self.id,
                }))
            } else {
                id -= self.get_external_count();
                if id < self.get_vnode_count() {
                    Ok(NodeRef::V(VNodeRef {
                        index: id,
                        id: self.id,
                    }))
                } else {
                    Err(ASABRError::ContactPlanError("illegal node id"))
                }
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
    pub fn mark_excluded(&mut self, exclusions: &[RealNodeRef<'id>]) {
        for (node, _) in self.internal_nodes.iter_mut() {
            node.info.excluded = false;
        }
        for node in self.external_nodes.iter_mut() {
            node.info.excluded = false
        }
        for node in exclusions {
            self[node].info.excluded = true;
        }
    }

    /// Retrieves the total number of vertices in the multigraph (rnode + enode + vnode).
    pub fn get_vertex_count(&self) -> usize {
        self.internal_nodes.len() + self.virtual_nodes.len() + self.external_nodes.len()
    }

    /// Retrieve the total number of real node in the multigraph (enode + node)
    pub fn get_internal_count(&self) -> usize {
        self.internal_nodes.len()
    }

    /// Retrieve the number of vnode in the multigraph
    pub fn get_vnode_count(&self) -> usize {
        self.virtual_nodes.len()
    }

    /// Retrieves a copy of the Id<'id>
    pub fn id(&self) -> Id<'id> {
        self.id
    }

    /// Retrieve the number of routable elements (aka rnodes + vnodes, but not enodes)
    pub fn get_routable_count(&self) -> usize {
        self.get_internal_count() + self.get_vnode_count()
    }

    /// Convert a NodeID into the usize index to use in routing operation
    pub fn flatten_route_id(&self, id: NodeID) -> Result<usize, ASABRError> {
        self.node_id_ref(id)?
            .routable()
            .map(|node| self.routable_to_usize(node))
    }

    /// return a stable index between 0 and self.get_routable_count
    pub fn into_nodeid(&self, node: NodeRef<'id>) -> NodeID {
        match node {
            NodeRef::I(inoderef) => usize::from(inoderef).into(),
            NodeRef::E(enoderef) => (usize::from(enoderef) + self.get_internal_count()).into(),
            NodeRef::V(vnode_ref) => {
                (usize::from(vnode_ref) + self.get_nonvirtualnode_count()).into()
            }
        }
    }

    /// Converts a routable node reference into the flattened routing index.
    pub fn routable_to_usize(&self, node: RoutableNodeRef<'id>) -> usize {
        match node {
            RoutableNodeRef::I(inode_ref) => inode_ref.into(),
            RoutableNodeRef::V(vnode_ref) => self.get_internal_count() + usize::from(vnode_ref),
        }
    }

    /// Returns the graph node ID for a virtual node reference.
    pub fn vnode_id(&self, vnode: VNodeRef) -> NodeID {
        (self.get_vertex_count() - self.get_vnode_count() + vnode.index).into()
    }
    /// Returns the number of real nodes.
    pub fn get_nonvirtualnode_count(&self) -> usize {
        self.get_internal_count() + self.get_external_count()
    }

    /// Returns the number of external nodes.
    pub fn get_external_count(&self) -> usize {
        self.external_nodes.len()
    }

    /// Returns an iterator over the real nodes represented by a virtual node.
    pub fn iter_virtualnode(&self, node: VNodeRef<'id>) -> impl Iterator<Item = RealNodeRef<'id>> {
        self.virtual_nodes[node.index].iter().copied()
    }
    /// Returns an iterator over the real nodes represented by any node reference.
    pub fn iter_node(&self, node: NodeRef<'id>) -> impl Iterator<Item = RealNodeRef<'id>> {
        match node {
            NodeRef::E(enode_ref) => Either::Left(iter::once(RealNodeRef::E(enode_ref))),
            NodeRef::I(inode_ref) => Either::Left(iter::once(RealNodeRef::I(inode_ref))),
            NodeRef::V(vnode_ref) => Either::Right(self.iter_virtualnode(vnode_ref)),
        }
    }

    /// For a given node, return a three element tuple containing:
    /// - the node manager
    /// - real nodes neigbors, as an iterator over `(ref of a neighbor, manager of the neighbor, iterator over contacts between the two)`
    /// - vnode neigbors, as an iterator over (ref of a neighbor,iterator(real_node,it's manager,contact))
    pub fn iter_iter_contacts(
        &self,
        noderef: INodeRef<'id>,
        _prune_time: Option<Date>, //TODO: prune old contacts
    ) -> (
        &Node<NM>,
        impl Iterator<
            Item = (
                INodeRef<'id>,
                &Node<NM>,
                impl Iterator<Item = (ContactRef<'id>, &Contact<CM>)>,
            ),
        >,
        impl Iterator<
            Item = (
                VNodeRef<'id>,
                impl Iterator<Item = (RealNodeRef<'id>, &Node<NM>, ContactRef<'id>, &Contact<CM>)>,
            ),
        >,
    ) {
        let (node, neigbhours) = &self.internal_nodes[noderef.index];
        let neighboor_reals = neigbhours.reals.iter().map(|(neig, contacts)| {
            (
                *neig,
                &self.internal_nodes[neig.index].0,
                contacts.clone().map(|idx| {
                    (
                        ContactRef {
                            index: idx,
                            id: self.id,
                        },
                        &self.contacts[idx],
                    )
                }),
            )
        });
        let neighboor_virt = neigbhours.virtuals.iter().map(|(vnode, contacts)| {
            (
                *vnode,
                contacts.iter().map(|(rnode, ct)| {
                    (
                        *rnode,
                        &self[*rnode],
                        ContactRef {
                            index: *ct,
                            id: self.id,
                        },
                        &self.contacts[*ct],
                    )
                }),
            )
        });
        // TODO: Fill the vnode iterator
        (node, neighboor_reals, neighboor_virt)
    }

    ///for a given inode pair, iter on contacts between the two
    pub fn iter_contacts(
        &self,
        tx: INodeRef<'id>,
        rx: INodeRef<'id>,
    ) -> impl Iterator<Item = (ContactRef<'id>, &Contact<CM>)> {
        let id = self.id;
        let arr = &self.internal_nodes[usize::from(tx)].1.reals;
        match arr.binary_search_by_key(&rx, |elt| elt.0) {
            Ok(index) => Either::Left(
                arr[index]
                    .1
                    .clone()
                    .map(move |index| (ContactRef { index, id }, &self.contacts[index])),
            ),
            Err(_) => Either::Right(iter::empty()),
        }
    }
    /// Iterates mutably over contacts between two internal nodes.
    pub fn iter_contacts_mut(
        &mut self,
        tx: INodeRef<'id>,
        rx: INodeRef<'id>,
    ) -> impl Iterator<Item = (ContactRef<'id>, &mut Contact<CM>)> {
        let arr = &self.internal_nodes[usize::from(tx)].1.reals;
        match arr.binary_search_by_key(&rx, |elt| elt.0) {
            Ok(index) => Either::Left({
                let id = self.id;
                let range = arr[index].1.clone();
                range
                    .clone()
                    .map(move |index| ContactRef { index, id })
                    .zip(self.contacts[range].iter_mut())
            }),
            Err(_) => Either::Right(iter::empty()),
        }
    }
}

macro_rules! graph_index {
    ($s:ident,$i:ident,$T:ty => $($tt:tt)*) => {
        impl<'id, NM:NodeManager, CM:ContactManager> Index<$T> for Multigraph<'id,NM,CM> {
            type Output = Node<NM>;
            fn index($s:&Self,$i: $T) -> &Self::Output {
                & $($tt)*
            }
        }
        impl<'id,NM:NodeManager, CM:ContactManager> IndexMut<$T> for Multigraph<'id,NM,CM> {
            fn index_mut($s:&mut Self,$i: $T) -> &mut Node<NM> {
                &mut $($tt)*
            }
        }

    }
}

graph_index!(self,index,INodeRef<'id> => self.internal_nodes[index.index].0);
graph_index!(self,index,ENodeRef<'id> => self.external_nodes[index.index]);

impl<'id, NM: NodeManager, CM: ContactManager> Index<RealNodeRef<'id>> for Multigraph<'id, NM, CM> {
    type Output = Node<NM>;
    fn index(&self, index: RealNodeRef<'id>) -> &Self::Output {
        match index {
            RealNodeRef::E(enode_ref) => &self[enode_ref],
            RealNodeRef::I(inode_ref) => &self[inode_ref],
        }
    }
}
impl<'id, NM: NodeManager, CM: ContactManager> IndexMut<RealNodeRef<'id>>
    for Multigraph<'id, NM, CM>
{
    fn index_mut(&mut self, index: RealNodeRef<'id>) -> &mut Self::Output {
        match index {
            RealNodeRef::E(enode_ref) => &mut self[enode_ref],
            RealNodeRef::I(inode_ref) => &mut self[inode_ref],
        }
    }
}

impl<'id, NM: NodeManager, CM: ContactManager> Index<ContactRef<'id>> for Multigraph<'id, NM, CM> {
    type Output = Contact<CM>;
    fn index(&self, index: ContactRef<'id>) -> &Self::Output {
        &self.contacts[index.index]
    }
}
impl<'id, NM: NodeManager, CM: ContactManager> IndexMut<ContactRef<'id>>
    for Multigraph<'id, NM, CM>
{
    fn index_mut(&mut self, index: ContactRef<'id>) -> &mut Self::Output {
        &mut self.contacts[index.index]
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

impl<'id> From<INodeRef<'id>> for NodeID {
    fn from(value: INodeRef<'id>) -> NodeID {
        value.index.into()
    }
}

impl<'id> TryFrom<NodeRef<'id>> for INodeRef<'id> {
    type Error = ASABRError;
    fn try_from(value: NodeRef<'id>) -> Result<Self, Self::Error> {
        match value {
            NodeRef::I(rnode_ref) => Ok(rnode_ref),
            _ => Err(ASABRError::ContactPlanError("This is not a inode")),
        }
    }
}
impl<'id> TryFrom<NodeRef<'id>> for VNodeRef<'id> {
    type Error = ASABRError;
    fn try_from(value: NodeRef<'id>) -> Result<Self, Self::Error> {
        match value {
            NodeRef::V(vnode_ref) => Ok(vnode_ref),
            _ => Err(ASABRError::ContactPlanError("This is not a vnode")),
        }
    }
}

impl Display for RealNodeRef<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        NodeRef::from(*self).fmt(f)
    }
}

impl<'id, NM: NodeManager, CM: ContactManager> Display for Multigraph<'id, NM, CM> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(
            f,
            "Multigraph: {} vertices ({} inode(s), {} enode(s) {} vnode(s))",
            self.get_vertex_count(),
            self.get_internal_count(),
            self.get_external_count(),
            self.get_vnode_count(),
        )?;

        writeln!(f, "Vnodes:")?;
        for vnode in self.virtual_nodes.iter().enumerate() {
            write!(f, "id: {}, rids: [", vnode.0 + self.get_routable_count())?;
            for rid in vnode.1 {
                write!(f, "{}, ", rid)?;
            }
            writeln!(f, "]")?;
        }

        writeln!(f, "\nIodes:")?;
        for rnode in self.internal_nodes.iter().enumerate() {
            writeln!(f, "id: {}", rnode.0)?;
            for ctg in &rnode.1.1.reals {
                writeln!(f, " -> node {} ", ctg.0.index)?;
                for ct in &self.contacts[ctg.1.clone()] {
                    writeln!(f, "  - Contact during {} ", ct.lifespan)?;
                }
            }
            for ctg in &rnode.1.1.virtuals {
                writeln!(f, " -> vnode {} ", ctg.0.index + self.get_routable_count())?;
                for ct in &ctg.1 {
                    writeln!(
                        f,
                        "  - Contact by {} during {} ",
                        ct.0, self.contacts[ct.1].lifespan
                    )?;
                }
            }
        }

        Ok(())
    }
}

impl<'id> NodeRef<'id> {
    /// Returns this reference as a real node reference, if it is real.
    pub fn real(self) -> Option<RealNodeRef<'id>> {
        match self {
            NodeRef::I(inode_ref) => Some(RealNodeRef::I(inode_ref)),
            NodeRef::E(enode_ref) => Some(RealNodeRef::E(enode_ref)),
            NodeRef::V(_vnode_ref) => None,
        }
    }
    /// Returns this reference as a virtual node reference, if it is virtual.
    pub fn virt(self) -> Option<VNodeRef<'id>> {
        match self {
            NodeRef::V(vnode_ref) => Some(vnode_ref),
            _ => None,
        }
    }
    /// Returns this reference as a routable node reference, if it is routable.
    pub fn routable(self) -> Result<RoutableNodeRef<'id>, ASABRError> {
        match self {
            NodeRef::I(inode_ref) => Ok(RoutableNodeRef::I(inode_ref)),
            NodeRef::V(vnode_ref) => Ok(RoutableNodeRef::V(vnode_ref)),
            NodeRef::E(_enode_ref) => Err(ASABRError::ContactPlanError(
                "This is a enode, it is not routable",
            )),
        }
    }
    /// Returns this reference as an internal node reference, if it is internal.
    pub fn internal(self) -> Option<INodeRef<'id>> {
        match self {
            NodeRef::I(inoderef) => Some(inoderef),
            _ => None,
        }
    }
    /// Returns this reference as an external node reference, if it is external.
    pub fn external(self) -> Option<ENodeRef<'id>> {
        match self {
            NodeRef::E(enoderef) => Some(enoderef),
            _ => None,
        }
    }
}

impl<'id> RealNodeRef<'id> {
    /// Returns this reference as an internal node reference, if it is internal.
    pub fn internal(self) -> Option<INodeRef<'id>> {
        match self {
            RealNodeRef::E(_enode_ref) => None,
            RealNodeRef::I(inode_ref) => Some(inode_ref),
        }
    }
    /// Returns this reference as an external node reference, if it is external.
    pub fn external(self) -> Option<ENodeRef<'id>> {
        match self {
            RealNodeRef::E(enode_ref) => Some(enode_ref),
            RealNodeRef::I(_inode_ref) => None,
        }
    }
}

impl Display for NodeRef<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NodeRef::I(inode_ref) => write!(f, "enode: {}", inode_ref.index),
            NodeRef::E(enode_ref) => write!(f, "enode: {}", enode_ref.index),
            NodeRef::V(vnode_ref) => write!(f, "vnode: {}", vnode_ref.index),
        }
    }
}

impl<'id> From<RealNodeRef<'id>> for NodeRef<'id> {
    fn from(value: RealNodeRef<'id>) -> Self {
        match value {
            RealNodeRef::E(node) => NodeRef::E(node),
            RealNodeRef::I(node) => NodeRef::I(node),
        }
    }
}
impl<'id> From<RoutableNodeRef<'id>> for NodeRef<'id> {
    fn from(value: RoutableNodeRef<'id>) -> Self {
        match value {
            RoutableNodeRef::I(inode_ref) => NodeRef::I(inode_ref),
            RoutableNodeRef::V(vnode_ref) => NodeRef::V(vnode_ref),
        }
    }
}
impl<'id> From<INodeRef<'id>> for NodeRef<'id> {
    fn from(value: INodeRef<'id>) -> Self {
        NodeRef::I(value)
    }
}
impl<'id> From<ENodeRef<'id>> for NodeRef<'id> {
    fn from(value: ENodeRef<'id>) -> Self {
        NodeRef::E(value)
    }
}
impl<'id> From<VNodeRef<'id>> for NodeRef<'id> {
    fn from(value: VNodeRef<'id>) -> Self {
        Self::V(value)
    }
}

impl From<VNodeRef<'_>> for usize {
    fn from(value: VNodeRef) -> Self {
        value.index
    }
}

impl From<INodeRef<'_>> for usize {
    fn from(value: INodeRef) -> Self {
        value.index
    }
}
impl From<ENodeRef<'_>> for usize {
    fn from(value: ENodeRef) -> Self {
        value.index
    }
}
impl<'id> From<INodeRef<'id>> for RealNodeRef<'id> {
    fn from(value: INodeRef<'id>) -> Self {
        RealNodeRef::I(value)
    }
}
impl<'id> From<ENodeRef<'id>> for RealNodeRef<'id> {
    fn from(value: ENodeRef<'id>) -> Self {
        RealNodeRef::E(value)
    }
}

impl<'id> From<INodeRef<'id>> for RoutableNodeRef<'id> {
    fn from(value: INodeRef<'id>) -> Self {
        RoutableNodeRef::I(value)
    }
}
impl<'id> From<VNodeRef<'id>> for RoutableNodeRef<'id> {
    fn from(value: VNodeRef<'id>) -> Self {
        Self::V(value)
    }
}
