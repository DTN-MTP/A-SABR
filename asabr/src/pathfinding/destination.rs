extern crate alloc;

use crate::{
    bundle::Bundle,
    contact_manager::{ContactManager, ContactManagerTxData},
    errors::ASABRError,
    multigraph::{INodeRef, Multigraph, NodeRef, RoutableNodeRef, VNodeRef},
    node_manager::{NodeManager, none::NoManagement},
    parsing::Either,
    pathfinding::{PathFindingOutput, PathIterator, Pathfinding},
    paths::PathFragment,
    types::Date,
};
use alloc::{boxed::Box, rc::Rc, vec, vec::Vec};
use itertools::Itertools;

/// A Destination you can search paths for
pub trait FindableDest<'id, NM: NodeManager, CM: ContactManager> {
    /// A new pathfinding begin, reinit to a state of no reachable nodes
    fn reinit(&mut self);
    /// This node have been poped from disktra prio_queue, should we stop ?
    fn now_reached(&mut self, node: RoutableNodeRef<'id>) -> bool;
    /// Should paths to this vnode be considered ?
    fn is_useful(&self, node: VNodeRef<'id>) -> bool;
    /// Wether this path tree is still valid/usefull to pass a bundle
    /// This will be the pathfinding output so you may as well update the path times while your at it
    /// # Safety
    /// self paths should have no cycle (wich is true of any reasonable PathfindingOutput not modified by hand outside of the library)
    unsafe fn validate(
        &self,
        paths: &mut PathFindingOutput<'id, '_>,
        time: Date,
        bundle: &Bundle,
        graph: &Multigraph<'id, NM, CM>,
    ) -> bool;
    /// Some pathfinder can provide performance improvement if this return Some
    /// Returning the same id for two different destination may however prevent the pathfinder from finding the best path (or a path at all)
    fn to_id(&self, graph: &Multigraph<'id, NM, CM>) -> Option<usize>;
}

/// A destination which can automatically handle complete routing
pub trait RoutableDest<'id, NM: NodeManager, CM: ContactManager>:
    FindableDest<'id, NM, CM>
{
    type RoutingOutput<'a>
    where
        'id: 'a;
    fn route<'a>(
        &mut self,
        graph: &mut Multigraph<'id, NM, CM>,
        bundle: &Bundle,
        finder: &'a mut (impl Pathfinding<'id, NM, CM, Self> + ?Sized),
        routing_time: Date,
        source: INodeRef<'id>,
        prune_time: Option<i64>,
    ) -> Result<Option<Self::RoutingOutput<'a>>, ASABRError>;
}

/// Standard destination selector.
pub enum Dest<'id> {
    /// A single internal node destination.
    INode(INodeRef<'id>),
    /// A single virtual node destination.
    VNode(VNodeRef<'id>),
    /// All routable nodes are destinations.
    AllNodes(),
    /// Stop after reaching any one internal node.
    AnyCast(Rc<[INodeRef<'id>]>),
    /// Stop after reaching all listed internal nodes.
    MultiCast(Rc<[INodeRef<'id>]>, Box<[bool]>, usize),
}

pub fn classical_route<'id, 'a>(
    path: Option<PathFindingOutput<'id, 'a>>,
    dest: impl Into<RoutableNodeRef<'id>> + Copy,
    bundle: &Bundle,
    graph: &mut Multigraph<'id, impl NodeManager, impl ContactManager>,
) -> Result<
    Option<(
        PathIterator<'id, 'a, PathFindingOutput<'id, 'a>>,
        PathFragment<'id>,
    )>,
    ASABRError,
> {
    let Some(path) = path else {
        return Ok(None);
    };
    let last = path.commit_path_to(dest.into(), bundle, graph)?;
    let path = path.full_path_rev_owned(dest.into(), graph);
    Ok(path.zip(last))
}

impl<'id, CM: ContactManager> FindableDest<'id, NoManagement, CM> for Dest<'id> {
    fn reinit(&mut self) {
        if let Self::MultiCast(_, reached, counter) = self {
            for r in reached.iter_mut() {
                *r = false
            }
            *counter = 0
        }
    }

    fn now_reached(&mut self, node: RoutableNodeRef<'id>) -> bool {
        match (self, node) {
            (Self::INode(dest), RoutableNodeRef::I(node)) => *dest == node,
            (Self::VNode(_), RoutableNodeRef::V(_)) => true, // because the correct vnode is the only one accepted
            (Self::AllNodes(), _) => false,
            (Self::AnyCast(dests), RoutableNodeRef::I(node)) => dests.binary_search(&node).is_ok(),
            (Self::MultiCast(dests, reached, counter), RoutableNodeRef::I(node)) => {
                if let Ok(idx) = dests.binary_search(&node) {
                    if !reached[idx] {
                        reached[idx] = true;
                        *counter += 1;
                        *counter == dests.len()
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false,
        }
    }
    fn is_useful(&self, node: VNodeRef<'id>) -> bool {
        match self {
            Self::VNode(dest) => *dest == node,
            Self::AllNodes() => true,
            _ => false,
        }
    }

    unsafe fn validate(
        &self,
        paths: &mut PathFindingOutput<'id, '_>,
        time: Date,
        bundle: &Bundle,
        graph: &Multigraph<'id, NoManagement, CM>,
    ) -> bool {
        unsafe {
            match self {
                Dest::INode(rnode_ref) => {
                    paths.validate(RoutableNodeRef::I(*rnode_ref), time, bundle, graph)
                }
                Dest::VNode(vnode_ref) => {
                    paths.validate(RoutableNodeRef::V(*vnode_ref), time, bundle, graph)
                }
                Dest::AllNodes() => All::validate(&All, paths, time, bundle, graph),
                Dest::AnyCast(rnode_refs) => rnode_refs
                    .iter()
                    .any(|dest| paths.validate(RoutableNodeRef::I(*dest), time, bundle, graph)),
                Dest::MultiCast(rnode_refs, _items, _) => rnode_refs
                    .iter()
                    // This path is not technically fully valid, but hey, it is still interesting, so we want to extract it
                    .all(|dest| paths.validate(RoutableNodeRef::I(*dest), time, bundle, graph)),
            }
        }
    }

    fn to_id(&self, graph: &Multigraph<'id, NoManagement, CM>) -> Option<usize> {
        match self {
            Dest::INode(rnode_ref) => Some((*rnode_ref).into()),
            Dest::VNode(vnode_ref) => Some(graph.routable_to_usize((*vnode_ref).into())),
            Dest::AllNodes() => None,
            Dest::AnyCast(_rnode_refs) => None,
            Dest::MultiCast(..) => None,
        }
    }
}

impl<'id, CM: ContactManager> RoutableDest<'id, NoManagement, CM> for Dest<'id> {
    type RoutingOutput<'a>
        = Either<
        (
            PathIterator<'id, 'a, PathFindingOutput<'id, 'a>>,
            PathFragment<'id>,
        ),
        (Vec<(usize, Vec<INodeRef<'id>>)>, PathFindingOutput<'id, 'a>),
    >
    where
        'id: 'a;
    fn route<'a>(
        &mut self,
        graph: &mut Multigraph<'id, NoManagement, CM>,
        bundle: &Bundle,
        finder: &'a mut (impl Pathfinding<'id, NoManagement, CM, Self> + ?Sized),
        routing_time: Date,
        source: INodeRef<'id>,
        prune_time: Option<i64>,
    ) -> Result<Option<Self::RoutingOutput<'a>>, ASABRError> {
        let path = finder.find_path(graph, routing_time, source, bundle, self, prune_time)?;
        match self {
            Dest::INode(inode_ref) => {
                classical_route(path, *inode_ref, bundle, graph).map(|o| o.map(Either::Left))
            }
            Dest::VNode(vnode_ref) => {
                classical_route(path, *vnode_ref, bundle, graph).map(|o| o.map(Either::Left))
            }
            Dest::AllNodes() => todo!(),
            Dest::AnyCast(_inode_refs) => todo!(),
            Dest::MultiCast(inode_refs, _, counter) => {
                if path.is_none() || *counter == 0 {
                    return Ok(None);
                }
                let mut pathtree = path.unwrap().into_owned();
                let mut collect = vec![None; graph.get_internal_count()].into_boxed_slice();
                for node in inode_refs.iter() {
                    let mut next: usize = (*node).into();
                    while let Some(PathFragment {
                        via,
                        recv: arrival_time,
                        rx_node,
                        ..
                    }) = pathtree[next]
                    {
                        let opt = &mut collect[usize::from(rx_node.internal().unwrap())];
                        if opt.is_none_or(|(time, _old)| time > arrival_time.end) {
                            *opt = Some((arrival_time.end, next));
                            match via {
                                Some(via) => next = via.parent_frag,
                                None => break,
                            }
                        } else {
                            break;
                        }
                    }
                }

                let mut collect2 = Vec::with_capacity(inode_refs.len());
                for node in inode_refs.iter() {
                    let mut next = usize::from(*node);
                    let last = loop {
                        let (date, effective) = collect[next].unwrap();
                        if date == Date::MIN {
                            break effective;
                        }
                        let mut new = pathtree[effective].unwrap();
                        if let Some(via) = &mut new.via {
                            via.parent_frag = pathtree[via.parent_frag]
                                .unwrap()
                                .rx_node
                                .internal()
                                .unwrap()
                                .into();
                            next = via.parent_frag;
                            graph[via.contact].schedule_tx(
                                ContactManagerTxData {
                                    send: via.send,
                                    recv: new.recv,
                                },
                                bundle,
                            )?;
                            if pathtree[via.parent_frag].unwrap().via.is_none() {
                                break effective;
                            }
                        }
                        pathtree[next] = Some(new);
                    };
                    collect2.push((*node, last));

                    next = usize::from(*node);
                    while let Some((date, effective)) = &mut collect[next] {
                        if *date == Date::MIN {
                            break;
                        }
                        *date = Date::MIN;
                        if let Some(via) = pathtree[*effective].unwrap().via {
                            next = via.parent_frag;
                            *effective = last;
                        } else {
                            break;
                        }
                    }
                }

                if let PathFindingOutput {
                    path_tree: Either::Right(vec),
                } = &mut pathtree
                {
                    vec.truncate(graph.get_routable_count());
                }
                collect2.sort_unstable_by_key(|elt| elt.1);

                let iter = collect2.into_iter().chunk_by(|elt| elt.1);
                let final_co: Vec<_> = iter
                    .into_iter()
                    .map(|(key, group)| (key, group.map(|elt| elt.0).collect()))
                    .collect();
                Ok(Some(Either::Right((final_co, pathtree))))
            }
        }
    }
}

impl<'id> From<INodeRef<'id>> for Dest<'id> {
    fn from(value: INodeRef<'id>) -> Self {
        Self::INode(value)
    }
}
impl<'id> From<VNodeRef<'id>> for Dest<'id> {
    fn from(value: VNodeRef<'id>) -> Self {
        Self::VNode(value)
    }
}
impl<'id> TryFrom<NodeRef<'id>> for Dest<'id> {
    type Error = ASABRError;
    fn try_from(value: NodeRef<'id>) -> Result<Self, ASABRError> {
        match value {
            NodeRef::I(rnode_ref) => Ok(rnode_ref.into()),
            NodeRef::V(vnode_ref) => Ok(vnode_ref.into()),
            NodeRef::E(_) => Err(ASABRError::ContactPlanError("This node is not routable")),
        }
    }
}
impl<'id> From<RoutableNodeRef<'id>> for Dest<'id> {
    fn from(value: RoutableNodeRef<'id>) -> Self {
        match value {
            RoutableNodeRef::I(rnode_ref) => rnode_ref.into(),
            RoutableNodeRef::V(vnode_ref) => vnode_ref.into(),
        }
    }
}
impl<'id> From<All> for Dest<'id> {
    fn from(_value: All) -> Self {
        Self::AllNodes()
    }
}
impl<'id> Dest<'id> {
    /// Creates an anycast destination from sorted internal node references.
    pub fn anycast(casts: Rc<[INodeRef<'id>]>) -> Self {
        Self::AnyCast(casts)
    }
    /// Creates a multicast destination from sorted internal node references.
    pub fn multicast(casts: Rc<[INodeRef<'id>]>) -> Self {
        let bools = unsafe { Box::new_zeroed_slice(casts.len()).assume_init() };
        Self::MultiCast(casts, bools, 0)
    }
}

impl<'id, NM: NodeManager, CM: ContactManager> FindableDest<'id, NM, CM> for INodeRef<'id> {
    #[inline(always)]
    fn reinit(&mut self) {}

    #[inline(always)]
    fn now_reached(&mut self, node: RoutableNodeRef<'id>) -> bool {
        node == RoutableNodeRef::I(*self)
    }

    #[inline(always)]
    fn is_useful(&self, _node: VNodeRef<'id>) -> bool {
        false
    }

    unsafe fn validate(
        &self,
        paths: &mut PathFindingOutput<'id, '_>,
        time: Date,
        bundle: &Bundle,
        graph: &Multigraph<'id, NM, CM>,
    ) -> bool {
        unsafe { paths.validate(RoutableNodeRef::I(*self), time, bundle, graph) }
    }

    fn to_id(&self, _graph: &Multigraph<'id, NM, CM>) -> Option<usize> {
        Some((*self).into())
    }
}

impl<'id, NM: NodeManager, CM: ContactManager> RoutableDest<'id, NM, CM> for INodeRef<'id> {
    type RoutingOutput<'a>
        = (
        PathIterator<'id, 'a, PathFindingOutput<'id, 'a>>,
        PathFragment<'id>,
    )
    where
        'id: 'a;
    fn route<'a>(
        &mut self,
        graph: &mut Multigraph<'id, NM, CM>,
        bundle: &Bundle,
        finder: &'a mut (impl Pathfinding<'id, NM, CM, Self> + ?Sized),
        routing_time: Date,
        source: INodeRef<'id>,
        prune_time: Option<i64>,
    ) -> Result<Option<Self::RoutingOutput<'a>>, ASABRError> {
        let path = finder.find_path(graph, routing_time, source, bundle, self, prune_time)?;
        classical_route(path, *self, bundle, graph)
    }
}

impl<'id, NM: NodeManager, CM: ContactManager> FindableDest<'id, NM, CM> for VNodeRef<'id> {
    #[inline(always)]
    fn reinit(&mut self) {}

    #[inline(always)]
    fn now_reached(&mut self, node: RoutableNodeRef<'id>) -> bool {
        node == RoutableNodeRef::V(*self)
    }

    #[inline(always)]
    fn is_useful(&self, node: VNodeRef<'id>) -> bool {
        node == *self
    }

    unsafe fn validate(
        &self,
        paths: &mut PathFindingOutput<'id, '_>,
        time: Date,
        bundle: &Bundle,
        graph: &Multigraph<'id, NM, CM>,
    ) -> bool {
        unsafe { paths.validate(RoutableNodeRef::V(*self), time, bundle, graph) }
    }

    fn to_id(&self, _graph: &Multigraph<'id, NM, CM>) -> Option<usize> {
        Some((*self).into())
    }
}

impl<'id, NM: NodeManager, CM: ContactManager> RoutableDest<'id, NM, CM> for VNodeRef<'id> {
    type RoutingOutput<'a>
        = (
        PathIterator<'id, 'a, PathFindingOutput<'id, 'a>>,
        PathFragment<'id>,
    )
    where
        'id: 'a;
    fn route<'a>(
        &mut self,
        graph: &mut Multigraph<'id, NM, CM>,
        bundle: &Bundle,
        finder: &'a mut (impl Pathfinding<'id, NM, CM, Self> + ?Sized),
        routing_time: Date,
        source: INodeRef<'id>,
        prune_time: Option<i64>,
    ) -> Result<Option<Self::RoutingOutput<'a>>, ASABRError> {
        let path = finder.find_path(graph, routing_time, source, bundle, self, prune_time)?;
        classical_route(path, *self, bundle, graph)
    }
}

impl<'id, NM: NodeManager, CM: ContactManager> FindableDest<'id, NM, CM> for RoutableNodeRef<'id> {
    #[inline(always)]
    fn reinit(&mut self) {}

    #[inline(always)]
    fn now_reached(&mut self, node: RoutableNodeRef<'id>) -> bool {
        node == *self
    }

    #[inline(always)]
    fn is_useful(&self, node: VNodeRef<'id>) -> bool {
        match self {
            RoutableNodeRef::I(inode_ref) => {
                <INodeRef as FindableDest<NM, CM>>::is_useful(inode_ref, node)
            }
            RoutableNodeRef::V(vnode_ref) => {
                <VNodeRef as FindableDest<NM, CM>>::is_useful(vnode_ref, node)
            }
        }
    }
    unsafe fn validate(
        &self,
        paths: &mut PathFindingOutput<'id, '_>,
        time: Date,
        bundle: &Bundle,
        graph: &Multigraph<'id, NM, CM>,
    ) -> bool {
        unsafe { paths.validate(*self, time, bundle, graph) }
    }

    fn to_id(&self, graph: &Multigraph<'id, NM, CM>) -> Option<usize> {
        Some(graph.routable_to_usize(*self))
    }
}
impl<'id, NM: NodeManager, CM: ContactManager> RoutableDest<'id, NM, CM> for RoutableNodeRef<'id> {
    type RoutingOutput<'a>
        = (
        PathIterator<'id, 'a, PathFindingOutput<'id, 'a>>,
        PathFragment<'id>,
    )
    where
        'id: 'a;
    fn route<'a>(
        &mut self,
        graph: &mut Multigraph<'id, NM, CM>,
        bundle: &Bundle,
        finder: &'a mut (impl Pathfinding<'id, NM, CM, Self> + ?Sized),
        routing_time: Date,
        source: INodeRef<'id>,
        prune_time: Option<i64>,
    ) -> Result<Option<Self::RoutingOutput<'a>>, ASABRError> {
        let path = finder.find_path(graph, routing_time, source, bundle, self, prune_time)?;
        classical_route(path, *self, bundle, graph)
    }
}

/// Destination that keeps searching all useful routable nodes.
pub struct All;

impl<'id, NM: NodeManager, CM: ContactManager> FindableDest<'id, NM, CM> for All {
    #[inline(always)]
    fn reinit(&mut self) {}

    #[inline(always)]
    fn now_reached(&mut self, _node: RoutableNodeRef<'_>) -> bool {
        false
    }

    #[inline(always)]
    fn is_useful(&self, _node: VNodeRef<'_>) -> bool {
        true
    }
    unsafe fn validate(
        &self,
        _paths: &mut PathFindingOutput<'_, '_>,
        _time: Date,
        _bundle: &Bundle,
        _graph: &Multigraph<'_, NM, CM>,
    ) -> bool {
        true
    }

    fn to_id(&self, _graph: &Multigraph<'_, NM, CM>) -> Option<usize> {
        None
    }
}
impl<'id, CM: ContactManager> RoutableDest<'id, NoManagement, CM> for All {
    type RoutingOutput<'a>
        = PathFindingOutput<'id, 'a>
    where
        'id: 'a;
    fn route<'a>(
        &mut self,
        graph: &mut Multigraph<'id, NoManagement, CM>,
        bundle: &Bundle,
        finder: &'a mut (impl Pathfinding<'id, NoManagement, CM, Self> + ?Sized),
        routing_time: Date,
        source: INodeRef<'id>,
        prune_time: Option<i64>,
    ) -> Result<Option<Self::RoutingOutput<'a>>, ASABRError> {
        let _ = finder.find_path(graph, routing_time, source, bundle, self, prune_time);
        Ok(None)
    }
}
