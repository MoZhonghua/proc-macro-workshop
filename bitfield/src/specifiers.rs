pub trait Specifier {
    const BITS: usize;
    type DataType;
    fn from_u64(v: u64) -> Self::DataType;
    fn to_u64(v: Self::DataType) -> u64;
}

impl Specifier for bool {
    const BITS: usize = 1;
    type DataType = bool;
    fn from_u64(v: u64) -> Self::DataType {
        if v == 1 {
            true
        } else {
            false
        }
    }
    fn to_u64(v: Self::DataType) -> u64 {
        if v {
            1
        } else {
            0
        }
    }
}

seq::seq!(N in 1..=8 {
    pub struct B#N {}
    impl Specifier for B#N {
        const BITS: usize = N;
        type DataType = u8;
        fn from_u64(v: u64) -> Self::DataType {
            v as Self::DataType
        }
        fn to_u64(v: Self::DataType) -> u64 {
            v as u64
        }
    }
});

seq::seq!(N in 9..=16 {
    pub struct B#N {}
    impl Specifier for B#N {
        const BITS: usize = N;
        type DataType = u16;
        fn from_u64(v: u64) -> Self::DataType {
            v as Self::DataType
        }
        fn to_u64(v: Self::DataType) -> u64 {
            v as u64
        }
    }
});

seq::seq!(N in 17..=32 {
    pub struct B#N {}
    impl Specifier for B#N {
        const BITS: usize = N;
        type DataType = u32;
        fn from_u64(v: u64) -> Self::DataType {
            v as Self::DataType
        }
        fn to_u64(v: Self::DataType) -> u64 {
            v as u64
        }
    }
});

seq::seq!(N in 33..=64 {
    pub struct B#N {}
    impl Specifier for B#N {
        const BITS: usize = N;
        type DataType = u64;
        fn from_u64(v: u64) -> Self::DataType {
            v as Self::DataType
        }
        fn to_u64(v: Self::DataType) -> u64 {
            v as u64
        }
    }
});
