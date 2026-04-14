#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Colour {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Colour {
    #[inline]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    #[inline]
    pub(crate) fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0, 1.0);
    pub const TRANSPARENT: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    pub const RED: Self = Self::new(1.0, 0.0, 0.0, 1.0);
    pub const GREEN: Self = Self::new(0.0, 0.502, 0.0, 1.0);
    pub const BLUE: Self = Self::new(0.0, 0.0, 1.0, 1.0);
    pub const YELLOW: Self = Self::new(1.0, 1.0, 0.0, 1.0);
    pub const MAGENTA: Self = Self::new(1.0, 0.0, 1.0, 1.0);
    pub const CYAN: Self = Self::new(0.0, 1.0, 1.0, 1.0);

    pub const LIME: Self = Self::new(0.0, 1.0, 0.0, 1.0);
    pub const ORANGE: Self = Self::new(1.0, 0.647, 0.0, 1.0);

    pub const DARKBLUE: Self = Self::new(0.0902, 0.1373, 0.2745, 1.0);
    pub const CORNFLOWER_BLUE: Self = Self::new(0.3922, 0.5843, 0.9294, 1.0);
    pub const DARK_GREY: Self = Self::new(0.2, 0.2, 0.2, 1.0);
    pub const GREY: Self = Self::new(0.5, 0.5, 0.5, 1.0);
    pub const LIGHT_GREY: Self = Self::new(0.827, 0.827, 0.827, 1.0);
}
