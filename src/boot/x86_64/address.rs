#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysicalAddress(u64);

impl PhysicalAddress {
    pub const NULL: Self = Self(0);

    pub const fn new(address: u64) -> Self {
        Self(address)
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }

    pub const fn low32(self) -> u32 {
        self.0 as u32
    }

    pub const fn high32(self) -> u32 {
        (self.0 >> 32) as u32
    }

    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    pub const fn is_above_4g(self) -> bool {
        self.0 > u32::MAX as u64
    }

    pub const fn checked_add(self, size: u64) -> Option<Self> {
        match self.0.checked_add(size) {
            Some(address) => Some(Self(address)),
            None => None,
        }
    }
}

impl Default for PhysicalAddress {
    fn default() -> Self {
        Self::NULL
    }
}
