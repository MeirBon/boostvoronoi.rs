// Boost.Polygon library detail/robust_fpt.hpp header file

//          Copyright Andrii Sydorchuk 2010-2012.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE_1_0.txt or copy at
//          http://www.boost.org/LICENSE_1_0.txt)

// See http://www.boost.org for updates, documentation, and revision history.

// Ported from C++ boost 1.75.0 to Rust in 2020/2021 by Eadf (github.com/eadf)

use super::beach_line as VB;
use super::predicate as VP;
use super::InputType;
use super::Point;
use std::cmp::Ordering;
use std::ops::Neg;

///
/// This was declared as "typedef std::pair<point_type, beach_line_iterator> end_point_type" in C++
///
#[derive(Debug)]
pub(crate) struct EndPointPair<I>
where
    I: InputType + Neg<Output = I>,
{
    pub(crate) site: Point<I>,
    pub(crate) beachline_index: VB::BeachLineIndex,
}

impl<I> EndPointPair<I>
where
    I: InputType + Neg<Output = I>,
{
    pub(crate) fn new_2(first: Point<I>, second: VB::BeachLineIndex) -> Self {
        Self {
            site: first,
            beachline_index: second,
        }
    }
}

impl<I> PartialOrd for EndPointPair<I>
where
    I: InputType + Neg<Output = I>,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<I> Ord for EndPointPair<I>
where
    I: InputType + Neg<Output = I>,
{
    fn cmp(&self, other: &Self) -> Ordering {
        if VP::PointComparisonPredicate::<I>::point_comparison_predicate(&self.site, &other.site) {
            Ordering::Greater
        } else if self.site == other.site {
            Ordering::Equal
        } else {
            Ordering::Less
        }
    }
}

impl<I> PartialEq for EndPointPair<I>
where
    I: InputType + Neg<Output = I>,
{
    fn eq(&self, other: &Self) -> bool {
        self.site.eq(&other.site)
    }
}

impl<I> Eq for EndPointPair<I> where I: InputType + Neg<Output = I> {}
