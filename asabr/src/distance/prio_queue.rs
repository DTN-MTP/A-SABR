extern crate alloc;

use core::{cmp::Ordering, hint::cold_path, marker::PhantomData};

use alloc::vec::Vec;

use crate::{
    bundle::Bundle, contact_manager::ContactManager, distance::Distance, multigraph::Multigraph,
    node_manager::NodeManager, paths::PathFragment,
};

/// A custom if fairly classical implementation of a priority queue using a binary heap, allowing to pass a reference to the graph in order to compare elements
/// This is a min priority queue respective to the distance D
pub struct PrioQueue<'id, D: Distance<NM, CM>, NM: NodeManager, CM: ContactManager, T: Copy> {
    /// Triplet pathfragment, node reached by it, custom aditional data
    elts: Vec<(PathFragment<'id>, T)>,
    _phantom: PhantomData<fn(&'id (), D, NM, CM)>,
}

fn parent(i: usize) -> Option<usize> {
    if i == 0 { None } else { Some((i - 1) / 2) }
}
fn left_child(i: usize) -> usize {
    2 * i + 1
}
fn right_child(i: usize) -> usize {
    2 * i + 2
}

impl<'id, D: Distance<NM, CM>, NM: NodeManager, CM: ContactManager, T: Copy>
    PrioQueue<'id, D, NM, CM, T>
{
    /// Create a new empty priority queue
    pub fn new() -> Self {
        Self {
            elts: Vec::new(),
            _phantom: PhantomData,
        }
    }
    /// Create a new priority queue reserving enough space for capacity elements
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            elts: Vec::with_capacity(capacity),
            _phantom: PhantomData,
        }
    }
    fn left_child(&self, i: usize) -> Option<usize> {
        let child = left_child(i);
        if child >= self.elts.len() {
            None
        } else {
            Some(child)
        }
    }
    fn right_child(&self, i: usize) -> Option<usize> {
        let child = right_child(i);
        if child >= self.elts.len() {
            None
        } else {
            Some(child)
        }
    }
    /// Insert an element in the priority queue, sorting it by the distance D, requiring a reference to the graph to do the comparison.
    /// It is obviously a logic error to change the multigraph in a way wich change the distance while having a live queue
    pub fn insert(
        &mut self,
        elt: (PathFragment<'id>, T),
        graph: &Multigraph<'id, NM, CM>,
        bundle: &Bundle,
    ) {
        let mut id = self.elts.len();
        self.elts.push(elt);
        while let Some(parent) = parent(id)
            && D::cmp(&self.elts[parent].0, &elt.0, graph, bundle) == Ordering::Greater
        {
            self.elts[id] = self.elts[parent];
            id = parent;
        }
        self.elts[id] = elt;
    }
    /// Check the minimum element
    pub fn peek_min(&self) -> Option<&(PathFragment<'id>, T)> {
        self.elts.get(0)
    }
    /// Pop the minimum element, returning it. Reorganize the queue according to the distance D, requiring a reference to the graph to do the comparison.
    /// It is obviously a logic error to change the multigraph in a way wich change the distance while having a live queue
    pub fn pop_min(
        &mut self,
        graph: &Multigraph<'id, NM, CM>,
        bundle: &Bundle,
    ) -> Option<(PathFragment<'id>, T)> {
        if self.elts.len() == 0 {
            cold_path();

            return None;
        } else {
            let ret = self.elts[0];
            let fst = self.elts.pop().unwrap();
            if !self.elts.is_empty() {
                let mut id = 0;
                loop {
                    match (self.left_child(id), self.right_child(id)) {
                        (None, None) => {
                            self.elts[id] = fst;
                            break;
                        }
                        (Some(left), None) => {
                            if D::cmp(&self.elts[left].0, &fst.0, graph, bundle) == Ordering::Less {
                                self.elts[id] = self.elts[left];
                                self.elts[left] = fst;
                            } else {
                                self.elts[id] = fst;
                            }
                            break;
                        }
                        (Some(left), Some(right)) => {
                            let min =
                                if D::cmp(&self.elts[left].0, &self.elts[right].0, graph, bundle)
                                    == Ordering::Less
                                {
                                    left
                                } else {
                                    right
                                };
                            if D::cmp(&self.elts[min].0, &fst.0, graph, bundle) == Ordering::Less {
                                self.elts[id] = self.elts[min];
                                id = min
                            } else {
                                self.elts[id] = fst;
                                break;
                            }
                        }
                        (None, Some(_)) => unreachable!(),
                    }
                }
            }

            Some(ret)
        }
    }
    pub fn is_empty(&self) -> bool {
        self.elts.is_empty()
    }
}

// TODO: test
