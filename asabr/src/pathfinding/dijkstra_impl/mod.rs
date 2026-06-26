extern crate alloc;
use alloc::vec;
use alloc::vec::Vec;

mod contact_parenting;
mod hybrid_parenting;
mod node_parenting;
pub use contact_parenting::ContactParenting;
pub use hybrid_parenting::{HybridParenting, HybridParentingOrd};
pub use node_parenting::NodeParenting;

use crate::{parsing::Either, pathfinding::PathFindingOutput, paths::PathFragment};

pub fn flatten<'id, 'a>(
    paths: &[PathFragment<'id>],
    by_dest: impl Iterator<Item = Option<usize>>,
) -> PathFindingOutput<'id, 'a> {
    let mut elided_tree = Vec::new();
    let mut new_indexs = vec![None; paths.len()];
    for (i, possible_path) in by_dest.enumerate() {
        let path = possible_path.map(|index| {
            new_indexs[index] = Some(i);
            paths[index]
        });
        elided_tree.push(path);
    }

    for i in 0..elided_tree.len() {
        if let Some(mut frag) = elided_tree[i] {
            let mut index = i;
            loop {
                if let Some(via) = frag.via.as_mut() {
                    if let Some(new_idx) = new_indexs[via.parent_frag] {
                        via.parent_frag = new_idx;
                        elided_tree[index] = Some(frag);
                        break;
                    } else {
                        let old_idx = via.parent_frag;
                        let new_idx = elided_tree.len();
                        elided_tree.push(None);
                        via.parent_frag = new_idx;
                        frag = paths[old_idx];
                        index = new_idx;
                    }
                } else {
                    elided_tree[index] = Some(frag);
                    break;
                }
            }
        } else {
            continue;
        }
    }

    PathFindingOutput {
        path_tree: Either::Right(elided_tree),
    }
}
