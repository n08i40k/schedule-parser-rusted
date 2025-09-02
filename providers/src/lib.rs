pub use base;

pub use provider_engels_polytechnic::EngelsPolytechnicProvider;
pub use provider_engels_polytechnic::UpdateSource as EngelsPolytechnicUpdateSource;

#[cfg(feature = "test")]
pub mod test_utils {
    pub use provider_engels_polytechnic::test_utils as engels_polytechnic;
}
