// This module contains the macro implementations
// The actual macro is implemented in a separate crate (libsql-orm-macros)

pub use crate::traits::Orso;

#[macro_export]
macro_rules! filter {
    ($column:expr, $op:expr, $value:expr) => {
        $crate::Filter::new($column, $op, $crate::FilterValue::Single($value.into()))
    };

    ($column:expr, in, $values:expr) => {
        $crate::Filter::in_values($column, $values)
    };

    ($column:expr, not_in, $values:expr) => {
        $crate::Filter::not_in_values($column, $values)
    };

    ($column:expr, between, $min:expr, $max:expr) => {
        $crate::Filter::between($column, $min, $max)
    };

    ($column:expr, not_between, $min:expr, $max:expr) => {
        $crate::Filter::not_between($column, $min, $max)
    };

    ($column:expr, is_null) => {
        $crate::Filter::is_null($column)
    };

    ($column:expr, is_not_null) => {
        $crate::Filter::is_not_null($column)
    };
}

#[macro_export]
macro_rules! sort {
    ($column:expr, asc) => {
        $crate::Sort::asc($column)
    };

    ($column:expr, desc) => {
        $crate::Sort::desc($column)
    };

    ($column:expr) => {
        $crate::Sort::asc($column)
    };
}

#[macro_export]
macro_rules! search {
    ($query:expr, $($columns:expr),*) => {
        $crate::SearchFilter::new($query, vec![$($columns),*])
    };
}

#[macro_export]
macro_rules! pagination {
    ($page:expr, $per_page:expr) => {
        $crate::Pagination::new($page, $per_page)
    };

    ($page:expr) => {
        $crate::Pagination::new($page, 20)
    };
}

#[macro_export]
macro_rules! query {
    ($table:expr) => {
        $crate::QueryBuilder::new($table)
    };
}

#[macro_export]
macro_rules! filter_op {
    (and, $($filters:expr),*) => {
        $crate::FilterOperator::and(vec![$($filters),*])
    };

    (or, $($filters:expr),*) => {
        $crate::FilterOperator::or(vec![$($filters),*])
    };

    (not, $filter:expr) => {
        $crate::FilterOperator::not($filter)
    };

    ($filter:expr) => {
        $crate::FilterOperator::Single($filter)
    };
}
