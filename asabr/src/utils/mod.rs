extern crate alloc;

///rexeport for macro use
pub use generativity::{make_guard,Id,Guard};

pub struct OptUsize(usize);

impl From<Option<usize>> for OptUsize {
    fn from(value: Option<usize>) -> Self {
        match value {
            None => OptUsize(usize::MAX),
            Some(v) => OptUsize(v)
        }
    }
}

impl From<OptUsize> for Option<usize> {
    fn from(value: OptUsize) -> Self {
        match value.0 {
            usize::MAX => None,
            v => Some(v)
        }
    }
}

/// mk_graph_pathfinding!(graphname,pathfindername,NODE_MANAGER,CONTACT_MANAGER,PATHFINDERTYPE,content,content_type?)
/// create a new graph and pathfinder with the provided names and flavor, parsing a ASABR CP from content
/// The optional content_type argument can precise how the content is handed:
///    iter: An iterator over contact plan lines [default]
///    raw: An &str over the whole file content
///    filename: a file to open and parse. This require STD
#[macro_export]
macro_rules! mk_graph_pathfinding {
    ($graph:ident,$path:ident,$NM:ty,$CM:ty,$P:ty,$content:expr$(,iterator)?) => {
        $crate::utils::make_guard!($graph);
        let mut $graph = $crate::multigraph::Multigraph::new($graph, $crate::contact_plan::asabr_file_lexer::parse_from_iter::<$NM,$CM>($content)?)?;
        $crate::utils::make_guard!($path);
        let mut $path = <$P as $crate::pathfinding::Pathfinding<_,_>>::new($path,&mut $graph);
    };

    ($graph:ident,$path:ident,$NM:ty,$CM:ty,$P:ty,$content:ident,raw) => {
        let $path = $content.lines();
        $crate::mk_graph_pathfinding!($graph,$path,$NM,$CM,$P,$path);
    };
    ($graph:ident,$path:ident,$NM:ty,$CM:ty,$P:ty,$content:ident,file) => {
        let $path = match std::fs::File::open($content) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Error while trying to open file: {e}");
                return Err($crate::errors::ASABRError::ParsingError($crate::parsing::Located{
                    data: "Error while opennig file",
                    line: 0,
                    toknum: 0,
                }))
            }
        }
        let $path = {
                use std::io::{BufRead, BufReader};
                std::id::Bufreader::new($path).lines().map(|l|) {
                    l.map_err(|e| {
                        eprintln!("Error while reading file: {e}"),
                        return Err($crate::errors::ASABRError::ParsingError($crate::parsing::Located{
                            data: "Error while opennig file",
                            line: 0,
                            toknum: 0,
                        }))
                    })
                }
            }
        $crate::mk_graph_pathfinding!($graph,$path,$NM,$CM,$P,$path);
    }
}
