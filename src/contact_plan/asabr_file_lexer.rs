use crate::{
    contact_manager::ContactManager,
    contact_plan::ContactPlan,
    errors::ASABRError,
    node_manager::NodeManager,
    parsing::{LexFrom, Located, Parse},
};

/// Take an iterator over strings assumed to be lines, and parse a ContactPlan from it.
/// Templated over a NodeManager and a ContactManager, wich must be compatible with the file syntax
/// to successfully parse from it
pub fn parse_from_iter<
    NM: NodeManager + LexFrom<str>,
    CM: ContactManager + LexFrom<str>,
    I: Iterator<Item: AsRef<str>>,
>(
    iter: I,
) -> Result<ContactPlan<NM, CM>, ASABRError> {
    let mut parser = Default::default();

    for (linenum, data) in iter.enumerate() {
        let mut line = data.as_ref();
        if let Some((new, _)) = line.split_once('#') {
            line = new
        }
        for (toknum, word) in line.split_ascii_whitespace().enumerate() {
            let locate = |e| {
                ASABRError::ParsingError(Located {
                    data: e,
                    line: linenum,
                    toknum,
                })
            };

            let main = word.trim_start_matches(['[', ',']);
            let diff = word.len() - main.len();

            for i in 0..diff {
                ContactPlan::feed(
                    ContactPlan::lex(&word[i..i + 1], &parser).map_err(locate)?,
                    &mut parser,
                )
                .map_err(locate)?;
            }

            let main2 = main.trim_end_matches([',', ']']);
            let end = &main[main2.len()..];

            if !main2.is_empty() {
                ContactPlan::feed(
                    ContactPlan::lex(main2, &parser).map_err(locate)?,
                    &mut parser,
                )
                .map_err(locate)?;
            }
            for i in 0..end.len() {
                ContactPlan::feed(
                    ContactPlan::lex(&end[i..i + 1], &parser).map_err(locate)?,
                    &mut parser,
                )
                .map_err(locate)?;
            }
        }
    }
    ContactPlan::parse(parser).map_err(ASABRError::ContactPlanError)
}
