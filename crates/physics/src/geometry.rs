//! Closest-point queries between points, segments and triangles.
//!
//! These are the building blocks of every collision query: a capsule is a segment with a
//! radius, a sphere is a capsule with a zero-length segment, and the world is triangles. The
//! algorithms follow Christer Ericson, *Real-Time Collision Detection* (2005), sections 5.1.5
//! (closest point on triangle) and 5.1.9 (closest points of two segments).

use glam::Vec3;

const PARALLEL_EPSILON: f32 = 1e-9;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Triangle {
    pub a: Vec3,
    pub b: Vec3,
    pub c: Vec3,
}

impl Triangle {
    pub fn new(a: Vec3, b: Vec3, c: Vec3) -> Triangle {
        Triangle { a, b, c }
    }

    /// The unit normal of the counter-clockwise front face (zero for a degenerate triangle).
    pub fn normal(&self) -> Vec3 {
        (self.b - self.a).cross(self.c - self.a).normalize_or_zero()
    }

    pub fn is_degenerate(&self) -> bool {
        (self.b - self.a).cross(self.c - self.a).length_squared() < 1e-12
    }

    pub fn transformed(&self, transform: &glam::Mat4) -> Triangle {
        Triangle {
            a: transform.transform_point3(self.a),
            b: transform.transform_point3(self.b),
            c: transform.transform_point3(self.c),
        }
    }

    pub fn edges(&self) -> [(Vec3, Vec3); 3] {
        [(self.a, self.b), (self.b, self.c), (self.c, self.a)]
    }
}

/// The point of the triangle closest to `point` (Ericson 5.1.5, Voronoi regions).
pub fn closest_point_on_triangle(point: Vec3, triangle: &Triangle) -> Vec3 {
    let Triangle { a, b, c } = *triangle;
    let ab = b - a;
    let ac = c - a;
    let ap = point - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = point - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = point - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let denominator = 1.0 / (va + vb + vc);
    a + ab * (vb * denominator) + ac * (vc * denominator)
}

/// The closest points of segments `p1..q1` and `p2..q2` (Ericson 5.1.9).
pub fn closest_points_between_segments(p1: Vec3, q1: Vec3, p2: Vec3, q2: Vec3) -> (Vec3, Vec3) {
    let d1 = q1 - p1;
    let d2 = q2 - p2;
    let r = p1 - p2;
    let a = d1.length_squared();
    let e = d2.length_squared();
    let f = d2.dot(r);
    let (s, t);
    if a <= PARALLEL_EPSILON && e <= PARALLEL_EPSILON {
        return (p1, p2);
    }
    if a <= PARALLEL_EPSILON {
        s = 0.0;
        t = (f / e).clamp(0.0, 1.0);
    } else {
        let c = d1.dot(r);
        if e <= PARALLEL_EPSILON {
            t = 0.0;
            s = (-c / a).clamp(0.0, 1.0);
        } else {
            let b = d1.dot(d2);
            let denominator = a * e - b * b;
            let mut s_candidate =
                if denominator > PARALLEL_EPSILON { ((b * f - c * e) / denominator).clamp(0.0, 1.0) } else { 0.0 };
            let mut t_candidate = (b * s_candidate + f) / e;
            if t_candidate < 0.0 {
                t_candidate = 0.0;
                s_candidate = (-c / a).clamp(0.0, 1.0);
            } else if t_candidate > 1.0 {
                t_candidate = 1.0;
                s_candidate = ((b - c) / a).clamp(0.0, 1.0);
            }
            s = s_candidate;
            t = t_candidate;
        }
    }
    (p1 + d1 * s, p2 + d2 * t)
}

/// Where segment `start..end` crosses the triangle, if it does.
pub fn segment_triangle_intersection(start: Vec3, end: Vec3, triangle: &Triangle) -> Option<Vec3> {
    // Möller–Trumbore, restricted to the segment's parameter range.
    let direction = end - start;
    let edge1 = triangle.b - triangle.a;
    let edge2 = triangle.c - triangle.a;
    let p = direction.cross(edge2);
    let determinant = edge1.dot(p);
    if determinant.abs() < PARALLEL_EPSILON {
        return None;
    }
    let inverse = 1.0 / determinant;
    let offset = start - triangle.a;
    let u = offset.dot(p) * inverse;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = offset.cross(edge1);
    let v = direction.dot(q) * inverse;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = edge2.dot(q) * inverse;
    (0.0..=1.0).contains(&t).then(|| start + direction * t)
}

/// The closest points between segment `start..end` and a triangle:
/// `(point on segment, point on triangle)`. They coincide when the segment pierces it.
pub fn closest_points_segment_triangle(start: Vec3, end: Vec3, triangle: &Triangle) -> (Vec3, Vec3) {
    if let Some(crossing) = segment_triangle_intersection(start, end, triangle) {
        return (crossing, crossing);
    }
    // Otherwise the closest pair involves a segment endpoint against the face, or the segment
    // against one of the triangle's edges. Take the best of the five candidates.
    let mut best = (start, closest_point_on_triangle(start, triangle));
    let mut best_distance_squared = best.0.distance_squared(best.1);
    let mut consider = |on_segment: Vec3, on_triangle: Vec3| {
        let distance_squared = on_segment.distance_squared(on_triangle);
        if distance_squared < best_distance_squared {
            best_distance_squared = distance_squared;
            best = (on_segment, on_triangle);
        }
    };
    consider(end, closest_point_on_triangle(end, triangle));
    for (edge_start, edge_end) in triangle.edges() {
        let (on_segment, on_edge) = closest_points_between_segments(start, end, edge_start, edge_end);
        consider(on_segment, on_edge);
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn floor_triangle() -> Triangle {
        // In the y = 0 plane, facing +Y (counter-clockwise seen from above).
        Triangle::new(Vec3::new(-1.0, 0.0, 1.0), Vec3::new(1.0, 0.0, 1.0), Vec3::new(0.0, 0.0, -1.0))
    }

    #[test]
    fn floor_triangle_faces_up() {
        assert!(floor_triangle().normal().abs_diff_eq(Vec3::Y, 1e-6));
    }

    #[test]
    fn closest_point_on_triangle_covers_face_edge_and_vertex_regions() {
        let triangle = floor_triangle();
        assert!(closest_point_on_triangle(Vec3::new(0.0, 2.0, 0.0), &triangle).abs_diff_eq(Vec3::ZERO, 1e-6));
        assert!(
            closest_point_on_triangle(Vec3::new(0.0, 1.0, 3.0), &triangle).abs_diff_eq(Vec3::new(0.0, 0.0, 1.0), 1e-6)
        );
        assert!(
            closest_point_on_triangle(Vec3::new(0.0, 0.0, -5.0), &triangle)
                .abs_diff_eq(Vec3::new(0.0, 0.0, -1.0), 1e-6)
        );
    }

    #[test]
    fn segments_closest_points() {
        let (on_first, on_second) = closest_points_between_segments(
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, -1.0),
            Vec3::new(0.0, 1.0, 1.0),
        );
        assert!(on_first.abs_diff_eq(Vec3::ZERO, 1e-6));
        assert!(on_second.abs_diff_eq(Vec3::Y, 1e-6));
        // Parallel segments still give a valid pair at distance 1.
        let (a, b) = closest_points_between_segments(Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Y + Vec3::X);
        assert!((a.distance(b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn segment_triangle_pierce_and_miss() {
        let triangle = floor_triangle();
        let (on_segment, on_triangle) = closest_points_segment_triangle(Vec3::Y, -Vec3::Y, &triangle);
        assert_eq!(on_segment, on_triangle);
        // A vertical segment hovering above: closest is its lower end.
        let (on_segment, on_triangle) =
            closest_points_segment_triangle(Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.0, 2.0, 0.0), &triangle);
        assert!(on_segment.abs_diff_eq(Vec3::new(0.0, 0.5, 0.0), 1e-6));
        assert!(on_triangle.abs_diff_eq(Vec3::ZERO, 1e-6));
        // A horizontal segment beside an edge: the edge-edge case.
        let (on_segment, on_triangle) =
            closest_points_segment_triangle(Vec3::new(-2.0, 0.0, 2.0), Vec3::new(2.0, 0.0, 2.0), &triangle);
        assert!((on_segment.distance(on_triangle) - 1.0).abs() < 1e-5);
    }
}
