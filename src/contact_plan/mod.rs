use crate::contact::Contact;
use crate::node::Node;

pub mod asabr_file_lexer;
pub mod from_asabr_lexer;
pub mod from_ion_file;
pub mod from_tvgutil_file;

type ContactPlan<NNM, CNM, CCM> = (Vec<Node<NNM>>, Vec<Contact<CNM, CCM>>);
