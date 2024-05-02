//! Transforms between coordinate systems (such as grid/logical <=> screen pixels).

use eframe::egui::Pos2;
use mdrc_pacbot_server::grid::Wall;
use nalgebra::Point2;

/// A 2D transform consisting of per-axis scale and translation.
#[derive(Copy, Clone)]
pub struct Transform {
    scale_x: f32,
    scale_y: f32,
    offset_x: f32,
    offset_y: f32,
    /// If true, x and y swap positions as part of the transform
    flipped: bool,
}

impl Transform {
    /// Creates a new `Transform` that maps the rect `(src_p1, src_p2)` inside `(dst_p1, dst_p2)`,
    /// adding padding/letterboxing so that the src rect fits inside the dst rect while preserving
    /// its aspect ratio.
    pub fn new_letterboxed(
        src_p1: Pos2,
        src_p2: Pos2,
        dst_p1: Pos2,
        dst_p2: Pos2,
        flipped: bool,
    ) -> Self {
        // Compare the aspect ratios to determine the letterboxing direction.
        let src_width = (src_p1.x - src_p2.x).abs();
        let src_height = (src_p1.y - src_p2.y).abs();
        let dst_width = (dst_p1.x - dst_p2.x).abs();
        let dst_height = (dst_p1.y - dst_p2.y).abs();
        if src_height * dst_width > dst_height * src_width {
            // The src rectangle's aspect ratio is "taller" than the dst rectangle's; add horizontal padding.
            Self::new_horizontal_padded(src_p1, src_p2, dst_p1, dst_p2, flipped)
        } else {
            // The src rectangle's aspect ratio is "wider" than the dst rectangle's; add vertical padding.
            fn tr(p: Pos2) -> Pos2 {
                Pos2::new(p.y, p.x)
            }
            Self::new_horizontal_padded(tr(src_p1), tr(src_p2), tr(dst_p1), tr(dst_p2), flipped)
                .transpose()
        }
    }

    /// Creates a new `Transform` that maps the rect `(src_p1, src_p2)` inside `(dst_p1, dst_p2)`, adding horizontal padding/letterboxing.
    fn new_horizontal_padded(
        src_p1: Pos2,
        src_p2: Pos2,
        dst_p1: Pos2,
        dst_p2: Pos2,
        flipped: bool,
    ) -> Self {
        let scale_y = (dst_p1.y - dst_p2.y) / (src_p1.y - src_p2.y);
        let offset_y = dst_p1.y - src_p1.y * scale_y;
        let scale_x = scale_y.copysign((src_p2.x - src_p1.x) * (dst_p2.x - dst_p1.x));
        let src_x_middle = (src_p1.x + src_p2.x) / 2.0;
        let dst_x_middle = (dst_p1.x + dst_p2.x) / 2.0;
        let offset_x = dst_x_middle - src_x_middle * scale_x;
        Self {
            scale_x,
            scale_y,
            offset_x,
            offset_y,
            flipped,
        }
    }

    /// Swaps the X and Y components of this `Transform`.
    pub fn transpose(&self) -> Self {
        Self {
            scale_x: self.scale_y,
            scale_y: self.scale_x,
            offset_x: self.offset_y,
            offset_y: self.offset_x,
            flipped: self.flipped,
        }
    }

    /// Returns the inverse `Transform`.
    /// Panics if the transformation is not invertible.
    pub fn inverse(&self) -> Self {
        assert!(self.scale_x != 0.0);
        assert!(self.scale_y != 0.0);
        if self.flipped {
            Self {
                scale_x: self.scale_x.recip(),
                scale_y: self.scale_y.recip(),
                offset_x: -self.offset_x / self.scale_x,
                offset_y: -self.offset_y / self.scale_y,
                flipped: self.flipped,
            }
        } else {
            Self {
                scale_x: self.scale_y.recip(),
                scale_y: self.scale_x.recip(),
                offset_x: -self.offset_y / self.scale_y,
                offset_y: -self.offset_x / self.scale_x,
                flipped: self.flipped,
            }
        }
    }

    /// Applies the transformation to a point.
    pub fn map_point(&self, p: Pos2) -> Pos2 {
        if self.flipped {
            Pos2::new(
                p.x * self.scale_x + self.offset_x,
                p.y * self.scale_y + self.offset_y,
            )
        } else {
            Pos2::new(
                p.y * self.scale_y + self.offset_y,
                p.x * self.scale_x + self.offset_x,
            )
        }
    }

    /// Applies the transformation to a Point<f32>
    pub fn map_point2(&self, p: Point2<f32>) -> Pos2 {
        self.map_point(Pos2::new(p.x, p.y))
    }

    /// Applies a scalar transformation
    pub fn map_dist(&self, x: f32) -> f32 {
        (x * self.scale_x).abs() * x.signum()
    }

    /// Returns the coordinates of the top left and bottom right corners of the [`Wall`] in screen coordinates.
    ///
    /// # Examples
    ///
    /// ```
    /// use rapier2d::na::Point2;
    /// use eframe::egui::Pos2;
    /// use mdrc_pacbot_util::grid::{IntLocation, Wall};
    /// use mdrc_pacbot_util::gui::transforms::Transform;
    ///
    /// let world_to_screen = Transform::new_letterboxed(
    ///     Pos2::new(-1.0, -1.0),
    ///     Pos2::new(32.0, 32.0),
    ///     Pos2::new(0.0, 0.0),
    ///     Pos2::new(330.0, 330.0),
    /// );
    /// let wall = Wall {
    ///     top_left: IntLocation::new(1, 2),
    ///     bottom_right: IntLocation::new(2, 2),
    /// };
    /// let (top_left, bottom_right) = world_to_screen.map_wall(&wall);
    /// assert_eq!(top_left, Pos2::new(30.0, 20.0));
    /// ```
    pub fn map_wall(&self, wall: &Wall) -> (Pos2, Pos2) {
        let top_left = Pos2::new(wall.top_left.x as f32, wall.top_left.y as f32);
        let bottom_right = Pos2::new(wall.bottom_right.x as f32, wall.bottom_right.y as f32);
        (self.map_point(top_left), self.map_point(bottom_right))
    }
}
