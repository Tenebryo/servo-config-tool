

#[derive(Clone, Copy, Debug)]
pub struct LayoutRect {
    pub x : u32, pub y : u32,
    pub w : u32, pub h : u32,
}

impl LayoutRect {
    pub fn new (w : u32, h : u32) -> LayoutRect {
        LayoutRect {
            x : 0, y : 0, w, h
        }
    }

    pub fn position(&self) -> [f32; 2] {
        [self.x as f32, self.y as f32]
    }

    pub fn dimensions(&self) -> [f32; 2] {
        [self.w as f32, self.h as f32]
    }

    pub fn vertical_split_left_abs(&self, w : u32) -> (LayoutRect, LayoutRect) {
        (
            LayoutRect {
                w,
                ..*self
            },
            LayoutRect {
                w : self.w - w,
                x : self.x + w,
                ..*self
            }
        )
    }
    
    pub fn vertical_split_right_abs(&self, w : u32) -> (LayoutRect, LayoutRect) {
        (
            LayoutRect {
                w : self.w - w,
                ..*self
            },
            LayoutRect {
                w,
                x : self.x + self.w - w,
                ..*self
            }
        )
    }
    

    pub fn horizontal_split_top_abs(&self, h : u32) -> (LayoutRect, LayoutRect) {
        (
            LayoutRect {
                h,
                ..*self
            },
            LayoutRect {
                h : self.h - h,
                y : self.y + h,
                ..*self
            }
        )
    }
    
    pub fn horizontal_split_bottom_abs(&self, h : u32) -> (LayoutRect, LayoutRect) {
        (
            LayoutRect {
                h : self.h - h,
                ..*self
            },
            LayoutRect {
                h,
                y : self.y + self.h - h,
                ..*self
            }
        )
    }
}