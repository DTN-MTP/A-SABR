use crate::{
    contact_manager::ContactManager,
    contact_plan::ContactPlan,
    errors::ASABRError,
    node_manager::NodeManager,
    parsing::{LexFrom, Located, Parse},
};

/// Take an iterator over strings assumed to be lines, and parse a ContactPlan from it.
pub fn parse_from_iter<
    D: AsRef<str>,
    I: Iterator<Item = D>,
    NM: NodeManager + LexFrom<str>,
    CM: ContactManager + LexFrom<str>,
>(
    iter: I,
) -> Result<ContactPlan<NM, CM>, ASABRError> {
    let mut parser = Default::default();

    for (linenum, data) in iter.enumerate() {
        let mut line = data.as_ref();
        if let Some((new, _)) = line.split_once('#') {
            line = new
        }
        for (toknum, mut word) in line.split_ascii_whitespace().enumerate() {
            let locate = |e| {
                ASABRError::ParsingError(Located {
                    data: e,
                    line: linenum,
                    toknum,
                })
            };

            while &word[0..1] == "[" {
                ContactPlan::feed(ContactPlan::lex("[", &parser).map_err(locate)?, &mut parser)
                    .map_err(locate)?;
                word = &word[1..]
            }
            let mut finalbracket: usize = 0;
            while &word[word.len() - 1..word.len()] == "]" {
                word = &word[..word.len() - 1];
                finalbracket += 1;
            }
            if !word.is_empty() {
                ContactPlan::feed(
                    ContactPlan::lex(word, &parser).map_err(locate)?,
                    &mut parser,
                )
                .map_err(locate)?;
            }
            for _ in 0..finalbracket {
                ContactPlan::feed(ContactPlan::lex("]", &parser).map_err(locate)?, &mut parser)
                    .map_err(locate)?;
            }
        }
    }
    ContactPlan::parse(parser).map_err(ASABRError::ContactPlanError)
}
