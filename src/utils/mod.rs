extern crate alloc;
use alloc::rc::Rc;
use core::cell::RefCell;

use crate::{
    contact_manager::ContactManager, contact_plan::asabr_file_lexer::parse_from_iter,
    errors::ASABRError, multigraph::Multigraph, node_manager::NodeManager, parsing::LexFrom,
    pathfinding::Pathfinding,
};

pub fn init_pathfinding<
    NM: NodeManager + LexFrom<str>,
    CM: ContactManager + LexFrom<str>,
    P: Pathfinding<NM, CM>,
    D: AsRef<str>,
    I: Iterator<Item = D>,
>(
    content: I,
) -> Result<P, ASABRError> {
    let graph = parse_from_iter(content)?;

    Ok(P::new(Rc::new(RefCell::new(Multigraph::new(graph)?))))
}
