use sha1::Digest;
use std::hash::Hasher;

/// Hesher returning hash from the algorithm implementing Digest
pub struct DigestHasher<D: Digest> {
    digest: D,
}

impl<D> DigestHasher<D>
where
    D: Digest,
{
    /// Obtain hash.
    pub fn finalize(self) -> String {
        hex::encode(self.digest.finalize().0)
    }
}

impl<D> From<D> for DigestHasher<D>
where
    D: Digest,
{
    /// Creating a hash from an algorithm implementing Digest.
    fn from(digest: D) -> Self {
        DigestHasher { digest }
    }
}

impl<D: Digest> Hasher for DigestHasher<D> {
    /// Stopper to prevent calling the standard Hasher result.
    fn finish(&self) -> u64 {
        unimplemented!("Do not call finish()");
    }

    fn write(&mut self, bytes: &[u8]) {
        self.digest.update(bytes);
    }
}
