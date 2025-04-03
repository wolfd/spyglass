use na::{Point2, Vector2};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: Point2<f64>,
    pub max: Point2<f64>,
}

impl From<&[[f64; 2]; 2]> for Bounds {
    fn from(bounds: &[[f64; 2]; 2]) -> Self {
        Self {
            min: Point2::from(bounds[0]),
            max: Point2::from(bounds[1]),
        }
    }
}

impl Bounds {
    pub fn translate(&self, by: &Vector2<f64>) -> Self {
        Self {
            min: self.min + by,
            max: self.max + by,
        }
    }

    pub fn unit_position_to_plot_position(&self, point: &Point2<f64>) -> Point2<f64> {
        self.min + self.size().component_mul(&point.coords)
    }

    pub fn scale_from_point(&self, point: &Point2<f64>, by: &Vector2<f64>) -> Self {
        let new_min = point + by.component_mul(&(self.min - point));
        let new_max = point + by.component_mul(&(self.max - point));
        Self {
            min: new_min.into(),
            max: new_max.into(),
        }
    }

    pub fn size(&self) -> Vector2<f64> {
        self.max - self.min
    }

    pub fn center(&self) -> Point2<f64> {
        self.min + self.size().scale(0.5)
    }

    // concept: we reorigin the GPU scene whenever we think f32 calculations are going to cause the scene to look bad
    // next steps: we could also rescale the scene so that we can get to really high zoom levels
    pub fn transform_to_origin(&self) -> na::Similarity2<f64> {
        let mut transform = na::Similarity2::identity();
        let center = self.center();
        transform.append_translation_mut(&na::Translation2::new(-center.x, -center.y));
        transform.append_scaling_mut(1.0 / self.size().max());
        transform
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zeros_are_zero() {
        let bounds: Bounds = (&[[-5.0, -5.0], [5.0, 5.0]]).into();
        assert_eq!(bounds.center(), Point2::new(0.0, 0.0)); // sure why not exact
        assert_eq!(bounds.translate(&[0.0, 0.0].into()), bounds);
        assert_eq!(
            bounds.scale_from_point(&[0.0, 0.0].into(), &[1.0, 1.0].into()),
            bounds
        );
    }

    #[test]
    fn scaling_noop() {
        let bounds: Bounds = (&[[-5.0, -5.0], [5.0, 5.0]]).into();
        assert_eq!(
            bounds.scale_from_point(&[-2.5, 2.5].into(), &[1.0, 1.0].into()),
            bounds
        );
        assert_eq!(
            bounds.scale_from_point(&[-5.0, -5.0].into(), &[1.0, 1.0].into()),
            bounds
        );
    }

    #[test]
    fn scaling_by_two() {
        let bounds: Bounds = (&[[-5.0, -5.0], [5.0, 5.0]]).into();
        assert_eq!(
            bounds.scale_from_point(&[0.0, 0.0].into(), &[2.0, 2.0].into()),
            (&[[-10.0, -10.0], [10.0, 10.0]]).into()
        );
        assert_eq!(
            bounds.scale_from_point(&[0.0, 0.0].into(), &[2.0, 1.0].into()),
            (&[[-10.0, -5.0], [10.0, 5.0]]).into()
        );

        assert_eq!(
            bounds.scale_from_point(&[2.5, 2.5].into(), &[2.0, 1.0].into()),
            (&[[-12.5, -5.0], [7.5, 5.0]]).into()
        );
    }

    #[test]
    fn scaling_by_half() {
        let bounds: Bounds = (&[[-4.0, -6.0], [8.0, 12.0]]).into();
        assert_eq!(
            bounds.scale_from_point(&[0.0, 0.0].into(), &[0.5, 0.5].into()),
            (&[[-2.0, -3.0], [4.0, 6.0]]).into()
        );
        assert_eq!(
            bounds.scale_from_point(&[4.0, 6.0].into(), &[0.5, 0.5].into()),
            (&[[0.0, 0.0], [6.0, 9.0]]).into()
        );
    }
    #[test]
    fn scaling_with_negative_factors() {
        let bounds: Bounds = (&[[-2.0, -2.0], [2.0, 2.0]]).into();
        assert_eq!(
            bounds.scale_from_point(&[0.0, 0.0].into(), &[-1.0, -1.0].into()),
            (&[[2.0, 2.0], [-2.0, -2.0]]).into()
        );
    }

    #[test]
    fn non_uniform_scaling() {
        let bounds: Bounds = (&[[-3.0, -1.0], [3.0, 2.0]]).into();
        assert_eq!(
            bounds.scale_from_point(&[0.0, 0.0].into(), &[2.0, 0.5].into()),
            (&[[-6.0, -0.5], [6.0, 1.0]]).into()
        );
    }

    #[test]
    fn scaling_from_non_origin_point() {
        let bounds: Bounds = (&[[-2.0, -2.0], [4.0, 4.0]]).into();
        assert_eq!(
            bounds.scale_from_point(&[2.0, 2.0].into(), &[2.0, 2.0].into()),
            (&[[-6.0, -6.0], [6.0, 6.0]]).into()
        );
    }

    #[test]
    fn scaling_to_collapse() {
        let bounds: Bounds = (&[[-3.0, -3.0], [3.0, 3.0]]).into();
        assert_eq!(
            bounds.scale_from_point(&[0.0, 0.0].into(), &[0.0, 0.0].into()),
            (&[[0.0, 0.0], [0.0, 0.0]]).into()
        );
    }
}
