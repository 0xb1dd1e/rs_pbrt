//std
use std::sync::Arc;
// pbrt
use core::filter::Filter;
use core::geometry::{Point2f, Vector2f};
use core::paramset::ParamSet;
use core::pbrt::Float;

// see triangle.h

#[derive(Debug, Default, Copy, Clone)]
pub struct TriangleFilter {
    // inherited from Filter (see filter.h)
    pub radius: Vector2f,
    pub inv_radius: Vector2f,
}

impl TriangleFilter {
    pub fn create(ps: &ParamSet) -> Arc<Filter + Sync + Send> {
        let xw: Float = ps.find_one_float(String::from("xwidth"), 2.0);
        let yw: Float = ps.find_one_float(String::from("ywidth"), 2.0);
        let triangle_filter: Arc<Filter + Sync + Send> = Arc::new(TriangleFilter {
            radius: Vector2f { x: xw, y: yw },
            inv_radius: Vector2f {
                x: 1.0 / xw,
                y: 1.0 / yw,
            },
        });
        triangle_filter
    }
}

impl Filter for TriangleFilter {
    fn evaluate(&self, p: Point2f) -> Float {
        (0.0 as Float).max((self.radius.x - p.x.abs()) as Float)
            * (0.0 as Float).max((self.radius.y - p.y.abs()) as Float)
    }
    fn get_radius(&self) -> Vector2f {
        Vector2f {
            x: self.radius.x,
            y: self.radius.y,
        }
    }
}
