pub trait TotalSizeIsMultipleOfEightBits {}

// FIXME: no way to generate exactly same errors :(
pub struct ZeroMod8 {}
pub struct OneMod8 {}
pub struct TwoMod8 {}
pub struct ThreeMod8 {}
pub struct FourMod8 {}
pub struct FiveMod8 {}
pub struct SixMod8 {}
pub struct SevenMod8 {}

impl TotalSizeIsMultipleOfEightBits for ZeroMod8 {}

pub trait TypeSelector {
    type Type;
}

pub struct Selector<T> {
    _marker: std::marker::PhantomData<T>,
}

impl TypeSelector for Selector<[u8; 0]> {
    type Type = ZeroMod8;
}
impl TypeSelector for Selector<[u8; 1]> {
    type Type = OneMod8;
}
impl TypeSelector for Selector<[u8; 2]> {
    type Type = TwoMod8;
}
impl TypeSelector for Selector<[u8; 3]> {
    type Type = ThreeMod8;
}
impl TypeSelector for Selector<[u8; 4]> {
    type Type = FourMod8;
}
impl TypeSelector for Selector<[u8; 5]> {
    type Type = FiveMod8;
}
impl TypeSelector for Selector<[u8; 6]> {
    type Type = SixMod8;
}
impl TypeSelector for Selector<[u8; 7]> {
    type Type = SevenMod8;
}

pub struct Check<T: TotalSizeIsMultipleOfEightBits> {
    marker: std::marker::PhantomData<T>,
}

pub trait DiscriminantInRange {
}

pub struct True {}
pub struct False {}

impl DiscriminantInRange for True {}

pub struct Selector2<const T: bool> {
}

impl TypeSelector for Selector2<true> {
    type Type = True;
}

impl TypeSelector for Selector2<false> {
    type Type = False;
}

pub struct Check2<T: DiscriminantInRange> {
    marker: std::marker::PhantomData<T>,
}
