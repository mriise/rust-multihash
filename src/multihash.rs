use crate::hasher::Size;
use crate::Error;
#[cfg(feature = "std")]
use core::convert::TryInto;
use core::fmt::Debug;
use generic_array::GenericArray;

/// A Multihash instance that only supports the basic functionality and no hashing.
///
/// With this Multihash implementation you can operate on Multihashes in a generic way, but
/// no hasher implementation is associated with the code.
///
/// # Example
///
/// ```
/// use tiny_multihash::{Multihash, U64};
///
/// const Sha3_256: u64 = 0x16;
/// let digest_bytes = [
///     0x16, 0x20, 0x64, 0x4b, 0xcc, 0x7e, 0x56, 0x43, 0x73, 0x04, 0x09, 0x99, 0xaa, 0xc8, 0x9e,
///     0x76, 0x22, 0xf3, 0xca, 0x71, 0xfb, 0xa1, 0xd9, 0x72, 0xfd, 0x94, 0xa3, 0x1c, 0x3b, 0xfb,
///     0xf2, 0x4e, 0x39, 0x38,
/// ];
/// let mh = Multihash::<U64>::from_bytes(&digest_bytes).unwrap();
/// assert_eq!(mh.code(), Sha3_256);
/// assert_eq!(mh.size(), 32);
/// assert_eq!(mh.digest(), &digest_bytes[2..]);
/// ```
// TODO vmx 2020-09-22: Make custom codec serialization possible again
//#[cfg_attr(feature = "scale-codec", derive(parity_scale_codec::Decode))]
//#[cfg_attr(feature = "scale-codec", derive(parity_scale_codec::Encode))]
#[cfg_attr(feature = "serde-codec", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde-codec", derive(serde::Serialize))]
#[cfg_attr(feature = "serde-codec", serde(bound = "S: Size"))]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Multihash<S: Size> {
    /// The code of the Multihash.
    code: u64,
    /// The actual size of the digest in bytes (not the allocated size).
    size: u8,
    /// The digest.
    digest: GenericArray<u8, S>,
}

impl<S: Size> Multihash<S> {
    /// Wraps the digest in a multihash.
    pub fn wrap(code: u64, input_digest: &[u8]) -> Result<Self, Error> {
        if input_digest.len() > S::to_usize() {
            return Err(Error::InvalidSize(input_digest.len() as _));
        }
        let size = input_digest.len();
        let mut digest = GenericArray::default();
        digest[..size].copy_from_slice(input_digest);
        Ok(Self {
            code,
            size: size as u8,
            digest,
        })
    }

    /// Returns the code of the multihash.
    pub fn code(&self) -> u64 {
        self.code
    }

    /// Returns the size of the digest.
    pub fn size(&self) -> u8 {
        self.size
    }

    /// Returns the digest.
    pub fn digest(&self) -> &[u8] {
        &self.digest[..self.size as usize]
    }

    /// Reads a multihash from a byte stream.
    #[cfg(feature = "std")]
    pub fn read<R: std::io::Read>(r: R) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let (code, size, digest) = read_multihash(r)?;
        Ok(Self { code, size, digest })
    }

    /// Parses a multihash from a bytes.
    ///
    /// You need to make sure the passed in bytes have the correct length. The digest length
    /// needs to match the `size` value of the multihash.
    #[cfg(feature = "std")]
    pub fn from_bytes(mut bytes: &[u8]) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let result = Self::read(&mut bytes)?;
        // There were more bytes supplied than read
        if !bytes.is_empty() {
            return Err(Error::InvalidSize(bytes.len().try_into().expect(
                "Currently the maximum size is 255, therefore always fits into usize",
            )));
        }

        Ok(result)
    }

    /// Writes a multihash to a byte stream.
    #[cfg(feature = "std")]
    pub fn write<W: std::io::Write>(&self, w: W) -> Result<(), Error> {
        write_multihash(w, self.code(), self.size(), self.digest())
    }

    /// Returns the bytes of a multihash.
    #[cfg(feature = "std")]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        self.write(&mut bytes)
            .expect("writing to a vec should never fail");
        bytes
    }
}

/// Writes the multihash to a byte stream.
#[cfg(feature = "std")]
pub fn write_multihash<W>(mut w: W, code: u64, size: u8, digest: &[u8]) -> Result<(), Error>
where
    W: std::io::Write,
{
    use unsigned_varint::encode as varint_encode;

    let mut code_buf = varint_encode::u64_buffer();
    let code = varint_encode::u64(code, &mut code_buf);

    let mut size_buf = varint_encode::u8_buffer();
    let size = varint_encode::u8(size, &mut size_buf);

    w.write_all(code)?;
    w.write_all(size)?;
    w.write_all(digest)?;
    Ok(())
}

/// Reads a multihash from a byte stream that contains a full multihash (code, size and the digest)
///
/// Returns the code, size and the digest. The size is the actual size and not the
/// maximum/allocated size of the digest.
///
/// Currently the maximum size for a digest is 255 bytes.
#[cfg(feature = "std")]
pub fn read_multihash<R, S>(mut r: R) -> Result<(u64, u8, GenericArray<u8, S>), Error>
where
    R: std::io::Read,
    S: Size,
{
    use unsigned_varint::io::read_u64;

    let code = read_u64(&mut r)?;
    let size = read_u64(&mut r)?;

    if size > S::to_u64() || size > u8::MAX as u64 {
        return Err(Error::InvalidSize(size));
    }

    let mut digest = GenericArray::default();
    r.read_exact(&mut digest[..size as usize])?;
    Ok((code, size as u8, digest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multihash_impl::Code;

    #[test]
    fn roundtrip() {
        let hash = Code::Sha2_256.digest(b"hello world");
        let mut buf = [0u8; 35];
        hash.write(&mut buf[..]).unwrap();
        let hash2 = Multihash::read(&buf[..]).unwrap();
        assert_eq!(hash, hash2);
    }

    //#[test]
    //#[cfg(feature = "scale-codec")]
    //fn test_scale() {
    //    use parity_scale_codec::{Decode, Encode};
    //
    //    let mh = RawMultihash::default();
    //    let bytes = mh.encode();
    //    let mh2: RawMultihash = Decode::decode(&mut &bytes[..]).unwrap();
    //    assert_eq!(mh, mh2);
    //}
    //
    #[test]
    #[cfg(feature = "serde-codec")]
    fn test_serde() {
        let mh = Multihash::<crate::U32>::default();
        let bytes = serde_json::to_string(&mh).unwrap();
        let mh2 = serde_json::from_str(&bytes).unwrap();
        assert_eq!(mh, mh2);
    }
}
