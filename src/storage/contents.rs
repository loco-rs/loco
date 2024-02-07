use bytes::Bytes;

pub struct Contents {
    data: Bytes,
}

impl From<Bytes> for Contents {
    /// Converts a `Vec<u8>` into a `Contents` instance.
    ///
    /// # Returns
    ///
    /// Returns a `Contents` instance with the provided byte data.
    fn from(data: Bytes) -> Self {
        Self { data }
    }
}

impl From<Contents> for Vec<u8> {
    /// Convert `Contents` instance int a Vec<u8
    fn from(contents: Contents) -> Self {
        contents.data.to_vec()
    }
}

impl TryFrom<Contents> for String {
    type Error = std::string::FromUtf8Error;

    /// Tries to convert a `Contents` instance into a `String`.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a `String` with the UTF-8 representation
    /// of the byte data, or an error if the conversion fails.
    fn try_from(contents: Contents) -> Result<Self, Self::Error> {
        Self::from_utf8(contents.data.to_vec())
    }
}
