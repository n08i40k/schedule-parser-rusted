use sha1::Digest;
use std::hash::Hasher;

/// Хешер возвращающий хеш из алгоритма реализующего Digest
pub struct DigestHasher<D: Digest> {
    digest: D,
}

impl<D> DigestHasher<D>
where
    D: Digest,
{
    /// Получение хеша
    pub fn finalize(self) -> String {
        hex::encode(self.digest.finalize().0)
    }
}

impl<D> From<D> for DigestHasher<D>
where
    D: Digest,
{
    /// Создания хешера из алгоритма реализующего Digest
    fn from(digest: D) -> Self {
        DigestHasher { digest }
    }
}

impl<D: Digest> Hasher for DigestHasher<D> {
    /// Заглушка для предотвращения вызова стандартного результата Hasher
    fn finish(&self) -> u64 {
        unimplemented!("Do not call finish()");
    }

    fn write(&mut self, bytes: &[u8]) {
        self.digest.update(bytes);
    }
}
