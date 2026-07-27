#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub(super) enum BuildChannel {
    Debug,
    Internal,
    ProductionRelease,
}

pub(super) const fn compiled_build_channel() -> BuildChannel {
    #[cfg(debug_assertions)]
    {
        BuildChannel::Debug
    }
    #[cfg(all(not(debug_assertions), feature = "internal-write-preview"))]
    {
        BuildChannel::Internal
    }
    #[cfg(all(not(debug_assertions), not(feature = "internal-write-preview")))]
    {
        BuildChannel::ProductionRelease
    }
}
