extern crate alloc;
use crate::multigraph::{NodeRef, RNodeRef, VNodeRef};
use alloc::{boxed::Box, rc::Rc};

pub trait Destination<'id> {
    /// A new pathfinding begin, reinit to a state of no reachable nodes
    fn reinit(&mut self);
    /// This node have been poped from disktra prio_queue, should we stop ?
    fn now_reached(&mut self, node: NodeRef<'id>) -> bool;
    /// Should paths to this vnode be considered ?
    fn is_useful(&self, node: VNodeRef<'id>) -> bool;
}

pub enum Dest<'id> {
    RNode(RNodeRef<'id>),
    VNode(VNodeRef<'id>),
    AllNodes(),
    AnyCast(Rc<[RNodeRef<'id>]>),
    MultiCast(Rc<[RNodeRef<'id>]>, Box<[bool]>, usize),
}

impl<'id> Destination<'id> for Dest<'id> {
    fn reinit(&mut self) {
        match self {
            Self::MultiCast(_, reached, counter) => {
                for r in reached.iter_mut() {
                    *r = false
                }
                *counter = 0
            }
            _ => (),
        }
    }

    fn now_reached(&mut self, node: NodeRef<'id>) -> bool {
        match (self, node) {
            (Self::RNode(dest), NodeRef::R(node)) => *dest == node,
            (Self::VNode(_), NodeRef::V(_)) => true, // because the correct vnode is the only one accepted
            (Self::AllNodes(), _) => false,
            (Self::AnyCast(dests), NodeRef::R(node)) => dests.binary_search(&node).is_ok(),
            (Self::MultiCast(dests, reached, counter), NodeRef::R(node)) => {
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
}

impl<'id> From<RNodeRef<'id>> for Dest<'id> {
    fn from(value: RNodeRef<'id>) -> Self {
        Self::RNode(value)
    }
}
impl<'id> From<VNodeRef<'id>> for Dest<'id> {
    fn from(value: VNodeRef<'id>) -> Self {
        Self::VNode(value)
    }
}
impl<'id> From<NodeRef<'id>> for Dest<'id> {
    fn from(value: NodeRef<'id>) -> Self {
        match value {
            NodeRef::R(rnode_ref) => rnode_ref.into(),
            NodeRef::V(vnode_ref) => vnode_ref.into(),
        }
    }
}
impl<'id> From<All> for Dest<'id> {
    fn from(_value: All) -> Self {
        Self::AllNodes()
    }
}
impl<'id> Dest<'id> {
    pub fn anycast(casts: Rc<[RNodeRef<'id>]>) -> Self {
        Self::AnyCast(casts)
    }
    pub fn multicast(casts: Rc<[RNodeRef<'id>]>) -> Self {
        let bools = unsafe { Box::new_zeroed_slice(casts.len()).assume_init() };
        Self::MultiCast(casts, bools, 0)
    }
}

impl<'id> Destination<'id> for RNodeRef<'id> {
    #[inline(always)]
    fn reinit(&mut self) {}

    #[inline(always)]
    fn now_reached(&mut self, node: NodeRef<'id>) -> bool {
        node == NodeRef::R(*self)
    }

    #[inline(always)]
    fn is_useful(&self, _node: VNodeRef<'id>) -> bool {
        false
    }
}

impl<'id> Destination<'id> for VNodeRef<'id> {
    #[inline(always)]
    fn reinit(&mut self) {}

    #[inline(always)]
    fn now_reached(&mut self, node: NodeRef<'id>) -> bool {
        node == NodeRef::V(*self)
    }

    #[inline(always)]
    fn is_useful(&self, node: VNodeRef<'id>) -> bool {
        node == *self
    }
}

pub struct All;

impl Destination<'_> for All {
    #[inline(always)]
    fn reinit(&mut self) {}

    #[inline(always)]
    fn now_reached(&mut self, _node: NodeRef<'_>) -> bool {
        false
    }

    #[inline(always)]
    fn is_useful(&self, _node: VNodeRef<'_>) -> bool {
        true
    }
}
