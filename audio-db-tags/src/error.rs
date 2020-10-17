use snafu::{ResultExt, Snafu};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Error opening file for parsing: {}", source))]
    OpenFile { source: std::io::Error },

    #[snafu(display("Error parsing file: {}", source))]
    ParseFile { source: deku::error::DekuError },
}

pub(crate) trait ErrorContextExt<T, E> {
    fn ctx_open_file(self) -> Result<T, Error>
    where
        E: Into<std::io::Error>;

    fn ctx_parse_file(self) -> Result<T, Error>
    where
        E: Into<deku::error::DekuError>;
}

impl<T, E> ErrorContextExt<T, E> for Result<T, E> {
    fn ctx_open_file(self) -> Result<T, Error>
    where
        E: Into<std::io::Error>,
    {
        self.map_err(Into::into).context(OpenFile {})
    }

    fn ctx_parse_file(self) -> Result<T, Error>
    where
        E: Into<deku::error::DekuError>,
    {
        self.map_err(Into::into).context(ParseFile {})
    }
}
