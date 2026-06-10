extern crate alloc;

use crate::parse_single_tok;
use alloc::{collections::BTreeMap as HashMap, string::String, vec::Vec};
use core::{fmt::Display, marker::PhantomData, str::FromStr};

/// Represents a HashMap with node IDs as keys and node ID lists as values
pub type NodeIDMap = HashMap<NodeID, Vec<NodeID>>;

/// Represents the unique inner identifier for a node.
pub type NodeID = u16;
const_assert!(size_of::<NodeID>() <= size_of::<usize>());

/// Represents a duration in units (e.g., seconds).
pub type Duration = f64;

/// Represents a date (could represent days since a specific epoch).lo
pub type Date = Duration;

/// Represents the priority of a task or node.
pub type Priority = i8;

/// Represents the volume of data (in bytes, for example).
pub type Volume = f64;

/// Represents a data transfer rate (in bits per second).
pub type DataRate = f64;

/// Represents the count of hops in a routing path.
pub type HopCount = u16;

/// Represent an value encompassing all of the above, typically for use in parser
//  Must implement FromStr and TryInto to all the above
#[derive(Clone, Copy, Debug)]
pub struct AnyNumber(i64);
assert_impl_all!(
    AnyNumber: TryFrom<&'static str>,
    Into<Duration>,
    Into<Priority>,
    Into<Volume>,
    Into<DataRate>,
    Into<HopCount>,
);

/// The name of a node. Use the "debug" feature to populate it with usefull data
/// Can be created from a &str, and displayed
#[derive(Clone, Debug)]
pub struct NodeName {
    #[cfg(feature = "debug")]
    name: String,
    _phantom: PhantomData<String>,
}

impl FromStr for AnyNumber {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse().map_err(|_| ())?))
    }
}
impl TryFrom<&str> for AnyNumber {
    type Error = ();

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Ok(Self(s.parse().map_err(|_| ())?))
    }
}

impl From<AnyNumber> for f64 {
    fn from(value: AnyNumber) -> Self {
        value.0 as Self
    }
}
impl From<AnyNumber> for i8 {
    fn from(value: AnyNumber) -> Self {
        value.0 as Self
    }
}
impl From<AnyNumber> for u16 {
    fn from(value: AnyNumber) -> Self {
        value.0 as Self
    }
}

parse_single_tok!(NodeName);

impl Display for NodeName {
    #[allow(unused_variables)]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(feature = "debug")]
        write!(f, "{}", self.name)?;
        Ok(())
    }
}

impl<T: AsRef<str>> From<T> for NodeName {
    #[allow(unused_variables)]
    fn from(value: T) -> Self {
        Self {
            #[cfg(feature = "debug")]
            name: value.as_ref().into(),
            _phantom: PhantomData,
        }
    }
}
