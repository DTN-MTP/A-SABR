extern crate alloc;

/// Re-exports generativity utilities used by graph-construction macros.
pub use generativity::{Guard, Id, make_guard};

/// Compact representation of an optional `usize`.
///
/// `None` is encoded as `usize::MAX`; all other values are encoded directly.
pub struct OptUsize(usize);

impl From<Option<usize>> for OptUsize {
    fn from(value: Option<usize>) -> Self {
        match value {
            None => OptUsize(usize::MAX),
            Some(v) => OptUsize(v),
        }
    }
}

impl From<OptUsize> for Option<usize> {
    fn from(value: OptUsize) -> Self {
        match value.0 {
            usize::MAX => None,
            v => Some(v),
        }
    }
}

/// Builds a `Multigraph` from ASABR contact-plan content.
///
/// This macro creates a generativity guard, parses the contact plan, and binds
/// the resulting graph to the provided variable name.
///
/// Usage:
///
/// ```ignore
/// mk_graph!(graph, NoManagement, CMDynStandard, lines);
/// mk_graph!(graph, NoManagement, CMDynStandard, raw_content, raw);
/// mk_graph!(graph, NoManagement, CMDynStandard, filename, file);
/// ```
///
/// The optional content mode specifies how the input is supplied:
///
/// - `iterator`: an iterator over contact-plan lines. This is the default.
/// - `raw`: an `&str` containing the whole contact-plan content.
/// - `file`: a file path to open and parse. This requires `std`.
#[macro_export]
macro_rules! mk_graph {
    ($graph:ident,$NM:ty,$CM:ty,$content:expr$(,iterator)?) => {
        $crate::utils::make_guard!($graph);
        let mut $graph = $crate::multigraph::Multigraph::new(
            $graph,
            $crate::contact_plan::asabr_file_lexer::parse_from_iter::<$NM, $CM>($content)?,
        )?;
    };

    ($graph:ident,$NM:ty,$CM:ty,$content:ident,raw) => {
        $crate::mk_graph!($graph, $NM, $CM, $content.lines());
    };
    ($graph:ident,$NM:ty,$CM:ty,$content:ident,file) => {
        $crate::mk_graph!($graph, $NM, $CM, {
            use std::io::{BufRead, BufReader};
            std::io::BufReader::new(match std::fs::File::open($content) {
                Ok(content) => content,
                Err(e) => {
                    eprintln!("Error while trying to open file: {e}");
                    return Err($crate::errors::ASABRError::ParsingError(
                        $crate::parsing::Located {
                            data: "Error while opennig file",
                            line: 0,
                            toknum: 0,
                        },
                    ));
                }
            })
            .lines()
            .map(|l| {
                l.map_err(|e| {
                    eprintln!("Error while reading file: {e}");
                    panic!();
                })
                .unwrap()
            })
        });
    };
}
