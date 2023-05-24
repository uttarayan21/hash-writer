//! A hasher that will be a wrapper over any  
//! [`std::io::Write`][std::io::Write] /  
//! [`futures::io::AsyncWrite`][futures::io::AsyncWrite] /  
//! [`tokio::io::AsyncWrite`][tokio::io::AsyncWrite] object  
//!
//!  You can wrap any of the previous trait object inside and that will transparently hash the data that is being
//!  written to it.  
//!
//!
//! The object should implement AsyncRead so that it can wrap some data and then read from that
//! transparently while offloading the hashing to another thread.
//! ```rust
//! extern crate sha2;
//! use write_hasher::{WriteHasher, MinDigest};
//! let mut src = std::fs::File::open(".gitignore").unwrap();
//! let sink = std::io::sink();
//! let mut hasher = WriteHasher::<sha2::Sha256, _>::new(sink);
//! std::io::copy(&mut src, &mut hasher).unwrap();
//! let x = hasher.finalize();
//! let x = format!("{:x}", x);
//! assert_eq!(
//!     "c1e953ee360e77de57f7b02f1b7880bd6a3dc22d1a69e953c2ac2c52cc52d247",
//!     x
//! );
//! ```

#[cfg(all(
    feature = "digest",
    any(
        feature = "concrete_impls",
        feature = "sha1",
        feature = "sha2",
        feature = "md2",
        feature = "md4",
        feature = "md5",
        feature = "blake2",
        feature = "crc32fast"
    )
))]
compile_error!("Please either use digest feature (for generic impls) or
               concrete_impls (sha1, sha2, md2, md4, md5, blake2, crc32fast) features (for concrete impls),
               but not both");

#[cfg(any(feature = "futures", feature = "tokio"))]
use core::pin::Pin;
use core::task::Poll;
#[cfg(feature = "digest")]
use digest::Digest;

/// A hasher that will be a wrapper over any Write / AsyncWrite object and transparently calculate
/// hash for any data written to it
#[cfg_attr(any(feature = "futures", feature = "tokio"), pin_project::pin_project)]
#[derive(Default)]
pub struct WriteHasher<D, T> {
    hasher: D,
    #[cfg_attr(any(feature = "futures", feature = "tokio"), pin)]
    inner: T,
}

impl<D, T> WriteHasher<D, T> {
    pub fn new_with_hasher(inner: T, hasher: D) -> Self {
        Self { hasher, inner }
    }

    pub fn new(inner: T) -> Self
    where
        D: Default,
    {
        Self {
            hasher: Default::default(),
            inner,
        }
    }
}

// #[cfg(feature = "digest")]
// impl<D: Digest, T> WriteHasher<D, T> {
//     pub fn new(inner: T) -> Self {
//         Self {
//             hasher: D::new(),
//             inner,
//         }
//     }
// }

#[cfg(feature = "digest")]
impl<D: Digest + digest::Reset, T> WriteHasher<D, T> {
    pub fn reset(&mut self) {
        <D as Digest>::reset(&mut self.hasher)
    }
}

/// A minimal version of [`Digest`][digest::digest] trait that is used to implement the WriteHasher
/// and all implementations of the Digest trait.
pub trait MinDigest {
    type Output;
    fn update(&mut self, data: impl AsRef<[u8]>);
    fn finalize(self) -> Self::Output;
}

impl<MD: MinDigest, T> MinDigest for WriteHasher<MD, T> {
    type Output = MD::Output;
    fn update(&mut self, data: impl AsRef<[u8]>) {
        self.hasher.update(data)
    }
    fn finalize(self) -> MD::Output {
        self.hasher.finalize()
    }
}

#[cfg(feature = "digest")]
impl<T: Digest> MinDigest for T {
    type Output = digest::Output<T>;
    fn update(&mut self, data: impl AsRef<[u8]>) {
        <T as Digest>::update(self, data)
    }
    fn finalize(self) -> Self::Output {
        <T as Digest>::finalize(self)
    }
}

#[cfg(any(
    feature = "sha2",
    feature = "sha1",
    feature = "md2",
    feature = "md4",
    feature = "md5",
    feature = "blake2"
))]
macro_rules! delegate_digest_mindigest {
    ($($x:ty),*) => {
        $(
            impl MinDigest for $x {
                type Output = digest::Output<$x>;
                fn update(&mut self, data: impl AsRef<[u8]>) {
                    <Self as digest::Digest>::update(self, data)
                }
                fn finalize(self) -> Self::Output {
                    <Self as digest::Digest>::finalize(self)
                }
            }

            impl<T> crate::WriteHasher<$x, T> {
                pub fn new(inner: T) -> Self {
                    Self {
                        hasher: <$x as ::digest::Digest>::new(),
                        inner,
                    }
                }
            }

        )*
    };
}

#[cfg(feature = "sha2")]
mod sha2 {
    use super::MinDigest;
    delegate_digest_mindigest!(
        sha2::Sha224,
        sha2::Sha256,
        sha2::Sha384,
        sha2::Sha512,
        sha2::Sha512_224,
        sha2::Sha512_256
    );
}

#[cfg(feature = "sha1")]
mod sha1 {
    use super::MinDigest;
    delegate_digest_mindigest!(sha1::Sha1);
}

#[cfg(feature = "md2")]
mod md2 {
    use super::MinDigest;
    delegate_digest_mindigest!(md2::Md2);
}

#[cfg(feature = "md4")]
mod md4 {
    use super::MinDigest;
    delegate_digest_mindigest!(md4::Md4);
}

#[cfg(feature = "md5")]
mod md5 {
    use super::MinDigest;
    impl MinDigest for md5::Context {
        type Output = md5::Digest;
        fn update(&mut self, data: impl AsRef<[u8]>) {
            self.consume(data)
        }
        fn finalize(self) -> Self::Output {
            self.compute()
        }
    }

    impl<T> crate::WriteHasher<md5::Context, T> {
        pub fn new(inner: T) -> Self {
            Self {
                hasher: md5::Context::new(),
                inner,
            }
        }
    }
}

#[cfg(feature = "blake2")]
mod blake2 {
    use super::MinDigest;
    // use digest::consts::*;
    // use digest::typenum::*;

    // delegate_digest_mindigest!(blake2::Blake2b);
    delegate_digest_mindigest!(blake2::Blake2b512);
    // delegate_digest_mindigest!(blake2::Blake2bCore);
    // delegate_digest_mindigest!(blake2::Blake2bMac512);
    // delegate_digest_mindigest!(blake2::Blake2bVar);
    // delegate_digest_mindigest!(blake2::Blake2s);
    delegate_digest_mindigest!(blake2::Blake2s256);
    // delegate_digest_mindigest!(blake2::Blake2sCore);
    // delegate_digest_mindigest!(blake2::Blake2sMac256);
    // delegate_digest_mindigest!(blake2::Blake2sVar);
}

#[cfg(feature = "crc32fast")]
mod crc32fast {
    use super::MinDigest;
    impl MinDigest for crc32fast::Hasher {
        type Output = u32;
        fn update(&mut self, data: impl AsRef<[u8]>) {
            self.update(data.as_ref())
        }
        fn finalize(self) -> Self::Output {
            self.finalize()
        }
    }

    impl<T> crate::WriteHasher<crc32fast::Hasher, T> {
        pub fn new(inner: T) -> Self {
            Self {
                hasher: crc32fast::Hasher::new(),
                inner,
            }
        }
    }
}

// #[cfg(feature = "crc32c")]
pub mod crc32c {
    use super::MinDigest;
    #[repr(transparent)]
    #[derive(Debug, Default)]
    pub struct Crc32c(u32);

    impl Crc32c {
        pub fn new() -> Self {
            Default::default()
        }
    }

    impl MinDigest for Crc32c {
        type Output = u32;
        fn update(&mut self, data: impl AsRef<[u8]>) {
            self.0 = crc32c::crc32c_append(self.0, data.as_ref())
        }
        fn finalize(self) -> Self::Output {
            self.0
        }
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
#[cfg(feature = "tokio")]
impl<D: MinDigest, T: tokio::io::AsyncWrite + std::marker::Unpin> tokio::io::AsyncWrite
    for WriteHasher<D, T>
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let ah = self.project();
        let r = ah.inner.poll_write(cx, buf);
        if let Poll::Ready(Ok(n)) = r {
            ah.hasher.update(&buf[..n]);
        }
        r
    }
    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let ah = self.project();
        ah.inner.poll_flush(cx)
    }
    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let ah = self.project();
        ah.inner.poll_shutdown(cx)
    }
}

#[cfg(feature = "futures")]
impl<D: MinDigest, T: futures::io::AsyncWrite + std::marker::Unpin> futures::io::AsyncWrite
    for WriteHasher<D, T>
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<futures::io::Result<usize>> {
        let ah = self.project();
        let r = ah.inner.poll_write(cx, buf);
        if let Poll::Ready(Ok(n)) = r {
            ah.hasher.update(&buf[..n]);
        }
        r
    }
    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<futures::io::Result<()>> {
        let ah = self.project();
        ah.inner.poll_flush(cx)
    }
    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<futures::io::Result<()>> {
        let ah = self.project();
        ah.inner.poll_close(cx)
    }
}

#[cfg(feature = "stdio")]
impl<D: MinDigest, T: std::io::Write> std::io::Write for WriteHasher<D, T> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let r = std::io::Write::write(&mut self.inner, buf);
        if let Ok(n) = r {
            MinDigest::update(&mut self.hasher, &buf[..n]);
        }
        r
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    #[cfg(feature = "tokio")]
    #[cfg(any(feature = "sha2", feature = "digest"))]
    async fn test_read() {
        extern crate sha2;
        let mut src = tokio::fs::File::open(".gitignore").await.unwrap();
        let sink = tokio::io::sink();
        let mut hasher = WriteHasher::<sha2::Sha256, _>::new(sink);
        tokio::io::copy(&mut src, &mut hasher).await.unwrap();
        // hasher.write_all(b"hello worlding").await.unwrap();
        let x = hasher.finalize();
        let x = format!("{:x}", x);
        assert_eq!(
            "c1e953ee360e77de57f7b02f1b7880bd6a3dc22d1a69e953c2ac2c52cc52d247",
            x
        );
    }

    #[tokio::test]
    #[cfg(feature = "futures")]
    #[cfg(any(feature = "sha2", feature = "digest"))]
    async fn test_read_futures() {
        extern crate sha2;
        let src = std::fs::read(".gitignore").unwrap();
        let src = futures::io::Cursor::new(src);
        let sink = futures::io::sink();
        let mut hasher = WriteHasher::<sha2::Sha256, _>::new(sink);
        futures::io::copy(src, &mut hasher).await.unwrap();
        // hasher.write_all(b"hello worlding").await.unwrap();
        let x = hasher.finalize();
        let x = format!("{:x}", x);
        assert_eq!(
            "c1e953ee360e77de57f7b02f1b7880bd6a3dc22d1a69e953c2ac2c52cc52d247",
            x
        );
    }

    #[tokio::test]
    #[cfg(feature = "tokio")]
    #[cfg(feature = "crc32fast")]
    async fn test_crc32() {
        extern crate crc32fast;
        let mut src = tokio::fs::File::open(".gitignore").await.unwrap();
        let sink = tokio::io::sink();
        let mut hasher =
            WriteHasher::<crc32fast::Hasher, _>::new_with_hasher(sink, Default::default());
        tokio::io::copy(&mut src, &mut hasher).await.unwrap();
        // hasher.write_all(b"hello worlding").await.unwrap();
        let x = hasher.finalize();
        assert_eq!(x, 0x705ffe14);
    }

    #[test]
    #[cfg(feature = "stdio")]
    #[cfg(any(feature = "sha2", feature = "digest"))]
    fn test_read_stdio() {
        extern crate sha2;
        let mut src = std::fs::File::open(".gitignore").unwrap();
        let sink = std::io::sink();
        let mut hasher = WriteHasher::<sha2::Sha256, _>::new(sink);
        std::io::copy(&mut src, &mut hasher).unwrap();
        // hasher.write_all(b"hello worlding").await.unwrap();
        let x = hasher.finalize();
        let x = format!("{:x}", x);
        assert_eq!(
            "c1e953ee360e77de57f7b02f1b7880bd6a3dc22d1a69e953c2ac2c52cc52d247",
            x
        );
    }

    #[tokio::test]
    #[ignore]
    #[cfg(all(feature = "tokio", feature = "stdio"))]
    #[cfg(any(feature = "crc32c", feature = "digest"))]
    async fn test_tokio_bigfile() {
        let mut src = tokio::fs::File::open("file.zip").await.unwrap();
        let sink = tokio::io::sink();
        let mut hasher = WriteHasher::<crc32c::Crc32c, _>::new(sink);
        tokio::io::copy(&mut src, &mut hasher).await.unwrap();
        // hasher.write_all(b"hello worlding").await.unwrap();
        let x = hasher.finalize();
        assert_eq!(x, 0xbd7a7dfe);
        let mut src = std::fs::File::open("file.zip").unwrap();
        let sink = std::io::sink();
        let mut hasher = WriteHasher::<crc32c::Crc32c, _>::new(sink);
        std::io::copy(&mut src, &mut hasher).unwrap();
        // hasher.write_all(b"hello worlding").await.unwrap();
        let y = hasher.finalize();
        assert_eq!(x, y);
        assert_eq!(3178921470, x);
        assert_eq!(3178921470, y);
    }
}
