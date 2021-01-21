// Boost.Polygon library detail/voronoi_predicates.hpp header file

//          Copyright Andrii Sydorchuk 2010-2012.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE_1_0.txt or copy at
//          http://www.boost.org/LICENSE_1_0.txt)

// See http://www.boost.org for updates, documentation, and revision history.

// Ported from C++ boost 1.74.0 to Rust in 2020 by Eadf (github.com/eadf)

mod tests;

use super::voronoi_beachline as VB;
use super::voronoi_circleevent as VC;
use super::voronoi_robust_fpt as VR;
use super::voronoi_siteevent as VSE;
use super::voronoi_structures as VS;
use super::voronoi_ctypes::UlpComparison;
use super::TypeConverter as TCC;
use super::TypeCheckF as TCF;
use super::TypeCheckI as TCI;
use super::TypeConverter as TC;
use super::{BigFloatType, BigIntType, BoostInputType, BoostOutputType};
use geo::Point;
use num::FromPrimitive;
use num::ToPrimitive;
use num::{BigInt, Float, NumCast, PrimInt, Zero};
//use num_traits;
use ordered_float::OrderedFloat;
use std::cmp::Ordering;
use std::convert::TryInto;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::Neg;

// TODO: how to make these generic?
const ULPS: u64 = 64;
const ULPSX2: u64 = 128;

#[derive(Debug, PartialEq, Eq)]
enum UlpCmp {
    LESS,
    EQUAL,
    MORE,
}

#[inline(always)]
fn is_neg(number: &BigInt) -> bool {
    number < &BigInt::zero()
}

#[inline(always)]
fn is_pos(number: &BigInt) -> bool {
    number > &BigInt::zero()
}

#[inline(always)]
fn is_zero(number: &BigInt) -> bool {
    number.is_zero()
}

/// Predicate utilities. Operates with the coordinate types that could
/// be converted to the 32-bit signed integer without precision loss.
/// Todo! give this a lookover
#[derive(Default)]
pub struct VoronoiPredicates<I1, F1, I2, F2>
where
    I1: BoostInputType + Neg<Output = I1>,
    F1: BoostOutputType + Neg<Output = F1>,
    I2: BoostInputType + Neg<Output = I2>,
    F2: BoostOutputType + Neg<Output = F2>,
{
    _pdo: PhantomData<F1>,
    _pdi: PhantomData<I1>,
    _pdbi: PhantomData<I2>,
    _pdbf: PhantomData<F2>,
}

impl<I1, F1, I2, F2> VoronoiPredicates<I1, F1, I2, F2>
where
    I1: BoostInputType + Neg<Output = I1>,
    F1: BoostOutputType + Neg<Output = F1>,
    I2: BoostInputType + Neg<Output = I2>,
    F2: BoostOutputType + Neg<Output = F2>,
{
    #[inline(always)]
    pub(crate) fn is_vertical_1(site: &VSE::SiteEvent<I1, F1, I2, F2>) -> bool {
        Self::is_vertical_2(site.point0(), site.point1())
    }

    #[inline(always)]
    pub(crate) fn is_vertical_2(point1: &Point<I1>, point2: &Point<I1>) -> bool {
        point1.x() == point2.x()
    }

    /// Compute robust cross_product: a1 * b2 - b1 * a2.
    /// It was mathematically proven that the result is correct
    /// with epsilon relative error equal to 1EPS.
    #[inline(always)]
    pub(crate) fn robust_cross_product(a1_: I1, b1_: I1, a2_: I1, b2_: I1) -> F2 {
        let i1_to_i2 = TC::<I1, F1, I2, F2>::i1_to_i2;

        let a1: I2 = i1_to_i2(if TCI::<I1>::is_neg(a1_) { -a1_ } else { a1_ });
        let b1: I2 = i1_to_i2(if TCI::<I1>::is_neg(b1_) { -b1_ } else { b1_ });
        let a2: I2 = i1_to_i2(if TCI::<I1>::is_neg(a2_) { -a2_ } else { a2_ });
        let b2: I2 = i1_to_i2(if TCI::<I1>::is_neg(b2_) { -b2_ } else { b2_ });
        robust_cross_product_f::<I2, F2>(a1, b1, a2, b2)
    }

    /// Compute robust cross_product: a1 * b2 - b1 * a2.
    /// It was mathematically proven that the result is correct
    /// with epsilon relative error equal to 1EPS.
    /// TODO: this is supposed to use u32 if I1==i32
    #[inline(always)]
    pub(crate) fn robust_cross_product_2i(a1: I2, b1: I2, a2: I2, b2: I2) -> F2 {
        robust_cross_product_f::<I2, F2>(a1, b1, a2, b2)
    }

    #[inline(always)]
    pub(crate) fn ulps() -> u64 {
        // todo figure out how to cache this
        if std::mem::size_of::<F2>() > 4 {
            ULPSX2
        } else {
            ULPS
        }
    }
}

#[inline]
fn robust_cross_product_f<T, U>(a1_: T, b1_: T, a2_: T, b2_: T) -> U
where
    T: PrimInt
        + PartialOrd
        + PartialEq
        + NumCast
        + Copy
        + Clone
        + Display
        + Default
        + Debug
        + Zero
        + Neg<Output = T>,
    U: Float
        + PartialOrd
        + PartialEq
        + NumCast
        + Copy
        + Clone
        + Display
        + Default
        + Debug
        + Zero
        + Neg<Output = U>,
{
    let a1: T = if a1_ < T::zero() { -a1_ } else { a1_ };
    let b1: T = if b1_ < T::zero() { -b1_ } else { b1_ };
    let a2: T = if a2_ < T::zero() { -a2_ } else { a2_ };
    let b2: T = if b2_ < T::zero() { -b2_ } else { b2_ };

    let l: T = a1 * b2;
    let r: T = b1 * a2;

    if (a1_ < T::zero()) ^ (b2_ < T::zero()) {
        return if (a2_ < T::zero()) ^ (b1_ < T::zero()) {
            if l > r {
                -num::cast::<T, U>(l - r).unwrap()
            } else {
                num::cast::<T, U>(r - l).unwrap()
            }
        } else {
            -num::cast::<T, U>(l + r).unwrap()
        };
    }
    if (a2_ < T::zero()) ^ (b1_ < T::zero()) {
        return num::cast::<T, U>(l + r).unwrap();
    }
    if l < r {
        -num::cast::<T, U>(r - l).unwrap()
    } else {
        num::cast::<T, U>(l - r).unwrap()
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Orientation {
    RIGHT,     // = -1,
    COLLINEAR, // = 0,
    LEFT,      // = 1
}

#[derive(Default)]
pub struct OrientationTest<I1, F1, I2, F2>
where
    I1: BoostInputType + Neg<Output = I1>,
    F1: BoostOutputType + Neg<Output = F1>,
    I2: BoostInputType + Neg<Output = I2>,
    F2: BoostOutputType + Neg<Output = F2>,
{
    _pdo: PhantomData<F1>,
    _pdi: PhantomData<I1>,
    _pdbi: PhantomData<I2>,
    _pdbf: PhantomData<F2>,
}

impl<I1, F1, I2, F2> OrientationTest<I1, F1, I2, F2>
where
    I1: BoostInputType + Neg<Output = I1>,
    F1: BoostOutputType + Neg<Output = F1>,
    I2: BoostInputType + Neg<Output = I2>,
    F2: BoostOutputType + Neg<Output = F2>,
{
    /// Value is a determinant of two vectors (e.g. x1 * y2 - x2 * y1).
    /// Return orientation based on the sign of the determinant.
    fn eval_i(value: I1) -> Orientation {
        if TCI::<I1>::is_zero(value) {
            return Orientation::COLLINEAR;
        }
        match TCI::<I1>::is_neg(value) {
            true => Orientation::RIGHT,
            false => Orientation::LEFT,
        }
    }

    /// Value is a determinant of two vectors (e.g. x1 * y2 - x2 * y1).
    /// Return orientation based on the sign of the determinant.
    fn eval_f(value: F2) -> Orientation {
        if value.is_zero() {
            return Orientation::COLLINEAR;
        }
        match value.is_sign_negative() {
            true => Orientation::RIGHT,
            false => Orientation::LEFT,
        }
    }

    /// Value is a determinant of two vectors (e.g. x1 * y2 - x2 * y1).
    /// Return orientation based on the sign of the determinant.
    fn eval_bf(value: F2) -> Orientation {
        if value.is_zero() {
            return Orientation::COLLINEAR;
        }
        match value.is_sign_negative() {
            true => Orientation::RIGHT,
            false => Orientation::LEFT,
        }
    }

    fn eval_3(point1: &Point<I1>, point2: &Point<I1>, point3: &Point<I1>) -> Orientation {
        let i1_to_i2 = TC::<I1, F1, I2, F2>::i1_to_i2;
        let dx1: I2 = i1_to_i2(point1.x()) - i1_to_i2(point2.x());
        let dx2: I2 = i1_to_i2(point2.x()) - i1_to_i2(point3.x());
        let dy1: I2 = i1_to_i2(point1.y()) - i1_to_i2(point2.y());
        let dy2: I2 = i1_to_i2(point2.y()) - i1_to_i2(point3.y());
        let cp: F2 =
            VoronoiPredicates::<I1, F1, I2, F2>::robust_cross_product_2i(dx1, dy1, dx2, dy2);
        Self::eval_bf(cp)
    }

    fn eval_4(dif_x1_: I2, dif_y1_: I2, dif_x2_: I2, dif_y2_: I2) -> Orientation {
        let a = VoronoiPredicates::<I1, F1, I2, F2>::robust_cross_product_2i(
            dif_x1_, dif_y1_, dif_x2_, dif_y2_,
        );
        Self::eval_bf(a)
    }
}

#[derive(Default)]
pub struct PointComparisonPredicate<I1>
where
    I1: BoostInputType + Neg<Output = I1>,
{
    _pdi: PhantomData<I1>,
}

impl<I1> PointComparisonPredicate<I1>
where
    I1: BoostInputType + Neg<Output = I1>,
{
    pub(crate) fn point_comparison_predicate(lhs: &Point<I1>, rhs: &Point<I1>) -> bool {
        if lhs.x() == rhs.x() {
            lhs.y() < rhs.y()
        } else {
            lhs.x() < rhs.x()
        }
    }
}

#[derive(Default)]
pub struct EventComparisonPredicate<I1, F1, I2, F2>
where
    I1: BoostInputType + Neg<Output = I1>,
    F1: BoostOutputType + Neg<Output = F1>,
    I2: BoostInputType + Neg<Output = I2>,
    F2: BoostOutputType + Neg<Output = F2>,
{
    _pdo: PhantomData<F1>,
    _pdi: PhantomData<I1>,
    _pdbi: PhantomData<I2>,
    _pdbf: PhantomData<F2>,
}

impl<I1, F1, I2, F2> EventComparisonPredicate<I1, F1, I2, F2>
where
    I1: BoostInputType + Neg<Output = I1>,
    F1: BoostOutputType + Neg<Output = F1>,
    I2: BoostInputType + Neg<Output = I2>,
    F2: BoostOutputType + Neg<Output = F2>,
{
    /// boolean predicate between two sites (bool int int)
    pub(crate) fn event_comparison_predicate_bii(
        lhs: &VSE::SiteEvent<I1, F1, I2, F2>,
        rhs: &VSE::SiteEvent<I1, F1, I2, F2>,
    ) -> bool {
        if lhs.x0() != rhs.x0() {
            return lhs.x0() < rhs.x0();
        }
        if !lhs.is_segment() {
            if !rhs.is_segment() {
                return lhs.y0() < rhs.y0();
            }
            if VoronoiPredicates::<I1, F1, I2, F2>::is_vertical_2(&rhs.point0_, &rhs.point1_) {
                return lhs.y0() <= rhs.y0();
            }
            true
        } else {
            if VoronoiPredicates::<I1, F1, I2, F2>::is_vertical_2(&rhs.point0_, &rhs.point1_) {
                if VoronoiPredicates::<I1, F1, I2, F2>::is_vertical_2(&lhs.point0_, &lhs.point1_) {
                    return lhs.y0() < rhs.y0();
                }
                return false;
            }
            if VoronoiPredicates::<I1, F1, I2, F2>::is_vertical_2(&lhs.point0_, &lhs.point1_) {
                return true;
            }
            if lhs.y0() != rhs.y0() {
                return lhs.y0() < rhs.y0();
            }
            return OrientationTest::<I1, F1, I2, F2>::eval_3(
                &lhs.point1(),
                &lhs.point0(),
                &rhs.point1(),
            ) == Orientation::LEFT;
        }
    }

    /// Ordering predicate between two sites (int int)
    pub(crate) fn event_comparison_predicate_ii(
        lhs: &VSE::SiteEvent<I1, F1, I2, F2>,
        rhs: &VSE::SiteEvent<I1, F1, I2, F2>,
    ) -> std::cmp::Ordering {
        if Self::event_comparison_predicate_bii(lhs, rhs) {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    }

    ///
    /// boolean predicate between site and circle (integer<->float)
    ///
    pub(crate) fn event_comparison_predicate_bif(
        lhs: &VSE::SiteEvent<I1, F1, I2, F2>,
        rhs: &VC::CircleEvent<F2>,
    ) -> bool {
        /* typename ulp_cmp_type::Result xCmp =
         *     ulp_cmp(VoronoiPredicates::<I1, F1, I2, F2>::to_fpt(lhs.x0()), VoronoiPredicates::<I1, F1, I2, F2>::to_fpt(rhs.lower_x()), ULPS);
         * return xCmp == ulp_cmp_type::LESS;
         */

        let lhs = TC::<I1, F1, I2, F2>::i1_to_f64(lhs.x0());
        let rhs = TC::<I1, F1, I2, F2>::f2_to_f64(rhs.lower_x().into_inner());
        let ulps = VoronoiPredicates::<I1,F1,I2,F2>::ulps();

        UlpComparison::ulp_comparison(lhs, rhs,  ulps) == Ordering::Less
    }

    pub(crate) fn event_comparison_predicate_if(
        lhs: &VSE::SiteEvent<I1, F1, I2, F2>,
        rhs: &VC::CircleEvent<F2>,
    ) -> std::cmp::Ordering {
        if Self::event_comparison_predicate_bif(lhs, rhs) {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    }
}

/// Represents the result of the epsilon robust predicate. If the
/// result is undefined some further processing is usually required.
#[derive(Debug, PartialEq, Eq)]
enum KPredicateResult {
    LESS,      // = -1,
    UNDEFINED, // = 0,
    MORE,      // = 1
}

pub struct DistancePredicate<I1, F1, I2, F2>
where
    I1: BoostInputType + Neg<Output = I1>,
    F1: BoostOutputType + Neg<Output = F1>,
    I2: BigIntType + Neg<Output = I2>,
    F2: BigFloatType + Neg<Output = F2>,
{
    _pdo: PhantomData<F1>,
    _pdi: PhantomData<I1>,
    _pdbi: PhantomData<I2>,
    _pdbf: PhantomData<F2>,
}

impl<I1, F1, I2, F2> DistancePredicate<I1, F1, I2, F2>
where
    I1: BoostInputType + Neg<Output = I1>,
    F1: BoostOutputType + Neg<Output = F1>,
    I2: BigIntType + Neg<Output = I2>,
    F2: BigFloatType + Neg<Output = F2>,
{
    pub(crate) fn distance_predicate_debug(
        left_site: &VSE::SiteEvent<I1, F1, I2, F2>,
        right_site: &VSE::SiteEvent<I1, F1, I2, F2>,
        new_point: &Point<I1>,
    ) -> bool {
        let rv = Self::distance_predicate(left_site, right_site, new_point);
        println!(
            "distance_predicate: L:{} R:{} K:{:?}=={}",
            left_site, right_site, new_point, rv
        );
        rv
    }
    /// Returns true if a horizontal line going through a new site intersects
    /// right arc at first, else returns false. If horizontal line goes
    /// through intersection point of the given two arcs returns false also.
    pub(crate) fn distance_predicate(
        left_site: &VSE::SiteEvent<I1, F1, I2, F2>,
        right_site: &VSE::SiteEvent<I1, F1, I2, F2>,
        new_point: &Point<I1>,
    ) -> bool {
        //dbg!(&left_site, &right_site, &new_point);

        if !left_site.is_segment() {
            if !right_site.is_segment() {
                Self::pp(left_site, right_site, new_point)
            } else {
                Self::ps(left_site, right_site, new_point, false)
            }
        } else if !right_site.is_segment() {
            Self::ps(right_site, left_site, new_point, true)
        } else {
            Self::ss(left_site, right_site, new_point)
        }
    }

    //private:

    pub fn pp_debug(
        left_site: &VSE::SiteEvent<I1, F1, I2, F2>,
        right_site: &VSE::SiteEvent<I1, F1, I2, F2>,
        new_point: &Point<I1>,
    ) -> bool {
        let rv = Self::pp(left_site, right_site, new_point);
        println!(
            " pp(L:{} R:{} K:{:?})=={}",
            left_site, right_site, new_point, rv
        );
        rv
    }

    /// Robust predicate, avoids using high-precision libraries.
    /// Returns true if a horizontal line going through the new point site
    /// intersects right arc at first, else returns false. If horizontal line
    /// goes through intersection point of the given two arcs returns false.
    fn pp(
        left_site: &VSE::SiteEvent<I1, F1, I2, F2>,
        right_site: &VSE::SiteEvent<I1, F1, I2, F2>,
        new_point: &Point<I1>,
    ) -> bool {
        let left_point: &Point<I1> = left_site.point0();
        let right_point: &Point<I1> = right_site.point0();
        let i1_to_i2 = TC::<I1, F1, I2, F2>::i1_to_i2;
        //dbg!(&left_site, &right_site, &new_point);
        //dbg!(left_point.x(), left_point.y());
        //dbg!(right_point.x(), right_point.y());

        #[allow(clippy::comparison_chain)] // todo fix clippy
        if left_point.x() > right_point.x() {
            if new_point.y() <= left_point.y() {
                return false;
            }
        } else if left_point.x() < right_point.x() {
            if new_point.y() >= right_point.y() {
                return true;
            }
        } else {
            return i1_to_i2(left_point.y()) + i1_to_i2(right_point.y())
                < i1_to_i2(new_point.y()) * TCI::<I2>::two();
        }

        let dist1 = Self::find_distance_to_point_arc(left_site, new_point);
        let dist2 = Self::find_distance_to_point_arc(right_site, new_point);

        // The undefined ulp range is equal to 3EPS + 3EPS <= 6ULP.
        dist1 < dist2
    }

    pub fn ps_debug(
        left_site: &VSE::SiteEvent<I1, F1, I2, F2>,
        right_site: &VSE::SiteEvent<I1, F1, I2, F2>,
        new_point: &Point<I1>,
        reverse_order: bool,
    ) -> bool {
        println!(
            " ps(L:{}, R:{}, K:{:?}, {})==?",
            left_site, right_site, new_point, reverse_order
        );
        let rv = Self::ps(left_site, right_site, new_point, reverse_order);
        println!(
            " ps(L:{}, R:{}, K:{:?}, {})=={}",
            left_site, right_site, new_point, reverse_order, rv
        );
        rv
    }

    fn ps(
        left_site: &VSE::SiteEvent<I1, F1, I2, F2>,
        right_site: &VSE::SiteEvent<I1, F1, I2, F2>,
        new_point: &Point<I1>,
        reverse_order: bool,
    ) -> bool {
        let fast_res = Self::fast_ps(left_site, right_site, new_point, reverse_order);
        if fast_res != KPredicateResult::UNDEFINED {
            return fast_res == KPredicateResult::LESS;
        }

        let dist1 = Self::find_distance_to_point_arc(left_site, new_point);
        let dist2 = Self::find_distance_to_segment_arc(right_site, new_point);

        // The undefined ulp range is equal to 3EPS + 7EPS <= 10ULP.
        reverse_order ^ (dist1 < dist2)
    }

    pub fn ss_debug(
        left_site: &VSE::SiteEvent<I1, F1, I2, F2>,
        right_site: &VSE::SiteEvent<I1, F1, I2, F2>,
        new_point: &Point<I1>,
    ) -> bool {
        let rv = Self::ss(left_site, right_site, new_point);
        println!(
            " ss(L:{} R:{} K:{:?})=={}",
            left_site, right_site, new_point, rv
        );
        rv
    }

    fn ss(
        left_site: &VSE::SiteEvent<I1, F1, I2, F2>,
        right_site: &VSE::SiteEvent<I1, F1, I2, F2>,
        new_point: &Point<I1>,
    ) -> bool {
        // Handle temporary segment sites.
        if left_site.sorted_index() == right_site.sorted_index() {
            return OrientationTest::<I1, F1, I2, F2>::eval_3(
                left_site.point0(),
                left_site.point1(),
                new_point,
            ) == Orientation::LEFT;
        }

        let dist1 = Self::find_distance_to_segment_arc(left_site, new_point);
        let dist2 = Self::find_distance_to_segment_arc(right_site, new_point);

        // The undefined ulp range is equal to 7EPS + 7EPS <= 14ULP.
        dist1 < dist2
    }

    fn find_distance_to_point_arc(site: &VSE::SiteEvent<I1, F1, I2, F2>, point: &Point<I1>) -> F2 {
        let dx =
            TC::<I1, F1, I2, F2>::i1_to_f2(site.x()) - TC::<I1, F1, I2, F2>::i1_to_f2(point.x());
        let dy =
            TC::<I1, F1, I2, F2>::i1_to_f2(site.y()) - TC::<I1, F1, I2, F2>::i1_to_f2(point.y());
        // The relative error is at most 3EPS.
        (dx * dx + dy * dy) / (dx * TC::<I1, F1, I2, F2>::f32_to_f2(2.0))
    }

    fn find_distance_to_segment_arc(
        site: &VSE::SiteEvent<I1, F1, I2, F2>,
        point: &Point<I1>,
    ) -> F2 {
        let i1_to_i2 = TC::<I1, F1, I2, F2>::i1_to_i2;
        if VoronoiPredicates::<I1, F1, I2, F2>::is_vertical_1(site) {
            (TC::<I1, F1, I2, F2>::i1_to_f2(site.x()) - TC::<I1, F1, I2, F2>::i1_to_f2(point.x()))
                * TCF::<F2>::half()
        } else {
            let segment0: &Point<I1> = site.point0();
            let segment1: &Point<I1> = site.point1();
            let a1: F2 = TC::<I1, F1, I2, F2>::i1_to_f2(segment1.x())
                - TC::<I1, F1, I2, F2>::i1_to_f2(segment0.x());
            let b1: F2 = TC::<I1, F1, I2, F2>::i1_to_f2(segment1.y())
                - TC::<I1, F1, I2, F2>::i1_to_f2(segment0.y());
            let mut k: F2 = (a1 * a1 + b1 * b1).sqrt();
            // Avoid subtraction while computing k.
            if !TCF::<F2>::is_neg(b1) {
                k = TCF::<F2>::one() / (b1 + k);
            } else {
                k = (k - b1) / (a1 * a1);
            }
            // The relative error is at most 7EPS.
            k * VoronoiPredicates::<I1, F1, I2, F2>::robust_cross_product_2i(
                i1_to_i2(segment1.x()) - i1_to_i2(segment0.x()),
                i1_to_i2(segment1.y()) - i1_to_i2(segment0.y()),
                i1_to_i2(point.x()) - i1_to_i2(segment0.x()),
                i1_to_i2(point.y()) - i1_to_i2(segment0.y()),
            )
        }
    }

    fn fast_ps(
        left_site: &VSE::SiteEvent<I1, F1, I2, F2>,
        right_site: &VSE::SiteEvent<I1, F1, I2, F2>,
        new_point: &Point<I1>,
        reverse_order: bool,
    ) -> KPredicateResult {
        let i1_to_f2 = TC::<I1, F1, I2, F2>::i1_to_f2;
        let i1_to_i2 = TC::<I1, F1, I2, F2>::i1_to_i2;

        let site_point: &Point<I1> = left_site.point0();
        let segment_start: &Point<I1> = right_site.point0();
        let segment_end: &Point<I1> = right_site.point1();
        let eval: Orientation =
            OrientationTest::<I1, F1, I2, F2>::eval_3(segment_start, segment_end, new_point);
        if eval != Orientation::RIGHT {
            return if !right_site.is_inverse() {
                KPredicateResult::LESS
            } else {
                KPredicateResult::MORE
            };
        }

        let dif_x = i1_to_f2(new_point.x()) - i1_to_f2(site_point.x());
        let dif_y = i1_to_f2(new_point.y()) - i1_to_f2(site_point.y());
        let a = i1_to_f2(segment_end.x()) - i1_to_f2(segment_start.x());
        let b = i1_to_f2(segment_end.y()) - i1_to_f2(segment_start.y());

        if VoronoiPredicates::<I1, F1, I2, F2>::is_vertical_1(right_site) {
            if new_point.y() < site_point.y() && !reverse_order {
                return KPredicateResult::MORE;
            } else if new_point.y() > site_point.y() && reverse_order {
                return KPredicateResult::LESS;
            }
            return KPredicateResult::UNDEFINED;
        } else {
            let orientation = OrientationTest::<I1, F1, I2, F2>::eval_4(
                i1_to_i2(segment_end.x()) - i1_to_i2(segment_start.x()),
                i1_to_i2(segment_end.y()) - i1_to_i2(segment_start.y()),
                i1_to_i2(new_point.x()) - i1_to_i2(site_point.x()),
                i1_to_i2(new_point.y()) - i1_to_i2(site_point.y()),
            );
            if orientation == Orientation::LEFT {
                if !right_site.is_inverse() {
                    return if reverse_order {
                        KPredicateResult::LESS
                    } else {
                        KPredicateResult::UNDEFINED
                    };
                }
                return if reverse_order {
                    KPredicateResult::UNDEFINED
                } else {
                    KPredicateResult::MORE
                };
            }
        }

        let fast_left_expr = a * (dif_y + dif_x) * (dif_y - dif_x);
        let fast_right_expr = (TCF::<F2>::two() * b) * dif_x * dif_y;

        //let epsilon = F1::default_epsilon();
        let expr_cmp = if fast_left_expr > fast_right_expr {
            fast_left_expr - fast_right_expr
        } else {
            fast_right_expr - fast_left_expr
        } > TCF::<F2>::epsilon();

        //dbg!(fast_left_expr);
        //dbg!(fast_right_expr);
        //dbg!(expr_cmp);
        // rust expr_cmp === c++ (expr_cmp != ulp_cmp_type::EQUAL)
        return if expr_cmp {
            if (fast_left_expr > fast_right_expr) ^ reverse_order {
                if reverse_order {
                    KPredicateResult::LESS
                } else {
                    KPredicateResult::MORE
                }
            } else {
                KPredicateResult::UNDEFINED
            }
        } else {
            KPredicateResult::UNDEFINED
        };

        /* TODO! fix some ulps
        let expr_cmp = fast_left_expr.ulps(&fast_right_expr).cmp(4); //ulp_cmp(fast_left_expr, fast_right_expr, 4);

        if expr_cmp != UlpCmp::EQUAL {
            if (expr_cmp == UlpCmp::MORE) ^ reverse_order {
                return if reverse_order {
                    KPredicateResult::LESS
                } else {
                    KPredicateResult::MORE
                };
            }
            return KPredicateResult::UNDEFINED;
        }*/
        //        return KPredicateResult::UNDEFINED;
    }

    //    private:
    //    ulp_cmp_type ulp_cmp;
    //    to_fpt_converter to_fpt;
}

pub struct NodeComparisonPredicate<I1, F1, I2, F2>
where
    I1: BoostInputType + Neg<Output = I1>,
    F1: BoostOutputType + Neg<Output = F1>,
    I2: BoostInputType + Neg<Output = I2>,
    F2: BoostOutputType + Neg<Output = F2>,
{
    _pdo: PhantomData<F1>,
    _pdi: PhantomData<I1>,
    _pdbi: PhantomData<I2>,
    _pdbf: PhantomData<F2>,
}

impl<I1, F1, I2, F2> NodeComparisonPredicate<I1, F1, I2, F2>
where
    I1: BoostInputType + Neg<Output = I1>,
    F1: BoostOutputType + Neg<Output = F1>,
    I2: BoostInputType + Neg<Output = I2>,
    F2: BoostOutputType + Neg<Output = F2>,
{
    pub fn node_comparison_predicate_debug(
        node1: &VB::BeachLineNodeKey<I1, F1, I2, F2>,
        node2: &VB::BeachLineNodeKey<I1, F1, I2, F2>,
    ) -> bool {
        let rv = Self::node_comparison_predicate(node1, node2);
        println!(
            "node_comparison_predicate(node1={:?}, node2={:?}), rv={}",
            node1, node2, rv
        );
        rv
    }

    /// Compares nodes in the balanced binary search tree. Nodes are
    /// compared based on the y coordinates of the arcs intersection points.
    /// Nodes with less y coordinate of the intersection point go first.
    /// Comparison is only called during the new site events processing.
    /// That's why one of the nodes will always lie on the sweepline and may
    /// be represented as a straight horizontal line.
    pub fn node_comparison_predicate(
        node1: &VB::BeachLineNodeKey<I1, F1, I2, F2>,
        node2: &VB::BeachLineNodeKey<I1, F1, I2, F2>,
    ) -> bool {
        // Get x coordinate of the rightmost site from both nodes.
        let site1: &VSE::SiteEvent<I1, F1, I2, F2> =
            NodeComparisonPredicate::<I1, F1, I2, F2>::get_comparison_site(node1);
        let site2: &VSE::SiteEvent<I1, F1, I2, F2> =
            NodeComparisonPredicate::<I1, F1, I2, F2>::get_comparison_site(node2);
        let point1: &Point<I1> =
            NodeComparisonPredicate::<I1, F1, I2, F2>::get_comparison_point(site1);
        let point2: &Point<I1> =
            NodeComparisonPredicate::<I1, F1, I2, F2>::get_comparison_point(site2);

        if point1.x() < point2.x() {
            // The second node contains a new site.
            return DistancePredicate::<I1, F1, I2, F2>::distance_predicate(
                node1.left_site(),
                node1.right_site(),
                point2,
            );
        } else if point1.x() > point2.x() {
            // The first node contains a new site.
            return !DistancePredicate::<I1, F1, I2, F2>::distance_predicate(
                node2.left_site(),
                node2.right_site(),
                point1,
            );
        } else {
            // These checks were evaluated experimentally.
            match site1.sorted_index().cmp(&site2.sorted_index()) {
                Ordering::Equal => {
                    // Both nodes are new (inserted during same site event processing).
                    return Self::get_comparison_y(&node1, true)
                        < Self::get_comparison_y(&node2, true);
                }
                Ordering::Less => {
                    let y1 = Self::get_comparison_y(&node1, false);
                    let y2 = Self::get_comparison_y(&node2, true);
                    if y1.0 != y2.0 {
                        return y1.0 < y2.0;
                    }
                    return if !site1.is_segment() { y1.1 < 0 } else { false };
                }
                _ => {
                    let y1 = Self::get_comparison_y(node1, true);
                    let y2 = Self::get_comparison_y(node2, false);
                    if y1.0 != y2.0 {
                        return y1.0 < y2.0;
                    }
                    return if !site2.is_segment() { y2.1 > 0 } else { true };
                }
            }
        }
    }

    //private:
    /// Get the newer site.
    pub(crate) fn get_comparison_site(
        node: &VB::BeachLineNodeKey<I1, F1, I2, F2>,
    ) -> &VSE::SiteEvent<I1, F1, I2, F2> {
        if node.left_site().sorted_index() > node.right_site().sorted_index() {
            node.left_site()
        } else {
            node.right_site()
        }
    }

    pub(crate) fn get_comparison_point(site: &VSE::SiteEvent<I1, F1, I2, F2>) -> &Point<I1> {
        if PointComparisonPredicate::<I1>::point_comparison_predicate(site.point0(), site.point1())
        {
            site.point0()
        } else {
            site.point1()
        }
    }

    /// Get comparison pair: tuple of y coordinate and direction of the newer site.
    pub(crate) fn get_comparison_y(
        node: &VB::BeachLineNodeKey<I1, F1, I2, F2>,
        is_new_node: bool,
    ) -> (I1, i8) {
        if node.left_site().sorted_index() == node.right_site().sorted_index() {
            return (node.left_site().y0(), 0);
        }
        if node.left_site().sorted_index() > node.right_site().sorted_index() {
            if !is_new_node
                && node.left_site().is_segment()
                && VoronoiPredicates::<I1, F1, I2, F2>::is_vertical_1(node.left_site())
            {
                return (node.left_site().y0(), 1);
            }
            return (node.left_site().y1(), 1);
        }
        return (node.right_site().y0(), -1);
    }
}

//#[derive(Default)]

pub struct CircleExistencePredicate<I1, F1, I2, F2>
where
    I1: BoostInputType + Neg<Output = I1>,
    F1: BoostOutputType + Neg<Output = F1>,
    I2: BigIntType + Neg<Output = I2>,
    F2: BigFloatType + Neg<Output = F2>,
{
    _pdo: PhantomData<F1>,
    _pdi: PhantomData<I1>,
    _pdbi: PhantomData<I2>,
    _pdbf: PhantomData<F2>,
}

impl<I1, F1, I2, F2> CircleExistencePredicate<I1, F1, I2, F2>
where
    I1: BoostInputType + Neg<Output = I1>,
    F1: BoostOutputType + Neg<Output = F1>,
    I2: BigIntType + Neg<Output = I2>,
    F2: BigFloatType + Neg<Output = F2>,
{
    pub(crate) fn ppp_debug(
        site1: &VSE::SiteEvent<I1, F1, I2, F2>,
        site2: &VSE::SiteEvent<I1, F1, I2, F2>,
        site3: &VSE::SiteEvent<I1, F1, I2, F2>,
    ) -> bool {
        let rv = Self::ppp(site1, site2, site3);
        println!(
            "CircleExistencePredicate::ppp(site1={:?}, site2={:?}, site2={:?}), rv={}",
            site1, site2, site3, rv
        );
        rv
    }

    pub(crate) fn ppp(
        site1: &VSE::SiteEvent<I1, F1, I2, F2>,
        site2: &VSE::SiteEvent<I1, F1, I2, F2>,
        site3: &VSE::SiteEvent<I1, F1, I2, F2>,
    ) -> bool {
        OrientationTest::<I1, F1, I2, F2>::eval_3(site1.point0(), site2.point0(), site3.point0())
            == Orientation::RIGHT
    }

    pub(crate) fn pps_debug(
        site1: &VSE::SiteEvent<I1, F1, I2, F2>,
        site2: &VSE::SiteEvent<I1, F1, I2, F2>,
        site3: &VSE::SiteEvent<I1, F1, I2, F2>,
        segment_index: u64,
    ) -> bool {
        let rv = Self::pps(site1, site2, site3, segment_index);
        println!(
            "CircleExistencePredicate::pps(site1={:?}, site2={:?}, site2={:?},segment_index={}),  rv={}",
            site1, site2, site3, segment_index, rv
        );
        rv
    }

    pub(crate) fn pps(
        site1: &VSE::SiteEvent<I1, F1, I2, F2>,
        site2: &VSE::SiteEvent<I1, F1, I2, F2>,
        site3: &VSE::SiteEvent<I1, F1, I2, F2>,
        segment_index: u64,
    ) -> bool {
        if segment_index != 2 {
            let orient1 = OrientationTest::<I1, F1, I2, F2>::eval_3(
                site1.point0(),
                site2.point0(),
                site3.point0(),
            );
            let orient2 = OrientationTest::<I1, F1, I2, F2>::eval_3(
                site1.point0(),
                site2.point0(),
                site3.point1(),
            );
            if segment_index == 1 && site1.x0() >= site2.x0() {
                if orient1 != Orientation::RIGHT {
                    return false;
                }
            } else if segment_index == 3 && site2.x0() >= site1.x0() {
                if orient2 != Orientation::RIGHT {
                    return false;
                }
            } else if orient1 != Orientation::RIGHT && orient2 != Orientation::RIGHT {
                return false;
            }
        } else {
            return (site3.point0() != site1.point0()) || (site3.point1() != site2.point0());
        }
        true
    }

    pub(crate) fn pss_debug(
        site1: &VSE::SiteEvent<I1, F1, I2, F2>,
        site2: &VSE::SiteEvent<I1, F1, I2, F2>,
        site3: &VSE::SiteEvent<I1, F1, I2, F2>,
        point_index: i32,
    ) -> bool {
        let rv = Self::pss(site1, site2, site3, point_index);
        println!(
            "CircleExistencePredicate::pss(site1={:?}, site2={:?}, site2={:?},segment_index={})={}",
            site1, site2, site3, point_index, rv
        );
        rv
    }

    pub(crate) fn pss(
        site1: &VSE::SiteEvent<I1, F1, I2, F2>,
        site2: &VSE::SiteEvent<I1, F1, I2, F2>,
        site3: &VSE::SiteEvent<I1, F1, I2, F2>,
        point_index: i32,
    ) -> bool {
        if site2.sorted_index() == site3.sorted_index() {
            return false;
        }
        if point_index == 2i32 {
            if !site2.is_inverse() && site3.is_inverse() {
                return false;
            }
            if site2.is_inverse() == site3.is_inverse()
                && OrientationTest::<I1, F1, I2, F2>::eval_3(
                    site2.point0(),
                    site1.point0(),
                    site3.point1(),
                ) != Orientation::RIGHT
            {
                return false;
            }
        }
        true
    }

    pub(crate) fn sss_debug(
        site1: &VSE::SiteEvent<I1, F1, I2, F2>,
        site2: &VSE::SiteEvent<I1, F1, I2, F2>,
        site3: &VSE::SiteEvent<I1, F1, I2, F2>,
    ) -> bool {
        let rv = Self::sss(site1, site2, site3);
        println!(
            "CircleExistencePredicate::sss(site1={:?}, site2={:?}, site2={:?})={}",
            site1, site2, site3, rv
        );
        rv
    }

    pub(crate) fn sss(
        site1: &VSE::SiteEvent<I1, F1, I2, F2>,
        site2: &VSE::SiteEvent<I1, F1, I2, F2>,
        site3: &VSE::SiteEvent<I1, F1, I2, F2>,
    ) -> bool {
        (site1.sorted_index() != site2.sorted_index())
            && (site2.sorted_index() != site3.sorted_index())
    }
}

#[derive(Default)]
pub struct LazyCircleFormationFunctor<I1, F1, I2, F2>
where
    I1: BoostInputType + Neg<Output = I1>,
    F1: BoostOutputType + Neg<Output = F1>,
    I2: BigIntType + Neg<Output = I2>,
    F2: BigFloatType + Neg<Output = F2>,
{
    _pdo: PhantomData<F1>,
    _pdi: PhantomData<I1>,
    _pdbi: PhantomData<I2>,
    _pdbf: PhantomData<F2>,
}

#[allow(non_snake_case)]
impl<I1, F1, I2, F2> LazyCircleFormationFunctor<I1, F1, I2, F2>
where
    I1: BoostInputType + Neg<Output = I1>,
    F1: BoostOutputType + Neg<Output = F1>,
    I2: BigIntType + Neg<Output = I2>,
    F2: BigFloatType + Neg<Output = F2>,
{
    pub(crate) fn ppp_debug(
        site1: &VSE::SiteEvent<I1, F1, I2, F2>,
        site2: &VSE::SiteEvent<I1, F1, I2, F2>,
        site3: &VSE::SiteEvent<I1, F1, I2, F2>,
        c_event: &VC::CircleEventType<F2>
    ) {
        Self::ppp(site1, site2, site3, c_event);
        println!(
            "LazyCircleFormationFunctor::ppp(site1={:?}, site2={:?}, site3={:?}, c_event={:?})",
            site1, site2, site3, c_event
        );
    }

    fn ppp(
        site1: &VSE::SiteEvent<I1, F1, I2, F2>,
        site2: &VSE::SiteEvent<I1, F1, I2, F2>,
        site3: &VSE::SiteEvent<I1, F1, I2, F2>,
        c_event: &VC::CircleEventType<F2>,
    ) {
        let i1_to_f2 = TC::<I1, F1, I2, F2>::i1_to_f2;
        let i1_to_i2 = TC::<I1, F1, I2, F2>::i1_to_i2;
        let f2_to_f1 = TC::<I1, F1, I2, F2>::f2_to_f1;

        let dif_x1 = i1_to_f2(site1.x()) - i1_to_f2(site2.x());
        let dif_x2 = i1_to_f2(site2.x()) - i1_to_f2(site3.x());
        let dif_y1 = i1_to_f2(site1.y()) - i1_to_f2(site2.y());
        let dif_y2 = i1_to_f2(site2.y()) - i1_to_f2(site3.y());
        let orientation = VoronoiPredicates::<I1, F1, I2, F2>::robust_cross_product_2i(
            i1_to_i2(site1.x()) - i1_to_i2(site2.x()),
            i1_to_i2(site2.x()) - i1_to_i2(site3.x()),
            i1_to_i2(site1.y()) - i1_to_i2(site2.y()),
            i1_to_i2(site2.y()) - i1_to_i2(site3.y()),
        );
        let inv_orientation: VR::RobustFpt<F2> = VR::RobustFpt::<F2>::new_2(
            num::cast::<f32, F2>(0.5f32).unwrap() / orientation,
            num::cast::<f32, F2>(2.0f32).unwrap(),
        );
        let sum_x1: F2 = i1_to_f2(site1.x()) + i1_to_f2(site2.x());
        let sum_x2: F2 = i1_to_f2(site2.x()) + i1_to_f2(site3.x());
        let sum_y1: F2 = i1_to_f2(site1.y()) + i1_to_f2(site2.y());
        let sum_y2: F2 = i1_to_f2(site2.y()) + i1_to_f2(site3.y());
        let dif_x3: F2 = i1_to_f2(site1.x()) - i1_to_f2(site3.x());
        let dif_y3: F2 = i1_to_f2(site1.y()) - i1_to_f2(site3.y());
        let mut c_x = VR::RobustDif::<F2>::new();
        let mut c_y = VR::RobustDif::<F2>::new();
        let error = num::cast::<f32, F2>(2.0f32).unwrap();
        c_x += VR::RobustFpt::<F2>::new_2(dif_x1 * sum_x1 * dif_y2, error);
        c_x += VR::RobustFpt::<F2>::new_2(dif_y1 * sum_y1 * dif_y2, error);
        c_x -= VR::RobustFpt::<F2>::new_2(dif_x2 * sum_x2 * dif_y1, error);
        c_x -= VR::RobustFpt::<F2>::new_2(dif_y2 * sum_y2 * dif_y1, error);
        c_y += VR::RobustFpt::<F2>::new_2(dif_x2 * sum_x2 * dif_x1, error);
        c_y += VR::RobustFpt::<F2>::new_2(dif_y2 * sum_y2 * dif_x1, error);
        c_y -= VR::RobustFpt::<F2>::new_2(dif_x1 * sum_x1 * dif_x2, error);
        c_y -= VR::RobustFpt::<F2>::new_2(dif_y1 * sum_y1 * dif_x2, error);
        let mut lower_x = VR::RobustDif::<F2>::new_from(c_x);
        lower_x -= VR::RobustFpt::<F2>::new_2(
            ((dif_x1 * dif_x1 + dif_y1 * dif_y1)
                * (dif_x2 * dif_x2 + dif_y2 * dif_y2)
                * (dif_x3 * dif_x3 + dif_y3 * dif_y3))
                .sqrt(),
            num::cast::<f32, F2>(5.0f32).unwrap(),
        );
        //dbg!(c_x.dif().fpv(),c_y.dif().fpv() ,lower_x.dif().fpv() ,dif_y2,inv_orientation.fpv());

        c_event.set_3_raw(
            c_x.dif().fpv() * inv_orientation.fpv(),
            c_y.dif().fpv() * inv_orientation.fpv(),
            lower_x.dif().fpv() * inv_orientation.fpv(),
        );
        let ulps = TCC::<I1, F1,I2,F2>::u64_to_f2(VoronoiPredicates::<I1, F1, I2, F2>::ulps());
        let recompute_c_x = c_x.dif().ulp() > ulps;
        let recompute_c_y = c_y.dif().ulp() > ulps;
        let recompute_lower_x = lower_x.dif().ulp() > ulps;
        if recompute_c_x || recompute_c_y || recompute_lower_x {
            ExactCircleFormationFunctor::<I1, F1, I2, F2>::ppp(
                site1,
                site2,
                site3,
                &c_event,
                recompute_c_x,
                recompute_c_y,
                recompute_lower_x,
            );
        }
    }

    pub(crate) fn pps_debug(
        site1: &VSE::SiteEvent<I1, F1, I2, F2>,
        site2: &VSE::SiteEvent<I1, F1, I2, F2>,
        site3: &VSE::SiteEvent<I1, F1, I2, F2>,
        segment_index: usize,
        c_event: &VC::CircleEventType<F2>
    ) {
        Self::pps(site1, site2, site3, segment_index,c_event);
        println!(
            "LazyCircleFormationFunctor::pps(site1={:?}, site2={:?}, site3={:?}, segment_index={:?}, c_event={:?})",
            site1, site2, site3, segment_index, c_event
        );
    }

    fn pps(
        site1: &VSE::SiteEvent<I1, F1, I2, F2>,
        site2: &VSE::SiteEvent<I1, F1, I2, F2>,
        site3: &VSE::SiteEvent<I1, F1, I2, F2>,
        segment_index: usize,
        c_event: &VC::CircleEventType<F2>,
    ) {
        let i1_to_f2 = TC::<I1, F1, I2, F2>::i1_to_f2;
        let i1_to_i2 = TC::<I1, F1, I2, F2>::i1_to_i2;
        let f2_to_f1 = TC::<I1, F1, I2, F2>::f2_to_f1;

        let half = num::cast::<f32, F2>(0.5f32).unwrap();
        let one = num::cast::<f32, F2>(1.0f32).unwrap();
        let two = num::cast::<f32, F2>(2.0f32).unwrap();
        let three = num::cast::<f32, F2>(3.0f32).unwrap();
        let eight = num::cast::<f32, F2>(8.0f32).unwrap();

        let line_a = i1_to_f2(site3.y1()) - i1_to_f2(site3.y0());
        let line_b = i1_to_f2(site3.x0()) - i1_to_f2(site3.x1());
        let vec_x = i1_to_f2(site2.y()) - i1_to_f2(site1.y());
        let vec_y = i1_to_f2(site1.x()) - i1_to_f2(site2.x());
        let teta = VR::RobustFpt::<F2>::new_2(
            VoronoiPredicates::<I1, F1, I2, F2>::robust_cross_product_2i(
                i1_to_i2(site3.y1()) - i1_to_i2(site3.y0()),
                i1_to_i2(site3.x0()) - i1_to_i2(site3.x1()),
                i1_to_i2(site2.x()) - i1_to_i2(site1.x()),
                i1_to_i2(site2.y()) - i1_to_i2(site1.y()),
            ),
            one,
        );
        let A = VR::RobustFpt::<F2>::new_2(
            VoronoiPredicates::<I1, F1, I2, F2>::robust_cross_product_2i(
                i1_to_i2(site3.y0()) - i1_to_i2(site3.y1()),
                i1_to_i2(site3.x0()) - i1_to_i2(site3.x1()),
                i1_to_i2(site3.y1()) - i1_to_i2(site1.y()),
                i1_to_i2(site3.x1()) - i1_to_i2(site1.x()),
            ),
            one,
        );
        let B = VR::RobustFpt::<F2>::new_2(
            VoronoiPredicates::<I1, F1, I2, F2>::robust_cross_product_2i(
                i1_to_i2(site3.y0()) - i1_to_i2(site3.y1()),
                i1_to_i2(site3.x0()) - i1_to_i2(site3.x1()),
                i1_to_i2(site3.y1()) - i1_to_i2(site2.y()),
                i1_to_i2(site3.x1()) - i1_to_i2(site2.x()),
            ),
            one,
        );
        let denom = VR::RobustFpt::<F2>::new_2(
            VoronoiPredicates::<I1, F1, I2, F2>::robust_cross_product_2i(
                i1_to_i2(site1.y()) - i1_to_i2(site2.y()),
                i1_to_i2(site1.x()) - i1_to_i2(site2.x()),
                i1_to_i2(site3.y1()) - i1_to_i2(site3.y0()),
                i1_to_i2(site3.x1()) - i1_to_i2(site3.x0()),
            ),
            one,
        );
        let inv_segm_len =
            VR::RobustFpt::<F2>::new_2(one / (line_a * line_a + line_b * line_b).sqrt(), three);
        let mut t = VR::RobustFpt::<F2>::default();
        if OrientationTest::<I1, F1, I2, F2>::eval_f(denom.fpv()) == Orientation::COLLINEAR {
            t += teta / (VR::RobustFpt::<F2>::new_1(eight) * A);
            t -= A / (VR::RobustFpt::<F2>::new_1(two) * teta);
        } else {
            let det = ((teta * teta + denom * denom) * A * B).sqrt();
            if segment_index == 2 {
                t -= det / (denom * denom);
            } else {
                t += det / (denom * denom);
            }
            t += teta * (A + B) / (VR::RobustFpt::<F2>::new_1(two) * denom * denom);
        }
        let mut c_x = VR::RobustDif::<F2>::default();
        let mut c_y = VR::RobustDif::<F2>::default();
        c_x += VR::RobustFpt::<F2>::new_1(
            half * (TC::<I1, F1, I2, F2>::i1_to_f2(site1.x())
                + TC::<I1, F1, I2, F2>::i1_to_f2(site2.x())),
        );
        c_x += VR::RobustFpt::<F2>::new_1(vec_x) * t;
        c_y += VR::RobustFpt::<F2>::new_1(
            half * (TC::<I1, F1, I2, F2>::i1_to_f2(site1.y())
                + TC::<I1, F1, I2, F2>::i1_to_f2(site2.y())),
        );
        c_y += VR::RobustFpt::<F2>::new_1(vec_y) * t;
        let mut r = VR::RobustDif::<F2>::default();
        let mut lower_x = VR::RobustDif::<F2>::new_from(c_x);
        r -= VR::RobustFpt::<F2>::new_1(line_a)
            * VR::RobustFpt::<F2>::new_1(TC::<I1, F1, I2, F2>::i1_to_f2(site3.x0()));
        r -= VR::RobustFpt::<F2>::new_1(line_b)
            * VR::RobustFpt::<F2>::new_1(TC::<I1, F1, I2, F2>::i1_to_f2(site3.y0()));
        r += c_x * VR::RobustFpt::<F2>::new_1(line_a);
        r += c_y * VR::RobustFpt::<F2>::new_1(line_b);

        if r.positive().fpv() < r.negative().fpv() {
            r = -r;
        }
        lower_x += r * inv_segm_len;
        {
            // Todo check if this is correct
            //  = VC::CircleEvent::<F1>::new_3(c_x.dif(), c_y.dif(), lower_x.dif());
            let mut c_eventc: VC::CircleEvent<F2> = c_event.0.get();
            c_eventc.set_3_raw(
                c_x.dif().fpv(),
                c_y.dif().fpv(),
                lower_x.dif().fpv(),
            );
            c_event.0.set(c_eventc);
        }
        let ulps = TCC::<I1, F1,I2,F2>::u64_to_f2(VoronoiPredicates::<I1, F1, I2, F2>::ulps());
        let recompute_c_x = c_x.dif().ulp() > ulps;
        let recompute_c_y = c_y.dif().ulp() > ulps;
        let recompute_lower_x = lower_x.dif().ulp() > ulps;

        // TODO! remove this
        /*let recompute_c_x= true;
        let recompute_c_y= true;
        let recompute_lower_x= true;
        */// TODO! remove this

        if recompute_c_x || recompute_c_y || recompute_lower_x {
            ExactCircleFormationFunctor::<I1, F1, I2, F2>::pps(
                site1,
                site2,
                site3,
                segment_index,
                c_event,
                recompute_c_x,
                recompute_c_y,
                recompute_lower_x,
            );
        }
    }

    pub(crate) fn pss_debug(
        site1: &VSE::SiteEvent<I1, F1, I2, F2>,
        site2: &VSE::SiteEvent<I1, F1, I2, F2>,
        site3: &VSE::SiteEvent<I1, F1, I2, F2>,
        point_index: i32,
        c_event: &VC::CircleEventType<F2>
    ) {
        Self::pss(site1, site2, site3, point_index,c_event);
        println!(
            "LazyCircleFormationFunctor::pss(site1={:?}, site2={:?}, site3={:?}, point_index={:?}, c_event={:?})",
            site1, site2, site3, point_index, c_event
        );
    }

    #[allow(unused_parens)]
    fn pss(
        site1: &VSE::SiteEvent<I1, F1, I2, F2>,
        site2: &VSE::SiteEvent<I1, F1, I2, F2>,
        site3: &VSE::SiteEvent<I1, F1, I2, F2>,
        point_index: i32,
        c_event: &VC::CircleEventType<F2>,
    ) {
        let i1_to_f2 = TC::<I1, F1, I2, F2>::i1_to_f2;
        let i1_to_i2 = TC::<I1, F1, I2, F2>::i1_to_i2;
        let f2_to_f1 = TC::<I1, F1, I2, F2>::f2_to_f1;

        let half = num::cast::<f32, F2>(0.5f32).unwrap();
        let one = num::cast::<f32, F2>(1.0f32).unwrap();
        let two = num::cast::<f32, F2>(2.0f32).unwrap();
        let segm_start1 = site2.point1();
        let segm_end1 = site2.point0();
        let segm_start2 = site3.point0();
        let segm_end2 = site3.point1();
        let a1 = i1_to_f2(segm_end1.x()) - i1_to_f2(segm_start1.x());
        let b1 = i1_to_f2(segm_end1.y()) - i1_to_f2(segm_start1.y());
        let a2 = i1_to_f2(segm_end2.x()) - i1_to_f2(segm_start2.x());
        let b2 = i1_to_f2(segm_end2.y()) - i1_to_f2(segm_start2.y());
        let mut recompute_c_x = false;
        let mut recompute_c_y = false;
        let mut recompute_lower_x = false;

        let orientation = VR::RobustFpt::<F2>::new_2(
            VoronoiPredicates::<I1, F1, I2, F2>::robust_cross_product_2i(
                i1_to_i2(segm_end1.y()) - i1_to_i2(segm_start1.y()),
                i1_to_i2(segm_end1.x()) - i1_to_i2(segm_start1.x()),
                i1_to_i2(segm_end2.y()) - i1_to_i2(segm_start2.y()),
                i1_to_i2(segm_end2.x()) - i1_to_i2(segm_start2.x()),
            ),
            one,
        );
        if OrientationTest::<I1, F1, I2, F2>::eval_f(orientation.fpv()) == Orientation::COLLINEAR {
            let a = VR::RobustFpt::<F2>::new_2(a1 * a1 + b1 * b1, two);
            let c = VR::RobustFpt::<F2>::new_2(
                VoronoiPredicates::<I1, F1, I2, F2>::robust_cross_product_2i(
                    i1_to_i2(segm_end1.y()) - i1_to_i2(segm_start1.y()),
                    i1_to_i2(segm_end1.x()) - i1_to_i2(segm_start1.x()),
                    i1_to_i2(segm_start2.y()) - i1_to_i2(segm_start1.y()),
                    i1_to_i2(segm_start2.x()) - i1_to_i2(segm_start1.x()),
                ),
                one,
            );
            let det = VR::RobustFpt::<F2>::new_2(
                VoronoiPredicates::<I1, F1, I2, F2>::robust_cross_product_2i(
                    i1_to_i2(segm_end1.x()) - i1_to_i2(segm_start1.x()),
                    i1_to_i2(segm_end1.y()) - i1_to_i2(segm_start1.y()),
                    i1_to_i2(site1.x()) - i1_to_i2(segm_start1.x()),
                    i1_to_i2(site1.y()) - i1_to_i2(segm_start1.y()),
                ) * VoronoiPredicates::<I1, F1, I2, F2>::robust_cross_product_2i(
                    i1_to_i2(segm_end1.y()) - i1_to_i2(segm_start1.y()),
                    i1_to_i2(segm_end1.x()) - i1_to_i2(segm_start1.x()),
                    i1_to_i2(site1.y()) - i1_to_i2(segm_start2.y()),
                    i1_to_i2(site1.x()) - i1_to_i2(segm_start2.x()),
                ),
                num::cast::<f32, F2>(3.0f32).unwrap(),
            );
            let mut t = VR::RobustFpt::<F2>::default();
            t -= VR::RobustFpt::<F2>::new_1(a1)
                * VR::RobustFpt::<F2>::new_1(
                    (i1_to_f2(segm_start1.x()) + i1_to_f2(segm_start2.x())) * half
                        - i1_to_f2(site1.x()),
                );
            t -= VR::RobustFpt::<F2>::new_1(b1)
                * VR::RobustFpt::<F2>::new_1(
                    (i1_to_f2(segm_start1.y()) + i1_to_f2(segm_start2.y())) * half
                        - i1_to_f2(site1.y()),
                );
            if point_index == 2i32 {
                t += det.sqrt();
            } else {
                t -= det.sqrt();
            }
            t /= a;
            let mut c_x = VR::RobustDif::<F2>::default();
            let mut c_y = VR::RobustDif::<F2>::default();

            c_x += VR::RobustFpt::<F2>::new_1(
                half * (i1_to_f2(segm_start1.x()) + i1_to_f2(segm_start2.x())),
            );
            c_x += VR::RobustFpt::<F2>::new_1(a1) * t;
            c_y += VR::RobustFpt::<F2>::new_1(
                half * (i1_to_f2(segm_start1.y()) + i1_to_f2(segm_start2.y())),
            );
            c_y += VR::RobustFpt::<F2>::new_1(b1) * t;
            let mut lower_x = VR::RobustDif::<F2>::new_from(c_x);
            if c.is_sign_negative() {
                lower_x -= VR::RobustFpt::<F2>::new_1(half) * c / a.sqrt();
            } else {
                lower_x += VR::RobustFpt::<F2>::new_1(half) * c / a.sqrt();
            }
            let ulps =TCC::<I1, F1,I2,F2>::u64_to_f2(VoronoiPredicates::<I1, F1, I2, F2>::ulps());
            let recompute_c_x = c_x.dif().ulp() > ulps;
            let recompute_c_y = c_y.dif().ulp() > ulps;
            let recompute_lower_x = lower_x.dif().ulp() > ulps;
            c_event.set_3_raw(
                c_x.dif().fpv(),
                c_y.dif().fpv(),
                lower_x.dif().fpv(),
            );
        } else {
            let sqr_sum1 = VR::RobustFpt::<F2>::new_2((a1 * a1 + b1 * b1).sqrt(), two);
            let sqr_sum2 = VR::RobustFpt::<F2>::new_2((a2 * a2 + b2 * b2).sqrt(), two);
            let mut a = VR::RobustFpt::<F2>::new_2(
                VoronoiPredicates::<I1, F1, I2, F2>::robust_cross_product_2i(
                    i1_to_i2(segm_end1.x()) - i1_to_i2(segm_start1.x()),
                    i1_to_i2(segm_end1.y()) - i1_to_i2(segm_start1.y()),
                    i1_to_i2(segm_start2.y()) - i1_to_i2(segm_end2.y()),
                    i1_to_i2(segm_end2.x()) - i1_to_i2(segm_start2.x()),
                ),
                one,
            );
            if a.is_sign_positive() {
                a += sqr_sum1 * sqr_sum2;
            } else {
                a = (orientation * orientation) / (sqr_sum1 * sqr_sum2 - a);
            }
            let or1 = VR::RobustFpt::<F2>::new_2(
                VoronoiPredicates::<I1, F1, I2, F2>::robust_cross_product_2i(
                    i1_to_i2(segm_end1.y()) - i1_to_i2(segm_start1.y()),
                    i1_to_i2(segm_end1.x()) - i1_to_i2(segm_start1.x()),
                    i1_to_i2(segm_end1.y()) - i1_to_i2(site1.y()),
                    i1_to_i2(segm_end1.x()) - i1_to_i2(site1.x()),
                ),
                one,
            );
            let or2 = VR::RobustFpt::<F2>::new_2(
                VoronoiPredicates::<I1, F1, I2, F2>::robust_cross_product_2i(
                    i1_to_i2(segm_end2.x()) - i1_to_i2(segm_start2.x()),
                    i1_to_i2(segm_end2.y()) - i1_to_i2(segm_start2.y()),
                    i1_to_i2(segm_end2.x()) - i1_to_i2(site1.x()),
                    i1_to_i2(segm_end2.y()) - i1_to_i2(site1.y()),
                ),
                one,
            );
            let det = VR::RobustFpt::<F2>::new_1(two) * a * or1 * or2;
            let c1 = VR::RobustFpt::<F2>::new_2(
                VoronoiPredicates::<I1, F1, I2, F2>::robust_cross_product_2i(
                    i1_to_i2(segm_end1.y()) - i1_to_i2(segm_start1.y()),
                    i1_to_i2(segm_end1.x()) - i1_to_i2(segm_start1.x()),
                    i1_to_i2(segm_end1.y()),
                    i1_to_i2(segm_end1.x()),
                ),
                one,
            );
            let c2 = VR::RobustFpt::<F2>::new_2(
                VoronoiPredicates::<I1, F1, I2, F2>::robust_cross_product_2i(
                    i1_to_i2(segm_end2.x()) - i1_to_i2(segm_start2.x()),
                    i1_to_i2(segm_end2.y()) - i1_to_i2(segm_start2.y()),
                    i1_to_i2(segm_end2.x()),
                    i1_to_i2(segm_end2.y()),
                ),
                one,
            );
            let inv_orientation = VR::RobustFpt::<F2>::new_1(one) / orientation;
            let mut t = VR::RobustDif::<F2>::default();
            let mut b = VR::RobustDif::<F2>::default();
            let mut ix = VR::RobustDif::<F2>::default();
            let mut iy = VR::RobustDif::<F2>::default();

            ix += VR::RobustFpt::<F2>::new_1(a2) * c1 * inv_orientation;
            ix += VR::RobustFpt::<F2>::new_1(a1) * c2 * inv_orientation;
            iy += VR::RobustFpt::<F2>::new_1(b1) * c2 * inv_orientation;
            iy += VR::RobustFpt::<F2>::new_1(b2) * c1 * inv_orientation;

            b += ix * (VR::RobustFpt::<F2>::new_1(a1) * sqr_sum2);
            b += ix * (VR::RobustFpt::<F2>::new_1(a2) * sqr_sum1);
            b += iy * (VR::RobustFpt::<F2>::new_1(b1) * sqr_sum2);
            b += iy * (VR::RobustFpt::<F2>::new_1(b2) * sqr_sum1);
            b -= sqr_sum1
                * VR::RobustFpt::<F2>::new_2(
                    VoronoiPredicates::<I1, F1, I2, F2>::robust_cross_product_2i(
                        i1_to_i2(segm_end2.x()) - i1_to_i2(segm_start2.x()),
                        i1_to_i2(segm_end2.y()) - i1_to_i2(segm_start2.y()),
                        i1_to_i2(-site1.y()),
                        i1_to_i2(site1.x()),
                    ),
                    one,
                );
            b -= sqr_sum2
                * VR::RobustFpt::<F2>::new_2(
                    VoronoiPredicates::<I1, F1, I2, F2>::robust_cross_product_2i(
                        i1_to_i2(segm_end1.x()) - i1_to_i2(segm_start1.x()),
                        i1_to_i2(segm_end1.y()) - i1_to_i2(segm_start1.y()),
                        i1_to_i2(-site1.y()),
                        i1_to_i2(site1.x()),
                    ),
                    one,
                );
            t -= b;
            if point_index == 2i32 {
                t += det.sqrt();
            } else {
                t -= det.sqrt();
            }

            t /= (a * a);

            let mut c_x = VR::RobustDif::<F2>::new_from(ix);
            let mut c_y = VR::RobustDif::<F2>::new_from(iy);

            c_x += t * (VR::RobustFpt::<F2>::new_1(a1) * sqr_sum2);
            c_x += t * (VR::RobustFpt::<F2>::new_1(a2) * sqr_sum1);
            c_y += t * (VR::RobustFpt::<F2>::new_1(b1) * sqr_sum2);
            c_y += t * (VR::RobustFpt::<F2>::new_1(b2) * sqr_sum1);
            if t.positive().fpv() < t.negative().fpv() {
                t = -t;
            }
            let mut lower_x = VR::RobustDif::<F2>::new_from(c_x);
            if orientation.is_sign_negative() {
                lower_x -= t * orientation;
            } else {
                lower_x += t * orientation;
            }
            let ulps = TCC::<I1, F1,I2,F2>::u64_to_f2(VoronoiPredicates::<I1, F1, I2, F2>::ulps());
            recompute_c_x = c_x.dif().ulp() > ulps;
            recompute_c_y = c_y.dif().ulp() > ulps;
            recompute_lower_x = lower_x.dif().ulp() > ulps;
            // Todo! Is this correct? it was let c_event = ...
            c_event.set_3_raw(
                c_x.dif().fpv(),
                c_y.dif().fpv(),
                lower_x.dif().fpv(),
            );
        }
        if recompute_c_x || recompute_c_y || recompute_lower_x {
            ExactCircleFormationFunctor::pss(
                site1,
                site2,
                site3,
                point_index,
                &c_event,
                recompute_c_x,
                recompute_c_y,
                recompute_lower_x,
            );
        }
    }

    pub(crate) fn sss_debug(
        site1: &VSE::SiteEvent<I1, F1, I2, F2>,
        site2: &VSE::SiteEvent<I1, F1, I2, F2>,
        site3: &VSE::SiteEvent<I1, F1, I2, F2>,
        c_event: &VC::CircleEventType<F2>
    ) {
        Self::sss(site1, site2, site3,c_event);
        println!(
            "LazyCircleFormationFunctor::sss(site1={:?}, site2={:?}, site3={:?}, c_event={:?})",
            site1, site2, site3,  c_event
        );
    }

    fn sss(
        site1: &VSE::SiteEvent<I1, F1, I2, F2>,
        site2: &VSE::SiteEvent<I1, F1, I2, F2>,
        site3: &VSE::SiteEvent<I1, F1, I2, F2>,
        c_event: &VC::CircleEventType<F2>,
    ) {
        let i1_to_f2 = TC::<I1, F1, I2, F2>::i1_to_f2;
        let f2_to_f1 = TC::<I1, F1, I2, F2>::f2_to_f1;
        let i1_to_i2 = TC::<I1, F1, I2, F2>::i1_to_i2;

        let one = num::cast::<f32, F2>(1.0f32).unwrap();

        let a1 = VR::RobustFpt::<F2>::new_1(i1_to_f2(site1.x1()) - i1_to_f2(site1.x0()));
        let b1 = VR::RobustFpt::<F2>::new_1(i1_to_f2(site1.y1()) - i1_to_f2(site1.y0()));
        let c1 = VR::RobustFpt::<F2>::new_2(
            VoronoiPredicates::<I1, F1, I2, F2>::robust_cross_product(
                site1.x0(),
                site1.y0(),
                site1.x1(),
                site1.y1(),
            ),
            one,
        );

        let a2 = VR::RobustFpt::<F2>::new_1(i1_to_f2(site2.x1()) - i1_to_f2(site2.x0()));
        let b2 = VR::RobustFpt::<F2>::new_1(i1_to_f2(site2.y1()) - i1_to_f2(site2.y0()));
        let c2 = VR::RobustFpt::<F2>::new_2(
            VoronoiPredicates::<I1, F1, I2, F2>::robust_cross_product(
                site2.x0(),
                site2.y0(),
                site2.x1(),
                site2.y1(),
            ),
            one,
        );

        let a3 = VR::RobustFpt::<F2>::new_1(i1_to_f2(site3.x1()) - i1_to_f2(site3.x0()));
        let b3 = VR::RobustFpt::<F2>::new_1(i1_to_f2(site3.y1()) - i1_to_f2(site3.y0()));
        let c3 = VR::RobustFpt::<F2>::new_2(
            VoronoiPredicates::<I1, F1, I2, F2>::robust_cross_product(
                site3.x0(),
                site3.y0(),
                site3.x1(),
                site3.y1(),
            ),
            one,
        );

        let len1 = (a1 * a1 + b1 * b1).sqrt();
        let len2 = (a2 * a2 + b2 * b2).sqrt();
        let len3 = (a3 * a3 + b3 * b3).sqrt();
        let cross_12 = VR::RobustFpt::<F2>::new_2(
            VoronoiPredicates::<I1, F1, I2, F2>::robust_cross_product_2i(
                i1_to_i2(site1.x1()) - i1_to_i2(site1.x0()),
                i1_to_i2(site1.y1()) - i1_to_i2(site1.y0()),
                i1_to_i2(site2.x1()) - i1_to_i2(site2.x0()),
                i1_to_i2(site2.y1()) - i1_to_i2(site2.y0()),
            ),
            one,
        );
        let cross_23 = VR::RobustFpt::<F2>::new_2(
            VoronoiPredicates::<I1, F1, I2, F2>::robust_cross_product_2i(
                i1_to_i2(site2.x1()) - i1_to_i2(site2.x0()),
                i1_to_i2(site2.y1()) - i1_to_i2(site2.y0()),
                i1_to_i2(site3.x1()) - i1_to_i2(site3.x0()),
                i1_to_i2(site3.y1()) - i1_to_i2(site3.y0()),
            ),
            one,
        );
        let cross_31 = VR::RobustFpt::<F2>::new_2(
            VoronoiPredicates::<I1, F1, I2, F2>::robust_cross_product_2i(
                i1_to_i2(site3.x1()) - i1_to_i2(site3.x0()),
                i1_to_i2(site3.y1()) - i1_to_i2(site3.y0()),
                i1_to_i2(site1.x1()) - i1_to_i2(site1.x0()),
                i1_to_i2(site1.y1()) - i1_to_i2(site1.y0()),
            ),
            one,
        );

        // denom = cross_12 * len3 + cross_23 * len1 + cross_31 * len2.
        let mut denom = VR::RobustDif::<F2>::new();
        denom += cross_12 * len3;
        denom += cross_23 * len1;
        denom += cross_31 * len2;

        // denom * r = (b2 * c_x - a2 * c_y - c2 * denom) / len2.
        let mut r = VR::RobustDif::<F2>::new();
        r -= cross_12 * c3;
        r -= cross_23 * c1;
        r -= cross_31 * c2;

        let mut c_x = VR::RobustDif::<F2>::new();
        c_x += a1 * c2 * len3;
        c_x -= a2 * c1 * len3;
        c_x += a2 * c3 * len1;
        c_x -= a3 * c2 * len1;
        c_x += a3 * c1 * len2;
        c_x -= a1 * c3 * len2;

        let mut c_y = VR::RobustDif::<F2>::new();
        c_y += b1 * c2 * len3;
        c_y -= b2 * c1 * len3;
        c_y += b2 * c3 * len1;
        c_y -= b3 * c2 * len1;
        c_y += b3 * c1 * len2;
        c_y -= b1 * c3 * len2;

        let lower_x = c_x + r;

        let denom_dif = VR::RobustFpt::<F2>::copy_from(&denom.dif());
        let c_x_dif = VR::RobustFpt::<F2>::copy_from(&c_x.dif()) / denom_dif;
        let c_y_dif = VR::RobustFpt::<F2>::copy_from(&c_y.dif()) / denom_dif;
        let lower_x_dif = VR::RobustFpt::<F2>::copy_from(&lower_x.dif()) / denom_dif;

        let ulps = TCC::<I1, F1,I2,F2>::u64_to_f2(VoronoiPredicates::<I1, F1, I2, F2>::ulps());
        let recompute_c_x = c_x_dif.ulp() > ulps;
        let recompute_c_y = c_y_dif.ulp() > ulps;
        let recompute_lower_x = lower_x_dif.ulp() > ulps;
        c_event.set_3_raw(
            c_x_dif.fpv(),
            c_y_dif.fpv(),
            lower_x_dif.fpv(),
        );
        if recompute_c_x || recompute_c_y || recompute_lower_x {
            ExactCircleFormationFunctor::sss(
                site1,
                site2,
                site3,
                &c_event,
                recompute_c_x,
                recompute_c_y,
                recompute_lower_x,
            );
        }
    }
}

#[derive(Default)]
pub struct CircleFormationFunctor<I1, F1, I2, F2>
where
    I1: BoostInputType + Neg<Output = I1>,
    F1: BoostOutputType + Neg<Output = F1>,
    I2: BigIntType + Neg<Output = I2>,
    F2: BigFloatType + Neg<Output = F2>,
{
    _pdo: PhantomData<F1>,
    _pdi: PhantomData<I1>,
    _pdbi: PhantomData<I2>,
    _pdbf: PhantomData<F2>,
}

impl<I1, F1, I2, F2> CircleFormationFunctor<I1, F1, I2, F2>
where
    I1: BoostInputType + Neg<Output = I1>,
    F1: BoostOutputType + Neg<Output = F1>,
    I2: BigIntType + Neg<Output = I2>,
    F2: BigFloatType + Neg<Output = F2>,
{
    pub(crate) fn lies_outside_vertical_segment_debug(
        circle: &VC::CircleEventType<F2>,
        site: &VSE::SiteEvent<I1, F1, I2, F2>,
    ) -> bool {
        let rv = Self::lies_outside_vertical_segment(circle, site);
        println!(
            "CircleFormationFunctor::lies_outside_vertical_segment(circle={:?}, site={:?})={:?}",
            circle, site, rv
        );
        rv
    }

    pub(crate) fn lies_outside_vertical_segment(
        c: &VC::CircleEventType<F2>,
        s: &VSE::SiteEvent<I1, F1, I2, F2>,
    ) -> bool {
        let i1_to_f64 = TC::<I1, F1, I2, F2>::i1_to_f64;
        let f2_to_f64 = TC::<I1, F1, I2, F2>::f2_to_f64;

        if !s.is_segment() || !VoronoiPredicates::<I1, F1, I2, F2>::is_vertical_1(s) {
            return false;
        }
        let y0 = i1_to_f64(if s.is_inverse() { s.y1() } else { s.y0() });
        let y1 = i1_to_f64(if s.is_inverse() { s.y0() } else { s.y1() });
        let cc_y= f2_to_f64(c.0.get().y().into_inner());

        UlpComparison::ulp_comparison(cc_y, y0,  128) == Ordering::Less
            || UlpComparison::ulp_comparison(cc_y, y1, 128) == Ordering::Greater
    }

    pub(crate) fn circle_formation_predicate_debug(
        site1: &VSE::SiteEvent<I1, F1, I2, F2>,
        site2: &VSE::SiteEvent<I1, F1, I2, F2>,
        site3: &VSE::SiteEvent<I1, F1, I2, F2>,
        circle: &VC::CircleEventType<F2>,
    ) -> bool {
        let rv = Self::circle_formation_predicate(site1, site2, site3, circle);
        println!("->circle_formation_predicate");
        dbg!(&site1, &site2, &site3, &circle, rv);
        rv
    }

    // Create a circle event from the given three sites.
    // Returns true if the circle event exists, else false.
    // If exists circle event is saved into the c_event variable.
    pub(crate) fn circle_formation_predicate(
        site1: &VSE::SiteEvent<I1, F1, I2, F2>,
        site2: &VSE::SiteEvent<I1, F1, I2, F2>,
        site3: &VSE::SiteEvent<I1, F1, I2, F2>,
        circle: &VC::CircleEventType<F2>,
    ) -> bool {

        if !site1.is_segment() {
            if !site2.is_segment() {
                if !site3.is_segment() {
                    // (point, point, point) sites.
                    if !CircleExistencePredicate::<I1, F1, I2, F2>::ppp(site1, site2, site3) {
                        return false;
                    }
                    LazyCircleFormationFunctor::<I1, F1, I2, F2>::ppp(site1, site2, site3, circle);
                } else {
                    // (point, point, segment) sites.
                    if !CircleExistencePredicate::<I1, F1, I2, F2>::pps(site1, site2, site3, 3) {
                        return false;
                    }
                    LazyCircleFormationFunctor::<I1, F1, I2, F2>::pps(
                        site1, site2, site3, 3, circle,
                    );
                }
            } else if !site3.is_segment() {
                // (point, segment, point) sites.
                if !CircleExistencePredicate::<I1, F1, I2, F2>::pps(site1, site3, site2, 2) {
                    return false;
                }
                LazyCircleFormationFunctor::<I1, F1, I2, F2>::pps(site1, site3, site2, 2, circle);
            } else {
                // (point, segment, segment) sites.
                if !CircleExistencePredicate::<I1, F1, I2, F2>::pss(site1, site2, site3, 1) {
                    return false;
                }
                LazyCircleFormationFunctor::<I1, F1, I2, F2>::pss(site1, site2, site3, 1, circle);
            }
        } else if !site2.is_segment() {
            if !site3.is_segment() {
                // (segment, point, point) sites.
                if !CircleExistencePredicate::<I1, F1, I2, F2>::pps(site2, site3, site1, 1) {
                    return false;
                }
                LazyCircleFormationFunctor::<I1, F1, I2, F2>::pps(site2, site3, site1, 1, circle);
            } else {
                // (segment, point, segment) sites.
                if !CircleExistencePredicate::<I1, F1, I2, F2>::pss(site2, site1, site3, 2) {
                    return false;
                }
                LazyCircleFormationFunctor::<I1, F1, I2, F2>::pss(site2, site1, site3, 2, circle);
            }
        } else if !site3.is_segment() {
            // (segment, segment, point) sites.
            if !CircleExistencePredicate::<I1, F1, I2, F2>::pss(site3, site1, site2, 3) {
                return false;
            }
            LazyCircleFormationFunctor::<I1, F1, I2, F2>::pss(site3, site1, site2, 3, circle);
        } else {
            // (segment, segment, segment) sites.
            if !CircleExistencePredicate::<I1, F1, I2, F2>::sss(site1, site2, site3) {
                return false;
            }
            LazyCircleFormationFunctor::<I1, F1, I2, F2>::sss(site1, site2, site3, circle);
        }

        if Self::lies_outside_vertical_segment(&circle, site1)
            || Self::lies_outside_vertical_segment(&circle, site2)
            || Self::lies_outside_vertical_segment(&circle, site3)
        {
            return false;
        }
        true
    }
}

#[derive(Default)]
pub struct ExactCircleFormationFunctor<I1, F1, I2, F2>
where
    I1: BoostInputType + Neg<Output = I1>,
    F1: BoostOutputType + Neg<Output = F1>,
    I2: BigIntType + Neg<Output = I2>,
    F2: BigFloatType + Neg<Output = F2>,
{
    _pdo: PhantomData<F1>,
    _pdi: PhantomData<I1>,
    _pdbi: PhantomData<I2>,
    _pdbf: PhantomData<F2>,
}

//type FptType = f64;
//type EFptType = f64;

impl<I1, F1, I2, F2> ExactCircleFormationFunctor<I1, F1, I2, F2>
where
    I1: BoostInputType + Neg<Output = I1>,
    F1: BoostOutputType + Neg<Output = F1>,
    I2: BigIntType + Neg<Output = I2>,
    F2: BigFloatType + Neg<Output = F2>,
{
    pub(crate) fn ppp(
        site1: &VSE::SiteEvent<I1, F1, I2, F2>,
        site2: &VSE::SiteEvent<I1, F1, I2, F2>,
        site3: &VSE::SiteEvent<I1, F1, I2, F2>,
        circle: &VC::CircleEventType<F2>,
        recompute_c_x: bool,
        recompute_c_y: bool,
        recompute_lower_x: bool,
    ) {
        let i1_to_i2 = TC::<I1, F1, I2, F2>::i1_to_i2;
        let i2_to_f2 = TC::<I1, F1, I2, F2>::i2_to_f2;
        let f1_to_f2 = TC::<I1, F1, I2, F2>::f1_to_f2;
        let f2_to_f1 = TC::<I1, F1, I2, F2>::f2_to_f1;

        let half: F2 = num::cast::<f32, F2>(0.5f32).unwrap();
        let one = 1; //: F2 = num::cast::<f32, F2>(1.0f32).unwrap();
        let two = 2; //: F2 = num::cast::<f32, F2>(2.0f32).unwrap();
        let three = 3; //: F2 = num::cast::<f32, F2>(3.0f32).unwrap();
        let eight = 8; //;F2 = num::cast::<f32, F2>(8.0f32).unwrap();

        let mut dif_x: [I2; 3] = [I2::zero(); 3];
        let mut dif_y: [I2; 3] = [I2::zero(); 3];
        let mut sum_x: [I2; 2] = [I2::zero(); 2];
        let mut sum_y: [I2; 2] = [I2::zero(); 2];

        dif_x[0] = i1_to_i2(site1.x()) - i1_to_i2(site2.x());
        dif_x[1] = i1_to_i2(site2.x()) - i1_to_i2(site3.x());
        dif_x[2] = i1_to_i2(site1.x()) - i1_to_i2(site3.x());
        dif_y[0] = i1_to_i2(site1.y()) - i1_to_i2(site2.y());
        dif_y[1] = i1_to_i2(site2.y()) - i1_to_i2(site3.y());
        dif_y[2] = i1_to_i2(site1.y()) - i1_to_i2(site3.y());
        sum_x[0] = i1_to_i2(site1.x()) + i1_to_i2(site2.x());
        sum_x[1] = i1_to_i2(site2.x()) + i1_to_i2(site3.x());
        sum_y[0] = i1_to_i2(site1.y()) + i1_to_i2(site2.y());
        sum_y[1] = i1_to_i2(site2.y()) + i1_to_i2(site3.y());

        let inv_denom: F2 = half / i2_to_f2(dif_x[0] * dif_y[1] - dif_x[1] * dif_y[0]);
        let numer1: I2 = dif_x[0] * sum_x[0] + dif_y[0] * sum_y[0];
        let numer2: I2 = dif_x[1] * sum_x[1] + dif_y[1] * sum_y[1];

        if recompute_c_x || recompute_lower_x {
            let c_x: I2 = numer1 * dif_y[1] - numer2 * dif_y[0];
            circle.set_x_raw(i2_to_f2(c_x) * inv_denom);

            if recompute_lower_x {
                // Evaluate radius of the circle.
                let sqr_r: I2 = (dif_x[0] * dif_x[0] + dif_y[0] * dif_y[0])
                    * (dif_x[1] * dif_x[1] + dif_y[1] * dif_y[1])
                    * (dif_x[2] * dif_x[2] + dif_y[2] * dif_y[2]);
                let r: F2 = i2_to_f2(sqr_r).sqrt();

                // If c_x >= 0 then lower_x = c_x + r,
                // else lower_x = (c_x * c_x - r * r) / (c_x - r).
                // To guarantee epsilon relative error.

                // this value will be invalid after call to set_lower_x()
                let tmp_circle_x: F2 = *circle.0.get().x();

                if !TCF::<F2>::is_neg(tmp_circle_x) {
                    if !TCF::<F2>::is_neg(inv_denom) {
                        circle.set_lower_x_raw(tmp_circle_x + r * inv_denom);
                    } else {
                        circle.set_lower_x_raw(tmp_circle_x - r * inv_denom);
                    }
                } else {
                    let numer: I2 = c_x * c_x - sqr_r;
                    let lower_x: F2 = i2_to_f2(numer) * inv_denom / (i2_to_f2(c_x) + r);
                    circle.set_lower_x_raw(lower_x);
                }
            }
        }

        if recompute_c_y {
            let c_y: I2 = numer2 * dif_x[0] - numer1 * dif_x[1];
            circle.set_y_raw(i2_to_f2(c_y) * inv_denom);
        }
    }

    /// Recompute parameters of the circle event using high-precision library.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn pps(
        site1: &VSE::SiteEvent<I1, F1, I2, F2>,
        site2: &VSE::SiteEvent<I1, F1, I2, F2>,
        site3: &VSE::SiteEvent<I1, F1, I2, F2>,
        segment_index: usize,
        c_event: &VC::CircleEventType<F2>,
        recompute_c_x: bool,
        recompute_c_y: bool,
        recompute_lower_x: bool,
    ) {
        let bi_to_f2 = TC::<I1, F1, I2, F2>::bi_to_f2;
        let i1_to_bi = TC::<I1, F1, I2, F2>::i1_to_bi;
        let i1_to_i128 = TC::<I1, F1, I2, F2>::i1_to_i128;
        let f2_to_f1 = TC::<I1, F1, I2, F2>::f2_to_f1;

        let sqrt_expr_ = VR::robust_sqrt_expr::<F2>::new();
        let quarter: F2 = num::cast::<f64, F2>(1f64 / 4.0f64).unwrap();
        let half: F2 = num::cast::<f64, F2>(1f64 / 2.0f64).unwrap();
        let one: I2 = num::cast::<i8, I2>(1i8).unwrap();
        let neg_one = -1i32;
        //let two = 2;//: I2 = num::cast::<i8, I2>(2i8).unwrap();
        //let four: I2 = num::cast::<i8, I2>(4i8).unwrap();

        // Todo: is 5 the correct size?
        let mut ca: [BigInt; 5] = [
            BigInt::zero(),
            BigInt::zero(),
            BigInt::zero(),
            BigInt::zero(),
            BigInt::zero(),
        ];
        let mut cb: [BigInt; 5] = [
            BigInt::zero(),
            BigInt::zero(),
            BigInt::zero(),
            BigInt::zero(),
            BigInt::zero(),
        ];
        let line_a: BigInt = i1_to_bi(site3.y1()) - i1_to_i128(site3.y0());
        let line_b: BigInt = i1_to_bi(site3.x0()) - i1_to_i128(site3.x1());
        let segm_len = line_a.clone() * &line_a + &line_b * &line_b;
        let vec_x: BigInt = i1_to_bi(site2.y()) - i1_to_i128(site1.y());
        let vec_y: BigInt = i1_to_bi(site1.x()) - i1_to_i128(site2.x());
        let sum_x: BigInt = i1_to_bi(site1.x()) + i1_to_i128(site2.x());
        let sum_y: BigInt = i1_to_bi(site1.y()) + i1_to_i128(site2.y());
        let teta = line_a.clone() * &vec_x + &line_b * &vec_y;
        let mut denom: BigInt = vec_x.clone() * &line_b - &vec_y * &line_a;

        let mut dif0: BigInt = i1_to_bi(site3.y1()) - i1_to_i128(site1.y());
        let mut dif1: BigInt = i1_to_bi(site1.x()) - i1_to_i128(site3.x1());
        let a: BigInt = line_a.clone() * &dif1 - &line_b * &dif0;

        dif0 = i1_to_bi(site3.y1()) - i1_to_i128(site2.y());
        dif1 = i1_to_bi(site2.x()) - i1_to_i128(site3.x1());
        let b = line_a * dif1 - line_b * dif0;
        let sum_ab = a.clone() + &b;

        if is_zero(&denom) {
            let numer: BigInt = teta.clone() * &teta - &sum_ab * &sum_ab;
            denom = teta.clone() * &sum_ab;
            ca[0] = denom.clone() * &sum_x * 2 + &numer * &vec_x;
            cb[0] = segm_len.clone();
            ca[1] = denom.clone() * &sum_ab * 2 + &numer * &teta;
            cb[1] = BigInt::from(1);
            ca[2] = denom.clone() * &sum_y * 2 + &numer * &vec_y;
            let inv_denom: F2 =
                TC::<I1, F1, I2, F2>::i2_to_f2(one) / TC::<I1, F1, I2, F2>::bi_to_f2(&denom);
            if recompute_c_x {
                c_event.set_x_raw(quarter * bi_to_f2(&ca[0]) * inv_denom);
            }
            if recompute_c_y {
                c_event.set_y_raw(quarter * bi_to_f2(&ca[2]) * inv_denom);
            }
            if recompute_lower_x {
                c_event.set_lower_x_raw(
                    (sqrt_expr_.eval2(&ca, &cb) * quarter * inv_denom
                        / (bi_to_f2(&segm_len).sqrt()))
                    .fpv(),
                );
            }
            return;
        }
        let det: BigInt = (teta.clone() * &teta + &denom * &denom) * &a * &b * 4;
        let mut inv_denom_sqr: F2 =
            TC::<I1, F1, I2, F2>::i2_to_f2(one) / TC::<I1, F1, I2, F2>::bi_to_f2(&denom);
        inv_denom_sqr = inv_denom_sqr * inv_denom_sqr;

        if recompute_c_x || recompute_lower_x {
            ca[0] = sum_x.clone() * &denom * &denom + &teta * &sum_ab * &vec_x;
            cb[0] = BigInt::from(1);
            ca[1] = if segment_index == 2 {
                vec_x.clone() * -1
            } else {
                vec_x.clone()
            };
            cb[1] = det.clone();
            if recompute_c_x {
                c_event.set_x_raw(
                    (sqrt_expr_.eval2(&ca, &cb) * half * inv_denom_sqr).fpv(),
                );
            }
        }

        if recompute_c_y || recompute_lower_x {
            ca[2] = sum_y.clone() * &denom * &denom + &teta * &sum_ab * &vec_y;
            cb[2] = BigInt::from(1);
            ca[3] = if segment_index == 2 {
                vec_y * neg_one
            } else {
                vec_y
            };
            cb[3] = det.clone();
            if recompute_c_y {
                c_event.set_y_raw(
                    (sqrt_expr_.eval2(&ca[2..], &cb[2..]) * half * inv_denom_sqr).fpv(),
                );
            }
        }

        if recompute_lower_x {
            cb[0] = cb[0].clone() * &segm_len;
            cb[1] = cb[1].clone() * &segm_len;
            ca[2] = sum_ab.clone() * (&denom * &denom + &teta * &teta);
            cb[2] = BigInt::from(1);
            ca[3] = if segment_index == 2 { -teta } else { teta };
            cb[3] = det;
            let segm_len =
                VR::RobustFpt::<F2>::new_1(TC::<I1, F1, I2, F2>::bi_to_f2(&segm_len)).sqrt();

            c_event.set_lower_x_raw(
                (sqrt_expr_.eval4(&ca, &cb) * half * inv_denom_sqr / segm_len).fpv(),
            );
        }
    }

    /// Recompute parameters of the circle event using high-precision library.
    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn pss(
        site1: &VSE::SiteEvent<I1, F1, I2, F2>,
        site2: &VSE::SiteEvent<I1, F1, I2, F2>,
        site3: &VSE::SiteEvent<I1, F1, I2, F2>,
        point_index: i32,
        c_event: &VC::CircleEventType<F2>,
        recompute_c_x: bool,
        recompute_c_y: bool,
        recompute_lower_x: bool,
    ) {
        let i1_to_i128 = TC::<I1, F1, I2, F2>::i1_to_i128;
        let f2_to_f1 = TC::<I1, F1, I2, F2>::f2_to_f1;
        let bi_to_f2 = TC::<I1, F1, I2, F2>::bi_to_f2;

        /*if site1.sorted_index() == 5 && site2.sorted_index() == 6 && site3.sorted_index() == 4 {
            println!("site1:{}", site1);
            println!("site2:{}", site2);
            println!("site3:{}", site3);
        }*/
        let mut sqrt_expr_ = VR::robust_sqrt_expr::<F2>::new();
        let quarter: F2 = num::cast::<f64, F2>(1f64 / 4.0f64).unwrap();
        let half: F2 = num::cast::<f64, F2>(1f64 / 2.0f64).unwrap();
        let one: BigInt = BigInt::from(1); //num::cast::<i8, I2>(1i8).unwrap();
        let two = 2; //: BigInt = BigInt::from(2); //num::cast::<i8, I2>(2i8).unwrap();
        let four = 4; //: BigInt = BigInt::from(4); // I2 = num::cast::<i8, I2>(4i8).unwrap();

        let mut a: [BigInt; 2] = [BigInt::zero(), BigInt::zero()];
        let mut b: [BigInt; 2] = [BigInt::zero(), BigInt::zero()];
        let mut c: [BigInt; 2] = [BigInt::zero(), BigInt::zero()];
        let mut cA: [BigInt; 4] = [
            BigInt::zero(),
            BigInt::zero(),
            BigInt::zero(),
            BigInt::zero(),
        ];
        let mut cB: [BigInt; 4] = [
            BigInt::zero(),
            BigInt::zero(),
            BigInt::zero(),
            BigInt::zero(),
        ];

        let segm_start1 = site2.point1();
        let segm_end1 = site2.point0();
        let segm_start2 = site3.point0();
        let segm_end2 = site3.point1();
        a[0] = TC::<I1, F1, I2, F2>::i1_to_bi(segm_end1.x())
            - TC::<I1, F1, I2, F2>::i1_to_bi(segm_start1.x());
        b[0] = TC::<I1, F1, I2, F2>::i1_to_bi(segm_end1.y())
            - TC::<I1, F1, I2, F2>::i1_to_bi(segm_start1.y());
        a[1] = TC::<I1, F1, I2, F2>::i1_to_bi(segm_end2.x())
            - TC::<I1, F1, I2, F2>::i1_to_bi(segm_start2.x());
        b[1] = TC::<I1, F1, I2, F2>::i1_to_bi(segm_end2.y())
            - TC::<I1, F1, I2, F2>::i1_to_bi(segm_start2.y());
        let orientation: BigInt = a[1].clone() * &b[0] - &a[0] * &b[1];
        if orientation.is_zero() {
            let denom = {
                let denomp1 = a[0].clone() * &a[0];
                let denomp2 = b[0].clone() * &b[0] * 2;
                let denom: BigInt = denomp1 + denomp2;
                bi_to_f2(&denom)
            };
            c[0] = b[0].clone()
                * (TC::<I1, F1, I2, F2>::i1_to_bi(segm_start2.x())
                    - TC::<I1, F1, I2, F2>::i1_to_bi(segm_start1.x()))
                - &a[0]
                    * (TC::<I1, F1, I2, F2>::i1_to_bi(segm_start2.y())
                        - TC::<I1, F1, I2, F2>::i1_to_bi(segm_start1.y()));
            let dx: BigInt = a[0].clone()
                * (TC::<I1, F1, I2, F2>::i1_to_bi(site1.y())
                    - TC::<I1, F1, I2, F2>::i1_to_bi(segm_start1.y()))
                - &b[0]
                    * (TC::<I1, F1, I2, F2>::i1_to_bi(site1.x())
                        - TC::<I1, F1, I2, F2>::i1_to_bi(segm_start1.x()));
            let dy: BigInt = b[0].clone()
                * (TC::<I1, F1, I2, F2>::i1_to_bi(site1.x())
                    - TC::<I1, F1, I2, F2>::i1_to_bi(segm_start2.x()))
                - &a[0]
                    * (TC::<I1, F1, I2, F2>::i1_to_bi(site1.y())
                        - TC::<I1, F1, I2, F2>::i1_to_bi(segm_start2.y()));
            cB[0] = dx.clone() * &dy;
            cB[1] = BigInt::from(1);

            if recompute_c_y {
                cA[0] = b[0].clone() * if point_index == 2i32 { 2i32 } else { -2i32 };
                cA[1] = a[0].clone()
                    * &a[0]
                    * (TC::<I1, F1, I2, F2>::i1_to_bi(segm_start1.y())
                        + TC::<I1, F1, I2, F2>::i1_to_bi(segm_start2.y()))
                    - &a[0]
                        * &b[0]
                        * (TC::<I1, F1, I2, F2>::i1_to_bi(segm_start1.x())
                            + TC::<I1, F1, I2, F2>::i1_to_bi(segm_start2.x())
                            - TC::<I1, F1, I2, F2>::i1_to_bi(site1.x()))
                        * 2
                    + &b[0] * &b[0] * (TC::<I1, F1, I2, F2>::i1_to_bi(site1.y())) * 2;
                let c_y = sqrt_expr_.eval2(&cA, &cB);
                c_event.set_y_raw((c_y / denom).fpv());
            }

            if recompute_c_x || recompute_lower_x {
                cA[0] = a[0].clone() * BigInt::from(if point_index == 2i32 { 2i32 } else { -2i32 });
                cA[1] = b[0].clone()
                    * &b[0]
                    * (i1_to_i128(segm_start1.x()) + i1_to_i128(segm_start2.x()))
                    - &a[0]
                        * &b[0]
                        * (i1_to_i128(segm_start1.y()) + i1_to_i128(segm_start2.y())
                            - i1_to_i128(site1.y()))
                        * 2
                    + &a[0] * &a[0] * (i1_to_i128(site1.x())) * 2;

                if recompute_c_x {
                    let c_x = sqrt_expr_.eval2(&cA, &cB);
                    c_event.set_x_raw((c_x / denom).fpv());
                }

                if recompute_lower_x {
                    cA[2] = if is_neg(&c[0]) {
                        c[0].clone() * -1
                    } else {
                        c[0].clone()
                    };
                    cB[2] = a[0].clone() * &a[0] + &b[0] * &b[0];
                    let lower_x = sqrt_expr_.eval3(&cA, &cB);
                    c_event.set_lower_x_raw((lower_x / denom).fpv());
                }
            }
            return;
        }
        c[0] = b[0].clone() * TC::<I1, F1, I2, F2>::i1_to_i128(segm_end1.x())
            - &a[0] * i1_to_i128(segm_end1.y());
        c[1] = a[1].clone() * TC::<I1, F1, I2, F2>::i1_to_i128(segm_end2.y())
            - &b[1] * i1_to_i128(segm_end2.x());
        let ix: BigInt = a[0].clone() * &c[1] + &a[1] * &c[0];
        let iy: BigInt = b[0].clone() * &c[1] + &b[1] * &c[0];
        let dx: BigInt = ix.clone() - &orientation * TC::<I1, F1, I2, F2>::i1_to_i128(site1.x());
        let dy: BigInt = iy.clone() - &orientation * TC::<I1, F1, I2, F2>::i1_to_i128(site1.y());
        if is_zero(&dx) && is_zero(&dy) {
            let denom: F2 = TC::<I1, F1, I2, F2>::bi_to_f2(&orientation);
            let c_x: F2 = TC::<I1, F1, I2, F2>::bi_to_f2(&ix) / denom;
            let c_y: F2 = TC::<I1, F1, I2, F2>::bi_to_f2(&iy) / denom;
            c_event.set_3_raw(c_x, c_y, c_x);
            return;
        }

        let sign: BigInt = BigInt::from(if point_index == 2i32 { 1i32 } else { -1i32 })
            * if is_neg(&orientation) { one } else { -one };
        // todo: remove -1*-1
        cA[0] = a[1].clone() * -1 * &dx + &b[1] * -1 * &dy;
        cA[1] = a[0].clone() * -1 * &dx + &b[0] * -1 * &dy;
        cA[2] = sign.clone();
        cA[3] = BigInt::zero();
        cB[0] = a[0].clone() * &a[0] + &b[0] * &b[0];
        cB[1] = a[1].clone() * &a[1] + &b[1] * &b[1];
        cB[2] = a[0].clone() * &a[1] + &b[0] * &b[1];
        cB[3] = (a[0].clone() * &dy - &b[0] * &dx) * (&a[1] * &dy - &b[1] * &dx) * -2;
        let temp = sqrt_expr_.sqrt_expr_evaluator_pss4(&cA[0..], &cB[0..]);
        let denom = temp * TC::<I1, F1, I2, F2>::bi_to_f2(&orientation);

        if recompute_c_y {
            cA[0] = b[1].clone() * (&dx * &dx + &dy * &dy) - &iy * (&dx * &a[1] + &dy * &b[1]);
            cA[1] = b[0].clone() * (&dx * &dx + &dy * &dy) - &iy * (&dx * &a[0] + &dy * &b[0]);
            cA[2] = iy.clone() * &sign;
            let cy = sqrt_expr_.sqrt_expr_evaluator_pss4(&cA[0..], &cB[0..]);
            c_event.set_y_raw((cy / denom).fpv());
        }

        if recompute_c_x || recompute_lower_x {
            cA[0] = a[1].clone() * (&dx * &dx + &dy * &dy) - &ix * (&dx * &a[1] + &dy * &b[1]);
            cA[1] = a[0].clone() * (&dx * &dx + &dy * &dy) - &ix * (&dx * &a[0] + &dy * &b[0]);
            cA[2] = ix.clone() * &sign;

            if recompute_c_x {
                let cx = sqrt_expr_.sqrt_expr_evaluator_pss4(&cA, &cB);
                c_event.set_x_raw((cx / denom).fpv());
            }

            if recompute_lower_x {
                cA[3] = orientation.clone()
                    * (&dx * &dx + &dy * &dy)
                    * (if temp.is_sign_negative() { -1 } else { 1 });
                let lower_x = sqrt_expr_.sqrt_expr_evaluator_pss4(&cA, &cB);
                c_event.set_lower_x_raw((lower_x / denom).fpv());
            }
        }
    }

    /// Recompute parameters of the circle event using high-precision library.
    #[allow(non_snake_case)]
    #[allow(clippy::many_single_char_names)]
    fn sss(
        site1: &VSE::SiteEvent<I1, F1, I2, F2>,
        site2: &VSE::SiteEvent<I1, F1, I2, F2>,
        site3: &VSE::SiteEvent<I1, F1, I2, F2>,
        c_event: &VC::CircleEventType<F2>,
        recompute_c_x: bool,
        recompute_c_y: bool,
        recompute_lower_x: bool,
    ) {
        let i1_to_i2 = TC::<I1, F1, I2, F2>::i1_to_i2;
        let i1_to_bi = TC::<I1, F1, I2, F2>::i1_to_bi;
        let i1_to_i128 = TC::<I1, F1, I2, F2>::i1_to_i128;
        let f2_to_f1 = TC::<I1, F1, I2, F2>::f2_to_f1;
        let sqrt_expr_ = VR::robust_sqrt_expr::<F2>::new();

        let mut a: [BigInt; 3] = [BigInt::zero(), BigInt::zero(), BigInt::zero()];
        let mut b: [BigInt; 3] = [BigInt::zero(), BigInt::zero(), BigInt::zero()];
        let mut c: [BigInt; 3] = [BigInt::zero(), BigInt::zero(), BigInt::zero()];
        let mut cA: [BigInt; 4] = [
            BigInt::zero(),
            BigInt::zero(),
            BigInt::zero(),
            BigInt::zero(),
        ];
        let mut cB: [BigInt; 4] = [
            BigInt::zero(),
            BigInt::zero(),
            BigInt::zero(),
            BigInt::zero(),
        ];

        // cA - corresponds to the cross product.
        // cB - corresponds to the squared length.
        a[0] = i1_to_bi(site1.x1()) - i1_to_i128(site1.x0());
        a[1] = i1_to_bi(site2.x1()) - i1_to_i128(site2.x0());
        a[2] = i1_to_bi(site3.x1()) - i1_to_i128(site3.x0());

        b[0] = i1_to_bi(site1.y1()) - i1_to_i128(site1.y0());
        b[1] = i1_to_bi(site2.y1()) - i1_to_i128(site2.y0());
        b[2] = i1_to_bi(site3.y1()) - i1_to_i128(site3.y0());

        c[0] = i1_to_bi(site1.x0()) * i1_to_i128(site1.y1())
            - i1_to_i128(site1.y0()) * i1_to_i128(site1.x1());
        c[1] = i1_to_bi(site2.x0()) * i1_to_i128(site2.y1())
            - i1_to_i128(site2.y0()) * i1_to_i128(site2.x1());
        c[2] = i1_to_bi(site3.x0()) * i1_to_i128(site3.y1())
            - i1_to_i128(site3.y0()) * i1_to_i128(site3.x1());

        for (i, aa) in a.iter().enumerate().take(3) {
            cB[i] = aa.clone() * aa + &b[i] * &b[i];
        }

        for i in 0..3 {
            let j = (i + 1) % 3;
            let k = (i + 2) % 3;
            cA[i] = a[j].clone() * &b[k] - &a[k] * &b[j];
        }
        let denom = sqrt_expr_.eval3(&cA, &cB);

        if recompute_c_y {
            for i in 0..3 {
                let j = (i + 1) % 3;
                let k = (i + 2) % 3;
                cA[i] = b[j].clone() * &c[k] - &b[k] * &c[j];
            }
            let c_y = sqrt_expr_.eval3(&cA, &cB);
            c_event.set_y_raw((c_y / denom).fpv());
        }

        if recompute_c_x || recompute_lower_x {
            cA[3] = BigInt::zero();
            for i in 0..3 {
                let j = (i + 1) % 3;
                let k = (i + 2) % 3;
                cA[i] = a[j].clone() * &c[k] - &a[k] * &c[j];
                if recompute_lower_x {
                    cA[3] = cA[3].clone() + &cA[i] * &b[i];
                }
            }

            if recompute_c_x {
                let c_x = sqrt_expr_.eval3(&cA, &cB);
                c_event.set_x_raw((c_x / denom).fpv());
            }

            if recompute_lower_x {
                cB[3] = BigInt::from(1);
                let lower_x = sqrt_expr_.eval4(&cA, &cB);
                c_event.set_lower_x_raw((lower_x / denom).fpv());
            }
        }
    }
}
