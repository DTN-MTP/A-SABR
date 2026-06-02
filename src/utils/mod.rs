extern crate alloc;
use alloc::rc::Rc;
use core::cell::RefCell;

use crate::{
    contact_manager::ContactManager,
    contact_plan::{asabr_file_lexer::FileLexer, from_asabr_lexer::ASABRContactPlan},
    errors::ASABRError,
    multigraph::Multigraph,
    node_manager::NodeManager,
    parsing::{DispatchParser, Parser, StaticMarkerMap},
    pathfinding::Pathfinding,
};

pub fn init_pathfinding<
    'a,
    NM: NodeManager + DispatchParser<NM> + Parser<NM>,
    CM: ContactManager + DispatchParser<CM> + Parser<CM>,
    P: Pathfinding<NM, CM>,
    T: Iterator<Item = &'a str>,
>(
    content: T,
    node_marker_map: Option<&StaticMarkerMap<NM>>,
    contact_marker_map: Option<&StaticMarkerMap<CM>>,
) -> Result<P, ASABRError> {
    let mut mylexer = FileLexer::new(content);
    let nodes_n_contacts =
        ASABRContactPlan::parse::<NM, CM>(&mut mylexer, node_marker_map, contact_marker_map)
            .unwrap();

    Ok(P::new(Rc::new(RefCell::new(Multigraph::new(
        nodes_n_contacts,
    )?))))
}
