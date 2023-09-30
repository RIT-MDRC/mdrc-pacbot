//! Transforms between coordinate systems (such as grid/logical <=> screen pixels).

use egui::Pos2;

use crate::grid::Wall;

/// A 2D transform consisting of per-axis scale and translation.
pub struct Transform {
    scale_x: f32,
    scale_y: f32,
    offset_x: f32,
    offset_y: f32,
}

impl Transform {
    /// Creates a new `Transform` that maps the rect `(src_p1, src_p2)` inside `(dst_p1, dst_p2)`,
    /// adding padding/letterboxing so that the src rect fits inside the dst rect while preserving
    /// its aspect ratio.
    pub fn new_letterboxed(src_p1: Pos2, src_p2: Pos2, dst_p1: Pos2, dst_p2: Pos2) -> Self {
        // Compare the aspect ratios to determine the letterboxing direction.
        let src_width = (src_p1.x - src_p2.x).abs();
        let src_height = (src_p1.y - src_p2.y).abs();
        let dst_width = (dst_p1.x - dst_p2.x).abs();
        let dst_height = (dst_p1.y - dst_p2.y).abs();
        if src_height * dst_width > dst_height * src_width {
            // The src rectangle's aspect ratio is "taller" than the dst rectangle's; add horizontal padding.
            Self::new_horizontal_padded(src_p1, src_p2, dst_p1, dst_p2)
        } else {
            // The src rectangle's aspect ratio is "wider" than the dst rectangle's; add vertical padding.
            fn tr(p: Pos2) -> Pos2 {
                Pos2::new(p.y, p.x)
            }
            Self::new_horizontal_padded(tr(src_p1), tr(src_p2), tr(dst_p1), tr(dst_p2)).transpose()
        }
    }

    /// Creates a new `Transform` that maps the rect `(src_p1, src_p2)` inside `(dst_p1, dst_p2)`, adding horizontal padding/letterboxing.
    fn new_horizontal_padded(src_p1: Pos2, src_p2: Pos2, dst_p1: Pos2, dst_p2: Pos2) -> Self {
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
        }
    }

    /// Swaps the X and Y components of this `Transform`.
    pub fn transpose(&self) -> Self {
        Self {
            scale_x: self.scale_y,
            scale_y: self.scale_x,
            offset_x: self.offset_y,
            offset_y: self.offset_x,
        }
    }

    /// Returns the inverse `Transform`.
    /// Panics if the transformation is not invertible.
    pub fn inverse(&self) -> Self {
        assert!(self.scale_x != 0.0);
        assert!(self.scale_y != 0.0);
        Self {
            scale_x: self.scale_x.recip(),
            scale_y: self.scale_y.recip(),
            offset_x: -self.offset_x / self.scale_x,
            offset_y: -self.offset_y / self.scale_y,
        }
    }

    /// Applies the transformation to a point.
    pub fn map_point(&self, p: Pos2) -> Pos2 {
        Pos2::new(
            p.x * self.scale_x + self.offset_x,
            p.y * self.scale_y + self.offset_y,
        )
    }

    /// Returns the coordinates of the top left and bottom right corners of the [`Wall`] in screen coordinates.
    ///
    /// # Examples
    ///
    /// ```
    /// use rapier2d::na::Point2;
    /// use egui::Pos2;
    /// use mdrc_pacbot_util::grid::Wall;
    /// use mdrc_pacbot_util::gui::transforms::Transform;
    ///
    /// let world_to_screen = Transform::new_letterboxed(
    ///     Pos2::new(-1.0, 32.0),
    ///     Pos2::new(32.0, -1.0),
    ///     Pos2::new(0.0, 0.0),
    ///     Pos2::new(330.0, 330.0),
    /// );
    /// let wall = Wall {
    ///     left_bottom: Point2::new(1.0, 1.0),
    ///     right_top: Point2::new(2.0, 2.0),
    /// };
    /// let (left_bottom, right_top) = world_to_screen.map_wall(&wall);
    /// assert_eq!(left_bottom, Pos2::new(20.0, 310.0));
    /// ```
    pub fn map_wall(&self, wall: &Wall) -> (Pos2, Pos2) {
        let left_bottom = Pos2::new(wall.left_bottom.x as f32, wall.left_bottom.y as f32);
        let right_top = Pos2::new(wall.right_top.x as f32, wall.right_top.y as f32);
        (self.map_point(left_bottom), self.map_point(right_top))
    }
}
