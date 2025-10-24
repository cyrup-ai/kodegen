use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
/// ChangeFeedInclude statements are an appendix
#[allow(dead_code)]
pub enum ChangeFeedInclude {
	Original,
}

impl Default for ChangeFeedInclude {
	fn default() -> Self {
		Self::Original
	}
}

impl fmt::Display for ChangeFeedInclude {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.write_str(match self {
			Self::Original => "Original",
		})
	}
}
