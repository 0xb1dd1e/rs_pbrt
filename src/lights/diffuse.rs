// std
use std;
use std::f32::consts::PI;
use std::sync::Arc;
// pbrt
use core::geometry::{Normal3f, Point2f, Ray, Vector3f};
use core::geometry::{nrm_abs_dot_vec3, nrm_dot_vec3, vec3_coordinate_system, vec3_normalize};
use core::interaction::{Interaction, InteractionCommon};
use core::light::{AreaLight, Light, LightFlags, VisibilityTester};
use core::medium::{Medium, MediumInterface};
use core::pbrt::{Float, Spectrum};
use core::rng::FLOAT_ONE_MINUS_EPSILON;
use core::sampling::{cosine_hemisphere_pdf, cosine_sample_hemisphere};
use core::scene::Scene;
use core::shape::Shape;
use core::transform::Transform;

// see diffuse.h

pub struct DiffuseAreaLight {
    pub l_emit: Spectrum,
    pub shape: Arc<Shape + Send + Sync>,
    pub two_sided: bool,
    pub area: Float,
    // inherited from class Light (see light.h)
    pub flags: u8,
    pub n_samples: i32,
    pub medium_interface: MediumInterface,
    // light_to_world: Transform,
    // world_to_light: Transform,
}

impl DiffuseAreaLight {
    pub fn new(
        _light_to_world: &Transform,
        medium_interface: &MediumInterface,
        l_emit: &Spectrum,
        n_samples: i32,
        shape: Arc<Shape + Send + Sync>,
        two_sided: bool,
    ) -> Self {
        let area: Float = shape.area();
        let mut inside: Option<Arc<Medium + Send + Sync>> = None;
        let mut outside: Option<Arc<Medium + Send + Sync>> = None;
        if let Some(ref mi_inside) = medium_interface.inside {
            inside = Some(mi_inside.clone());
        }
        if let Some(ref mi_outside) = medium_interface.outside {
            outside = Some(mi_outside.clone());
        }
        DiffuseAreaLight {
            l_emit: *l_emit,
            shape: shape,
            two_sided: two_sided,
            area: area,
            // inherited from class Light (see light.h)
            flags: LightFlags::Area as u8,
            n_samples: std::cmp::max(1_i32, n_samples),
            medium_interface: MediumInterface {
                inside: inside,
                outside: outside,
            },
            // light_to_world: *light_to_world,
            // world_to_light: Transform::inverse(*light_to_world),
        }
    }
}

impl Light for DiffuseAreaLight {
    fn sample_li(
        &self,
        iref: &InteractionCommon,
        u: &Point2f,
        wi: &mut Vector3f,
        pdf: &mut Float,
        vis: &mut VisibilityTester,
    ) -> Spectrum {
        // TODO: ProfilePhase _(Prof::LightSample);
        let p_shape: InteractionCommon = self.shape.sample_with_ref_point(&iref, &*u, pdf);
        // TODO: iref.mediumInterface = mediumInterface;
        if *pdf == 0.0 as Float || (p_shape.p - iref.p).length_squared() == 0.0 as Float {
            *pdf = 0.0 as Float;
            return Spectrum::default();
        }
        let new_wi: Vector3f = vec3_normalize(&(p_shape.p - iref.p));
        *wi = new_wi;
        vis.p0 = InteractionCommon {
            p: iref.p,
            time: iref.time,
            p_error: iref.p_error,
            wo: iref.wo,
            n: iref.n,
            medium_interface: None,
        };
        vis.p1 = InteractionCommon {
            p: p_shape.p,
            time: p_shape.time,
            p_error: p_shape.p_error,
            wo: p_shape.wo,
            n: p_shape.n,
            medium_interface: None,
        };
        self.l(&p_shape, &-new_wi)
    }
    fn power(&self) -> Spectrum {
        // return (twoSided ? 2 : 1) * Lemit * area * Pi;
        let factor: Float;
        if self.two_sided {
            factor = 2.0 as Float;
        } else {
            factor = 1.0 as Float;
        }
        self.l_emit * factor * self.area * PI
    }
    fn preprocess(&self, _scene: &Scene) {
        // TODO?
    }
    fn le(&self, _ray: &mut Ray) -> Spectrum {
        Spectrum::default()
    }
    fn pdf_li(&self, iref: &Interaction, wi: Vector3f) -> Float {
        // TODO: ProfilePhase _(Prof::LightPdf);
        self.shape.pdf_with_ref_point(iref, &wi)
    }
    fn sample_le(
        &self,
        u1: &Point2f,
        u2: &Point2f,
        _time: Float,
        ray: &mut Ray,
        n_light: &mut Normal3f,
        pdf_pos: &mut Float,
        pdf_dir: &mut Float,
    ) -> Spectrum {
        // TODO: ProfilePhase _(Prof::LightSample);

        // sample a point on the area light's _Shape_, _p_shape_
        let ic: InteractionCommon = self.shape.sample(u1, pdf_pos);
        // TODO: p_shape.mediumInterface = mediumInterface;
        *n_light = ic.n;
        // sample a cosine-weighted outgoing direction _w_ for area light
        let mut w: Vector3f;
        if self.two_sided {
            let mut u: Point2f = Point2f { x: u2.x, y: u2.y };
            // choose a side to sample and then remap u[0] to [0,1]
            // before applying cosine-weighted hemisphere sampling for
            // the chosen side.
            if u[0] < 0.5 as Float {
                u[0] = (u[0] * 2.0 as Float).min(FLOAT_ONE_MINUS_EPSILON);
                w = cosine_sample_hemisphere(&u);
            } else {
                u[0] = ((u[0] - 0.5 as Float) * 2.0 as Float).min(FLOAT_ONE_MINUS_EPSILON);
                w = cosine_sample_hemisphere(&u);
                w.z *= -1.0 as Float;
            }
            *pdf_dir = 0.5 as Float * cosine_hemisphere_pdf(w.z.abs());
        } else {
            w = cosine_sample_hemisphere(u2);
            *pdf_dir = cosine_hemisphere_pdf(w.z);
        }
        let n: Vector3f = Vector3f::from(ic.n);
        let mut v1: Vector3f = Vector3f::default();
        let mut v2: Vector3f = Vector3f::default();
        vec3_coordinate_system(&n, &mut v1, &mut v2);
        w = v1 * w.x + v2 * w.y + n * w.z;
        *ray = ic.spawn_ray(&w);
        self.l(&ic, &w)
    }
    fn pdf_le(&self, ray: &Ray, n: &Normal3f, pdf_pos: &mut Float, pdf_dir: &mut Float) {
        *pdf_pos = self.shape.pdf(&InteractionCommon::default());
        if self.two_sided {
            *pdf_dir = 0.5 as Float * cosine_hemisphere_pdf(nrm_abs_dot_vec3(&n, &ray.d));
        } else {
            *pdf_dir = cosine_hemisphere_pdf(nrm_dot_vec3(&n, &ray.d));
        }
    }
    fn get_flags(&self) -> u8 {
        self.flags
    }
    fn get_n_samples(&self) -> i32 {
        self.n_samples
    }
}

impl AreaLight for DiffuseAreaLight {
    fn l(&self, intr: &InteractionCommon, w: &Vector3f) -> Spectrum {
        if self.two_sided || nrm_dot_vec3(&intr.n, &w) > 0.0 as Float {
            self.l_emit
        } else {
            Spectrum::new(0.0 as Float)
        }
    }
}
