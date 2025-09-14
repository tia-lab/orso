// Pagination support
use serde::{Deserialize, Serialize};

// Pagination parameters for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    /// Page number (1-based)
    pub page: u32,
    /// Number of items per page
    pub per_page: u32,
    /// Total number of items (set after query execution)
    pub total: Option<u64>,
    /// Total number of pages (calculated)
    pub total_pages: Option<u32>,
}

impl Pagination {
    /// Create a new pagination instance
    pub fn new(page: u32, per_page: u32) -> Self {
        Self {
            page,
            per_page,
            total: None,
            total_pages: None,
        }
    }

    /// Get the offset for SQL LIMIT/OFFSET
    pub fn offset(&self) -> u32 {
        (self.page - 1) * self.per_page
    }

    /// Get the limit for SQL LIMIT/OFFSET
    pub fn limit(&self) -> u32 {
        self.per_page
    }

    /// Set the total count and calculate total pages
    pub fn set_total(&mut self, total: u64) {
        self.total = Some(total);
        self.total_pages = Some(((total as f64) / (self.per_page as f64)).ceil() as u32);
    }

    /// Check if there's a next page
    pub fn has_next(&self) -> bool {
        if let (Some(total_pages), Some(current_page)) = (self.total_pages, Some(self.page)) {
            current_page < total_pages
        } else {
            false
        }
    }

    /// Check if there's a previous page
    pub fn has_prev(&self) -> bool {
        self.page > 1
    }

    /// Get the start item number for the current page
    pub fn start_item(&self) -> u32 {
        (self.page - 1) * self.per_page + 1
    }

    /// Get the end item number for the current page
    pub fn end_item(&self) -> u32 {
        self.page * self.per_page
    }

    /// Get the next page number
    pub fn next_page(&self) -> Option<u32> {
        if self.has_next() {
            Some(self.page + 1)
        } else {
            None
        }
    }

    /// Get the previous page number
    pub fn prev_page(&self) -> Option<u32> {
        if self.has_prev() {
            Some(self.page - 1)
        } else {
            None
        }
    }
}

impl Default for Pagination {
    fn default() -> Self {
        Self::new(1, 20)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResult<T> {
    /// The data items for the current page
    pub data: Vec<T>,
    /// Pagination metadata
    pub pagination: Pagination,
}

impl<T> PaginatedResult<T> {
    /// Create a new paginated result
    pub fn new(data: Vec<T>, pagination: Pagination) -> Self {
        Self { data, pagination }
    }

    /// Create a paginated result with total count
    pub fn with_total(data: Vec<T>, mut pagination: Pagination, total: u64) -> Self {
        pagination.set_total(total);
        Self { data, pagination }
    }

    /// Get the data items
    pub fn data(&self) -> &[T] {
        &self.data
    }

    /// Get the pagination metadata
    pub fn pagination(&self) -> &Pagination {
        &self.pagination
    }

    /// Get the number of items in the current page
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the current page is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Map the data items to a new type
    pub fn map<U, F>(self, f: F) -> PaginatedResult<U>
    where
        F: FnMut(T) -> U,
    {
        PaginatedResult {
            data: self.data.into_iter().map(f).collect(),
            pagination: self.pagination,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPagination {
    /// Cursor for the next page
    pub cursor: Option<String>,
    /// Number of items per page
    pub limit: u32,
    /// Whether to include the cursor item in results
    pub include_cursor: bool,
    /// Whether there are more items
    pub has_next: bool,
    /// Whether there are previous items
    pub has_prev: bool,
    /// Cursor for the next page
    pub next_cursor: Option<String>,
    /// Cursor for the previous page
    pub prev_cursor: Option<String>,
    /// Total number of items
    pub total: Option<u64>,
}

impl CursorPagination {
    /// Create a new cursor pagination instance
    pub fn new(limit: u32) -> Self {
        Self {
            cursor: None,
            limit,
            include_cursor: false,
            has_next: false,
            has_prev: false,
            next_cursor: None,
            prev_cursor: None,
            total: None,
        }
    }

    /// Create with a specific cursor
    pub fn with_cursor(limit: u32, cursor: Option<String>) -> Self {
        let has_prev = cursor.is_some();
        Self {
            cursor,
            limit,
            include_cursor: false,
            has_next: false,
            has_prev,
            next_cursor: None,
            prev_cursor: None,
            total: None,
        }
    }

    /// Create with a specific cursor (deprecated, use with_cursor(limit, cursor) instead)
    pub fn with_cursor_old(cursor: String, limit: u32) -> Self {
        Self {
            cursor: Some(cursor),
            limit,
            include_cursor: false,
            has_next: false,
            has_prev: true,
            next_cursor: None,
            prev_cursor: None,
            total: None,
        }
    }

    /// Set the cursor
    pub fn set_cursor(&mut self, cursor: Option<String>) {
        self.cursor = cursor;
    }

    /// Get the limit for SQL queries
    pub fn limit(&self) -> u32 {
        self.limit
    }
}

impl Default for CursorPagination {
    fn default() -> Self {
        Self::new(20)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPaginatedResult<T> {
    /// The data items
    pub data: Vec<T>,
    /// Pagination metadata
    pub pagination: CursorPagination,
}

impl<T> CursorPaginatedResult<T> {
    /// Create a new cursor paginated result
    pub fn new(data: Vec<T>, pagination: CursorPagination) -> Self {
        Self { data, pagination }
    }

    /// Get the data items
    pub fn data(&self) -> &[T] {
        &self.data
    }

    /// Get the pagination metadata
    pub fn pagination(&self) -> &CursorPagination {
        &self.pagination
    }
}
