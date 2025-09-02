use sha1::Digest;
use sha1::digest::OutputSizeUser;
use sha1::digest::typenum::Unsigned;
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
        static ALPHABET: [char; 16] = [
            '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
        ];
        
        let mut hex = String::with_capacity(<D as OutputSizeUser>::OutputSize::USIZE * 2);

        for byte in self.digest.finalize().0.into_iter() {
            let byte: u8 = byte;

            hex.push(ALPHABET[(byte >> 4) as usize]);
            hex.push(ALPHABET[(byte & 0xF) as usize]);
        }

        hex
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
